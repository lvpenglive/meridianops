//! 日志告警联动调度器（Phase 5）
//!
//! 功能：
//! - 每 check_interval_secs 秒查询 ClickHouse：最近 window_minutes 分钟内
//!   ERROR+ 级别日志按 hostname 分组计数，超过 error_threshold 的主机触发告警
//! - 告警写入 alert_events 表，fingerprint = `log_surge:{hostname}:{bucket}` 保证
//!   同一窗口幂等不重复触发
//! - 同主机触发后 silence_minutes 分钟内静默，避免重复告警
//! - 触发后调用 notification_engine::dispatch_event 异步分发通知
//! - 同时调用 spawn_clue_logs_fetch 异步填充关联日志
//!
//! 调度器在 ClickHouse 不可达时静默忽略，不影响主服务。

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use sqlx::Row;
use uuid::Uuid;

use crate::config::LogsAlertingConfig;
use crate::routes::AppState;

/// 调度器入口：tokio::spawn 启动后常驻运行
pub async fn start_scheduler(state: Arc<AppState>) {
    let cfg = state.config.logs.alerting.clone();
    if !cfg.enabled {
        tracing::info!(
            target: "log_alert",
            "scheduler disabled (logs.alerting.enabled = false)"
        );
        return;
    }

    let interval_secs = cfg.check_interval_secs.max(10);
    // 启动时先延迟 30s，避免跟 CMDB 同步/通知清理等启动任务抢 DB 连接
    tokio::time::sleep(Duration::from_secs(30)).await;

    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
    interval.tick().await; // 立即跑第一轮（已 sleep 过 30s）

    tracing::info!(
        target: "log_alert",
        "scheduler started (interval={}s, window={}min, threshold={}, silence={}min, levels={})",
        interval_secs, cfg.window_minutes, cfg.error_threshold, cfg.silence_minutes, cfg.levels
    );

    loop {
        interval.tick().await;
        if let Err(e) = run_round(&state, &cfg).await {
            tracing::error!(target: "log_alert", "round failed: {}", e);
        }
    }
}

/// 单轮检测：查突增主机 → 静默过滤 → 创建告警 → 异步分发通知 + 关联日志
async fn run_round(state: &Arc<AppState>, cfg: &LogsAlertingConfig) -> anyhow::Result<()> {
    let surges = crate::log_routes::fetch_log_surge(
        state,
        cfg.window_minutes,
        &cfg.levels,
        cfg.error_threshold,
    )
    .await;

    if surges.is_empty() {
        return Ok(());
    }

    tracing::info!(
        target: "log_alert",
        "detected {} surge host(s) above threshold {}",
        surges.len(),
        cfg.error_threshold
    );

    for surge in surges {
        // 静默检查：近 silence_minutes 分钟内同主机已触发过则跳过
        if is_silenced(state, &surge.hostname, cfg.silence_minutes).await? {
            tracing::debug!(
                target: "log_alert",
                "host {} silenced within {}min, skip",
                surge.hostname,
                cfg.silence_minutes
            );
            continue;
        }

        // 创建告警
        match create_surge_alert(state, &surge, cfg).await {
            Ok(alert_id) => {
                tracing::info!(
                    target: "log_alert",
                    "created alert {} for host {} ({} errors, last_ts={})",
                    alert_id, surge.hostname, surge.error_count, surge.last_ts
                );

                // 异步获取关联日志（Phase 3 复用）
                let fired_at_str: String = surge.last_ts.chars().take(19).collect();
                crate::alert_routes::spawn_clue_logs_fetch(
                    state,
                    &alert_id,
                    Some(surge.hostname.clone()),
                    fired_at_str,
                );

                // 异步分发通知
                let state_clone = state.clone();
                let host = surge.hostname.clone();
                let count = surge.error_count;
                let window = cfg.window_minutes;
                let samples = surge.samples.clone();
                let alert_id_for_link = alert_id.clone();
                tokio::spawn(async move {
                    let sample_block = if samples.is_empty() {
                        String::new()
                    } else {
                        let bullets: Vec<String> = samples
                            .iter()
                            .map(|s| format!("  • {}", truncate(s, 200)))
                            .collect();
                        format!("\n样本日志：\n{}", bullets.join("\n"))
                    };
                    let body = format!(
                        "主机 {} 近 {} 分钟产生 {} 条 ERROR+ 级别日志，已超过阈值。{}",
                        host, window, count, sample_block
                    );
                    crate::notification_engine::dispatch_event(
                        &state_clone,
                        "log",                   // event_type
                        "log_surge",             // trigger_scene
                        Some("4"),               // severity: 4 = 重要
                        &format!("日志突增告警: {}", host),
                        &body,
                        &format!("/alerts?id={}", alert_id_for_link),
                        &Vec::new(),             // 无指定接收人，由规则匹配
                        Some(&host),
                    )
                    .await;
                });
            }
            Err(e) => {
                tracing::warn!(
                    target: "log_alert",
                    "create surge alert failed for host {}: {}",
                    surge.hostname,
                    e
                );
            }
        }
    }

    Ok(())
}

