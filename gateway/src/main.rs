mod alert_group_routes;
mod aiops_routes;
mod alert_routes;
mod audit;
mod audit_routes;
mod auth;
mod auth_routes;
mod component_routes;
mod cmdb_routes;
mod config;
mod credential_routes;
mod crypto;
mod dashboard_routes;
mod db;
mod dept_routes;
mod dict_routes;
mod error;
mod knowledge_routes;
mod license_crypto;
mod license_routes;
mod job_routes;
mod log_alert_scheduler;
mod log_routes;
mod report_routes;
mod role_routes;
mod routes;
mod ssh_executor;
mod system_routes;
mod template_routes;
mod notification_cleaner;
mod notification_engine;
mod notification_routes;
mod ticket_routes;
mod ticket_scheduler;
mod token_routes;
mod sms_strategy_routes;
mod eventide_lookup_sync;
mod http_outbound;
mod workflow_engine;

use clap::Parser;
use config::GatewayConfig;
use routes::{create_router, AppState};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "meridianops-gateway", about = "MeridianOps API 聚合网关")]
struct Cli {
    #[arg(short, long, default_value = "gateway-config.toml")]
    config: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let cli = Cli::parse();

    let config = if std::path::Path::new(&cli.config).exists() {
        GatewayConfig::load(&cli.config)?
    } else {
        tracing::warn!("Config file not found: {}, using defaults", cli.config);
        let mut c = GatewayConfig::default();
        c.apply_env_overrides();
        c
    };

    config.assert_secure_defaults()?;

    // 1. 数据库连接 + 迁移 + 种子
    let db_pool = db::connect(&config.database).await?;
    run_migrations_ignore_checksum(&db_pool).await?;
    db::seed_admin_if_empty(&db_pool, &config.auth).await?;

    let bind = config.server.bind.clone();
    tracing::info!("Starting MeridianOps Gateway on {}", bind);

    // 1.5 加载告警接入配置：优先从 system_settings 表读取，覆盖 toml 中的值。
    // 这样前端在「告警接入」面板修改的密钥/启用状态重启后仍然生效。
    let mut alerts_runtime = config.alerts.clone();
    if let Ok(Some(token)) = db::get_setting(&db_pool, "alert_ingress_token").await {
        if !token.is_empty() {
            alerts_runtime.ingress_token = token;
        }
    }
    if let Ok(Some(en)) = db::get_setting(&db_pool, "alert_ingress_enabled").await {
        if let Ok(b) = en.parse::<bool>() {
            alerts_runtime.ingress_enabled = b;
        }
    }
    tracing::info!(
        "alert ingress loaded from db: enabled={}, token_len={}",
        alerts_runtime.ingress_enabled,
        alerts_runtime.ingress_token.len()
    );

    let lookup_runtime =
        eventide_lookup_sync::load_runtime(&db_pool, &config.eventide_lookup).await;
    tracing::info!(
        "eventide lookup loaded: enabled={}, token_set={}, targets={}",
        lookup_runtime.enabled,
        !lookup_runtime.lookup_sync_token.trim().is_empty(),
        lookup_runtime
            .targets
            .iter()
            .filter(|t| !t.lookup_id.trim().is_empty())
            .count()
    );

    // 2. AppState
    let state = Arc::new(AppState {
        config: Arc::new(config.clone()),
        alerts_runtime: Arc::new(std::sync::RwLock::new(alerts_runtime)),
        lookup_runtime: Arc::new(std::sync::RwLock::new(lookup_runtime)),
        client: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?,
        db: db_pool,
        jwt_secret: config.auth.jwt_secret.clone(),
        jwt_ttl_hours: config.auth.token_ttl_hours,
        lookup_sync: Arc::new(eventide_lookup_sync::LookupSyncRuntime::new()),
    });

    let app = create_router(state.clone());

    // 3. 启动定时拉取后台任务
    tokio::spawn(cmdb_routes::pull_scheduler_loop(state.clone()));
    tokio::spawn(eventide_lookup_sync::start_scheduler(state.clone()));
    tokio::spawn(http_outbound::start_scheduler(state.clone()));

    // 3.1 一次性任务：用 jieba 重新分词知识库 content_text（幂等，已执行则跳过）
    tokio::spawn(knowledge_routes::resegment_knowledge_content(state.db.clone()));

    // 3.2 启动 SLA 超时自动升级后台任务
    tokio::spawn(ticket_scheduler::start_scheduler(state.db.clone()));

