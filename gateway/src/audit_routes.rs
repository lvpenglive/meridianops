use std::sync::Arc;

use axum::extract::{Query, State};
use axum::routing::get;
use axum::Json;
use axum::Router;
use serde::{Deserialize, Serialize};

use crate::auth;
use crate::db;
use crate::error::AppError;
use crate::routes::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/api/audit-logs", get(list_audit_logs))
}

#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    pub actor: Option<String>,
    pub action: Option<String>,
    pub target_type: Option<String>,
    pub status: Option<String>,
    pub start_from: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditPageResponse {
    pub total: i64,
    pub page: u64,
    pub page_size: u64,
    pub items: Vec<db::AuditLog>,
}

/// 分页查询审计日志（仅 admin）。
/// 支持按 actor/action/target_type/status/start_from 筛选。
async fn list_audit_logs(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Query(q): Query<AuditQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "audit:read")?;

    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(20).clamp(1, 100);

    let (total, items) = db::query_audit_logs(
        &state.db,
        q.actor.as_deref(),
        q.action.as_deref(),
        q.target_type.as_deref(),
        q.status.as_deref(),
        q.start_from.as_deref(),
        page,
        page_size,
    )
    .await?;

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": AuditPageResponse { total, page, page_size, items }
    })))
}
