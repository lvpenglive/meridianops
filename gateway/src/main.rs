mod audit;
mod audit_routes;
mod auth;
mod auth_routes;
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
mod report_routes;
mod role_routes;
mod routes;
mod ssh_executor;
mod system_routes;
mod token_routes;

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

    // JWT 默认密钥检测：非 loopback 部署仍用默认密钥时告警
    if config.auth.jwt_secret == "meridianops-dev-secret-change-me"
        && !config.server.bind.starts_with("127.0.0.1")
    {
        tracing::warn!(
            "JWT secret 仍是默认值且 bind 非 loopback，生产环境必须通过 MERIDIANOPS_JWT_SECRET 覆盖"
        );
    }

    // 1. 数据库连接 + 迁移 + 种子
    let db_pool = db::connect(&config.database).await?;
    run_migrations_ignore_checksum(&db_pool).await?;
    db::seed_admin_if_empty(&db_pool, &config.auth).await?;

    let bind = config.server.bind.clone();
    tracing::info!("Starting MeridianOps Gateway on {}", bind);

    // 2. AppState
    let state = Arc::new(AppState {
        config: Arc::new(config.clone()),
        client: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?,
        db: db_pool,
        jwt_secret: config.auth.jwt_secret.clone(),
        jwt_ttl_hours: config.auth.token_ttl_hours,
    });

    let app = create_router(state.clone());

    // 3. 启动定时拉取后台任务
    tokio::spawn(cmdb_routes::pull_scheduler_loop(state.db.clone()));

    // 3.1 一次性任务：用 jieba 重新分词知识库 content_text（幂等，已执行则跳过）
    tokio::spawn(knowledge_routes::resegment_knowledge_content(state.db.clone()));

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