/// 检查近 silence_minutes 分钟内是否已为该主机触发过日志突增告警
async fn is_silenced(
    state: &AppState,
    hostname: &str,
    silence_minutes: u32,
) -> anyhow::Result<bool> {
    let cutoff = Utc::now() - chrono::Duration::minutes(silence_minutes.max(1) as i64);
    let cutoff_str = cutoff.to_rfc3339();
    let fp_prefix = format!("log_surge:{}:", hostname);
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM alert_events \
         WHERE fingerprint LIKE ? AND created_at >= ?",
    )
    .bind(format!("{}%", fp_prefix))
    .bind(&cutoff_str)
    .fetch_one(&state.db)
    .await?;
    Ok(count > 0)
}

/// 创建一条日志突增告警，返回告警 ID
async fn create_surge_alert(
    state: &AppState,
    surge: &crate::log_routes::LogSurgeRow,
    cfg: &LogsAlertingConfig,
) -> anyhow::Result<String> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    // fingerprint 用 hostname + 5 分钟时间桶，保证同一窗口幂等
    // 时间桶：对齐到 window_minutes 边界（取当前分钟减去 minute % window）
    let bucket = format_bucket_key(cfg.window_minutes);
    let fingerprint = format!("log_surge:{}:{}", surge.hostname, bucket);

    let title = format!(
        "日志突增告警：{} 近{}分钟 ERROR+ 日志 {} 条",
        surge.hostname, cfg.window_minutes, surge.error_count
    );

    let sample_block = if surge.samples.is_empty() {
        String::from("无样本日志")
    } else {
        let bullets: Vec<String> = surge
            .samples
            .iter()
            .map(|s| format!("  • {}", truncate(s, 300)))
            .collect();
        format!("样本日志：\n{}", bullets.join("\n"))
    };
    let message = format!(
        "主机 {} 近 {} 分钟内产生 {} 条 {} 级别日志，已超过阈值 {} 条/{}分钟。\n\n{}",
        surge.hostname,
        cfg.window_minutes,
        surge.error_count,
        cfg.levels,
        cfg.error_threshold,
        cfg.window_minutes,
        sample_block
    );

    let labels = serde_json::json!({
        "alertname": "LogSurgeDetected",
        "hostname": surge.hostname,
        "category": "log",
        "eventType": "log",
        "source": "log_alert_scheduler",
        "threshold": cfg.error_threshold,
        "window_minutes": cfg.window_minutes,
        "levels": cfg.levels,
        "error_count": surge.error_count
    });
    let labels_str = serde_json::to_string(&labels).unwrap_or_else(|_| "{}".to_string());

    // fired_at 用 surge.last_ts 截断到秒；fallback 用 now
    let fired_at: String = {
        let truncated: String = surge.last_ts.chars().take(19).collect();
        if truncated.len() == 19 {
            // 把 "YYYY-MM-DD HH:MM:SS" 转成 RFC3339 UTC
            match chrono::NaiveDateTime::parse_from_str(&truncated, "%Y-%m-%d %H:%M:%S") {
                Ok(naive) => chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(naive, chrono::Utc).to_rfc3339(),
                Err(_) => now.clone(),
            }
        } else {
            now.clone()
        }
    };

    sqlx::query(
        "INSERT INTO alert_events \
         (id, fingerprint, external_id, source, ingress_channel, ingress_actor, \
          severity, status, title, message, labels, ci_id, ci_name_snapshot, \
          fire_count, first_fired_at, fired_at, ends_at, \
          acknowledged_by, acknowledged_at, resolved_by, resolved_at, \
          resolution_note, created_at, updated_at) \
         VALUES (?, ?, NULL, ?, ?, ?, ?, 'firing', ?, ?, ?, NULL, NULL, 1, ?, ?, NULL, NULL, NULL, NULL, NULL, ?, ?)",
    )
    .bind(&id)
    .bind(&fingerprint)
    .bind("system")
    .bind("system")
    .bind("log_alert_scheduler")
    .bind("4") // severity: 4 = 重要
    .bind(&title)
    .bind(&message)
    .bind(&labels_str)
    .bind(&fired_at)
    .bind(&fired_at)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await?;

    // 审计
    let _ = crate::db::insert_audit_log(
        &state.db,
        "log_alert_scheduler",
        "create_log_surge_alert",
        "alert_events",
        &id,
        Some(&serde_json::json!({
            "hostname": surge.hostname,
            "error_count": surge.error_count,
            "threshold": cfg.error_threshold,
            "window_minutes": cfg.window_minutes,
            "fingerprint": fingerprint,
        })),
        "scheduler",
        "success",
    )
    .await;

    Ok(id)
}

/// 生成时间桶 key：当前时间对齐到 window_minutes 边界
/// 如 window=5, now=08:37 → bucket=08:35
fn format_bucket_key(window_minutes: u32) -> String {
    let now = Utc::now();
    let win = window_minutes.max(1) as i64;
    let minute = now.format("%M").to_string().parse::<i64>().unwrap_or(0);
    let aligned_minute = minute - (minute % win);
    // 用 format 手动覆盖分钟部分
    format!(
        "{}-{:02}-{:02} {:02}:{:02}",
        now.format("%Y"),
        now.format("%m"),
        now.format("%d"),
        now.format("%H"),
        aligned_minute
    )
}

/// 截断字符串到 max_chars，超过加省略号
fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}...", truncated)
    }
}