    // 3.3 启动通知发送日志自动清理后台任务
    tokio::spawn(notification_cleaner::start_scheduler(
        state.db.clone(),
        state.config.notification_cleaner.clone(),
    ));

    // 3.4 启动日志告警联动后台任务（Phase 5，默认关闭，需在 [logs.alerting] 显式开启）
    tokio::spawn(log_alert_scheduler::start_scheduler(state.clone()));

    // 4. 启动 + graceful shutdown
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    Ok(())
}

/// 运行 sqlx 迁移，但跳过已 applied 记录的 checksum 校验。
/// 由于迁移文件在历史上多次被调整注释/内容，若严格按 sqlx 默认的 checksum 对比会在
/// 启动时 panic。这里按 version 做幂等：已 applied（success=1）的跳过，未 applied 的按 sqlx 顺序 apply。
async fn run_migrations_ignore_checksum(pool: &sqlx::MySqlPool) -> anyhow::Result<()> {
    use sqlx::migrate::Migrate;
    use std::collections::BTreeSet;

    // ============================================================
    // 预执行：确保 alert_events 的接入渠道/接入者列存在（兼容历史上失败过的迁移 31）
    // MySQL 不支持 ADD COLUMN IF NOT EXISTS，手动检查。
    // ============================================================
    let has_ing_channel: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM INFORMATION_SCHEMA.COLUMNS \
         WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'alert_events' AND COLUMN_NAME = 'ingress_channel'",
    )
    .fetch_optional(pool)
    .await
    .unwrap_or(None);
    if has_ing_channel.is_none() {
        tracing::info!("adding alert_events.ingress_channel column");
        sqlx::query(
            "ALTER TABLE alert_events \
             ADD COLUMN ingress_channel VARCHAR(32) NOT NULL DEFAULT 'manual' \
             COMMENT '接入渠道:webhook/manual/job/api_token/system' AFTER source",
        )
        .execute(pool)
        .await
        .ok();
    }
    let has_ing_actor: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM INFORMATION_SCHEMA.COLUMNS \
         WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'alert_events' AND COLUMN_NAME = 'ingress_actor'",
    )
    .fetch_optional(pool)
    .await
    .unwrap_or(None);
    if has_ing_actor.is_none() {
        tracing::info!("adding alert_events.ingress_actor column");
        sqlx::query(
            "ALTER TABLE alert_events \
             ADD COLUMN ingress_actor VARCHAR(128) NULL \
             COMMENT '接入者身份（通道名/用户名/token 名）' AFTER ingress_channel",
        )
        .execute(pool)
        .await
        .ok();
    }

    // 预执行：确保 alert_events 的 external_id 列存在（Eventide 双向回写所需）
    let has_external_id: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM INFORMATION_SCHEMA.COLUMNS \
         WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'alert_events' AND COLUMN_NAME = 'external_id'",
    )
    .fetch_optional(pool)
    .await
    .unwrap_or(None);
    if has_external_id.is_none() {
        tracing::info!("adding alert_events.external_id column");
        sqlx::query(
            "ALTER TABLE alert_events \
             ADD COLUMN external_id VARCHAR(128) NULL \
             COMMENT '外部系统告警 ID（Eventide alertId）用于双向回写' AFTER fingerprint",
        )
        .execute(pool)
        .await
        .ok();
    }

    // 清理 _sqlx_migrations 中历史遗留的失败记录（success=0），避免重复 apply 时主键冲突
    sqlx::query("DELETE FROM _sqlx_migrations WHERE success = 0")
        .execute(pool)
        .await
        .ok();

    // 获取所有已成功 applied 的 version
    let applied: BTreeSet<i64> = sqlx::query_scalar::<_, i64>(
        "SELECT version FROM _sqlx_migrations WHERE success = 1",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .collect();

    let migrator = sqlx::migrate!("./migrations");
    let mut applied_new = 0usize;
    for m in migrator.iter() {
        let ver = m.version as i64;
        if applied.contains(&ver) {
            continue; // 已 applied 则跳过（不再核对 checksum，避免历史修改后报错）
        }
        tracing::info!("applying migration {}: {}", m.version, m.description);
        let mut conn = pool.acquire().await?;
        conn.apply(m).await.map_err(|e| {
            anyhow::anyhow!(
                "failed to apply migration {} ({}): {}",
                m.version,
                m.description,
                e
            )
        })?;
        applied_new += 1;
    }
    if applied_new > 0 {
        tracing::info!("applied {} new migrations", applied_new);
    }
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutdown signal received, stopping");
}
