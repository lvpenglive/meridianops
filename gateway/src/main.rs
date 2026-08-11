mod audit;
mod audit_routes;
mod auth;
mod auth_routes;
mod config;
mod db;
mod dept_routes;
mod error;
mod role_routes;
mod routes;
mod system_routes;

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
    sqlx::migrate!("./migrations").run(&db_pool).await?;
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

    let app = create_router(state);

    // 3. 启动 + graceful shutdown
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

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
