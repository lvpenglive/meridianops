//! 通知发送日志清理任务
//!
//! 功能：
//! - 按天滚动删除 `notification_logs` 表中超龄的数据（默认保留 30 天）
//! - 采用 `LIMIT ?` 分批删除，避免 DELETE 长事务锁表；每批之间 sleep 50ms 让让其他事务切入
//! - 后台 interval 循环（默认 24h 一次），通过 tokio::spawn 启动
//! - 提供 pub `run_cleanup_once` 给手动 API 调用
//!
//! sent_at 类型是 VARCHAR(RFC3339)，在 MySQL 中既可按字符串字典序比较（因为 RFC3339 UTC
//! 以 `YYYY-MM-DDThh:mm:ss` 开头，时间先后等价于字符串排序），也可用 `STR_TO_DATE` 强转。
//! 这里使用参数化的 RFC3339 边界字符串直接比较，性能与可读性都较好。

use std::time::Duration;

use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;

use crate::config::NotificationCleanerConfig;

/// 单次清理的统计结果
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CleanupResult {
    /// 生效的保留天数（保底后）
    pub retention_days: u32,
    /// sent_at 截断点（RFC3339 UTC）
    pub cutoff: String,
    /// 删除的总行数
    pub deleted: u64,
    /// 分了多少批
    pub batches: u32,
    /// 运行耗时（ms）
    pub duration_ms: u64,
}

/// 按配置跑一次清理（pub 供调度循环 + 手动 API 复用）
pub async fn run_cleanup_once(
    pool: &MySqlPool,
    cfg: &NotificationCleanerConfig,
) -> anyhow::Result<CleanupResult> {
    let retention = if cfg.retention_days == 0 {
        30
    } else {
        cfg.retention_days
    };
    let batch = if cfg.batch_size == 0 { 1000 } else { cfg.batch_size };

    // 计算截止时间：NOW - retention_days，得到 RFC3339 UTC 字符串
    let cutoff_dt = chrono::Utc::now() - chrono::Duration::days(retention as i64);
    let cutoff = cutoff_dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    tracing::info!(
        target: "notification_cleaner",
        retention_days = retention,
        cutoff = %cutoff,
        batch_size = batch,
        "notification_logs cleanup starting..."
    );

    let started = std::time::Instant::now();
    let mut deleted: u64 = 0;
    let mut batches: u32 = 0;
    // 避免死循环：最多跑 10_000 批（约 1000 万行）足够覆盖一般场景
    let max_batches: u32 = 10_000;

    loop {
        if batches >= max_batches {
            tracing::warn!(
                target: "notification_cleaner",
                "reach max_batches={}, stop this round to avoid runaway loop", max_batches
            );
            break;
        }
        let affected = sqlx::query(
            "DELETE FROM notification_logs WHERE sent_at < ? ORDER BY sent_at ASC LIMIT ?"
        )
        .bind(&cutoff)
        .bind(batch)
        .execute(pool)
        .await
        .map_err(|e| anyhow::anyhow!("删除通知日志失败: {}", e))?
        .rows_affected();

        batches += 1;
        deleted += affected;

        if affected < batch as u64 {
            // 最后一批不足 batch_size → 本轮全部超龄数据已删完
            break;
        }

        // 批间短暂让步，避免持续持有 gap lock 影响告警写日志
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let duration_ms = started.elapsed().as_millis() as u64;
    let result = CleanupResult {
        retention_days: retention,
        cutoff,
        deleted,
        batches,
        duration_ms,
    };
    tracing::info!(
        target: "notification_cleaner",
        retention_days = result.retention_days,
        cutoff = %result.cutoff,
        deleted = result.deleted,
        batches = result.batches,
        duration_ms = result.duration_ms,
        "notification_logs cleanup finished"
    );
    Ok(result)
}

/// 后台调度循环，应通过 tokio::spawn 调用。
/// 配置 `enabled=false` 时直接返回，不占用资源。
pub async fn start_scheduler(pool: MySqlPool, cfg: NotificationCleanerConfig) {
    if !cfg.enabled {
        tracing::info!(target: "notification_cleaner", "cleanup disabled via config, scheduler skipped");
        return;
    }

    let interval_secs = if cfg.interval_secs == 0 {
        86400
    } else {
        cfg.interval_secs
    };

    // 启动时先延迟 30s 再跑第一轮，避免跟 CMDB 同步/知识库分词等启动任务抢 DB 连接
    tokio::time::sleep(Duration::from_secs(30)).await;

    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
    // interval 的首次 tick 是"立即"，我们已经 sleep 过 30s，按默认行为直接跑第一轮即可
    interval.tick().await;
    tracing::info!(
        target: "notification_cleaner",
        "scheduler started (interval={}s, retention_days={}, batch_size={})",
        interval_secs, cfg.retention_days, cfg.batch_size
    );

    loop {
        interval.tick().await;
        if let Err(e) = run_cleanup_once(&pool, &cfg).await {
            tracing::error!(target: "notification_cleaner", "cleanup round failed: {}", e);
        }
    }
}
