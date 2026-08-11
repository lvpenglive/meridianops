use std::net::SocketAddr;
use std::sync::Arc;

use axum::http::HeaderMap;

use crate::auth;
use crate::db;
use crate::routes::AppState;

/// 便捷函数：在 handler 末尾记录审计日志。
/// 使用 `let _ = audit::log_async(...).await;` 方式调用，失败不阻塞主流程。
pub async fn log_async(
    pool: &db::DbPool,
    actor: &auth::AuthUser,
    action: &str,
    target_type: &str,
    target_id: &str,
    detail: Option<&serde_json::Value>,
    ip: &str,
    status: &str,
) {
    let actor_username = actor.0.sub.clone();
    let _ = db::insert_audit_log(
        pool,
        &actor_username,
        action,
        target_type,
        target_id,
        detail,
        ip,
        status,
    )
    .await;
}

/// 从请求头中提取客户端 IP。
/// 优先使用 X-Forwarded-For 头（代理场景），否则用 remote_addr。
pub fn extract_ip(headers: &HeaderMap, remote_addr: Option<SocketAddr>) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|v| v.trim().to_string())
        .unwrap_or_else(|| {
            remote_addr
                .map(|a| a.ip().to_string())
                .unwrap_or_else(|| "unknown".to_string())
        })
}

/// 审计中间件：当前仅透传请求，留作未来扩展。
pub async fn audit_middleware(
    _state: Arc<AppState>,
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response<axum::body::Body> {
    next.run(req).await
}