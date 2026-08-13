use std::sync::Arc;

use axum::extract::{Query, State};
use axum::routing::get;
use axum::Json;
use axum::Router;
use serde::{Deserialize, Serialize};

use crate::auth;
use crate::auth_routes::UserInfo;
use crate::db;
use crate::error::AppError;
use crate::routes::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/reports/login-trend", get(login_trend))
        .route("/api/reports/login-failed-top", get(login_failed_top))
        .route("/api/reports/locked-users", get(locked_users))
        .route("/api/reports/sensitive-ops-trend", get(sensitive_ops_trend))
        .route("/api/reports/sensitive-ops-top", get(sensitive_ops_top))
        .route("/api/reports/sensitive-ops-list", get(sensitive_ops_list))
        .route("/api/reports/compliance-summary", get(compliance_summary))
        .route("/api/reports/inactive-users", get(inactive_users))
        .route("/api/reports/role-assignment", get(role_assignment))
}

#[derive(Debug, Deserialize)]
pub struct DaysQuery {
    pub days: Option<i64>,
}
#[derive(Debug, Deserialize)]
pub struct DaysLimitQuery {
    pub days: Option<i64>,
    pub limit: Option<i64>,
}
#[derive(Debug, Deserialize)]
pub struct SensitiveListQuery {
    pub days: Option<i64>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}
#[derive(Debug, Deserialize)]
pub struct InactiveQuery {
    pub days: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginTrendItem {
    pub date: String,
    pub success: i64,
    pub failed: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FailedTopItem {
    pub username: String,
    pub failed_count: i64,
    pub last_failed_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SensitiveTrendItem {
    pub date: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SensitiveTopItem {
    pub username: String,
    pub count: i64,
    pub last_action_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SensitiveListResponse {
    pub total: i64,
    pub page: u64,
    pub page_size: u64,
    pub items: Vec<db::AuditLog>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComplianceSummary {
    pub total_users: i64,
    pub weak_password_count: i64,
    pub expired_password_count: i64,
    pub inactive_90d_count: i64,
    pub password_expiry_days: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleAssignmentItem {
    pub role_name: String,
    pub user_count: i64,
}

/// 登录趋势（成功 vs 失败）。默认近 30 天。
async fn login_trend(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Query(q): Query<DaysQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "audit:read")?;
    let days = q.days.unwrap_or(30).clamp(1, 365);
    let rows = db::report_login_trend(&state.db, days).await?;
    let data: Vec<LoginTrendItem> = rows
        .into_iter()
        .map(|(d, s, f)| LoginTrendItem {
            date: d,
            success: s,
            failed: f,
        })
        .collect();
    Ok(Json(serde_json::json!({ "code": 0, "data": data })))
}

/// 失败登录 TOP 用户。默认近 30 天，TOP 10。
async fn login_failed_top(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Query(q): Query<DaysLimitQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "audit:read")?;
    let days = q.days.unwrap_or(30).clamp(1, 365);
    let limit = q.limit.unwrap_or(10).clamp(1, 100);
    let rows = db::report_login_failed_top(&state.db, days, limit).await?;
    let data: Vec<FailedTopItem> = rows
        .into_iter()
        .map(|(u, c, t)| FailedTopItem {
            username: u,
            failed_count: c,
            last_failed_at: t,
        })
        .collect();
    Ok(Json(serde_json::json!({ "code": 0, "data": data })))
}

/// 当前锁定账号列表。
async fn locked_users(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "audit:read")?;
    let users = db::report_locked_users(&state.db).await?;
    let data: Vec<UserInfo> = users.into_iter().map(UserInfo::from).collect();
    Ok(Json(serde_json::json!({ "code": 0, "data": data })))
}

/// 敏感操作趋势。默认近 30 天。
async fn sensitive_ops_trend(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Query(q): Query<DaysQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "audit:read")?;
    let days = q.days.unwrap_or(30).clamp(1, 365);
    let rows = db::report_sensitive_ops_trend(&state.db, days).await?;
    let data: Vec<SensitiveTrendItem> = rows
        .into_iter()
        .map(|(d, c)| SensitiveTrendItem { date: d, count: c })
        .collect();
    Ok(Json(serde_json::json!({ "code": 0, "data": data })))
}

/// 敏感操作 TOP 操作人。默认近 30 天，TOP 10。
async fn sensitive_ops_top(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Query(q): Query<DaysLimitQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "audit:read")?;
    let days = q.days.unwrap_or(30).clamp(1, 365);
    let limit = q.limit.unwrap_or(10).clamp(1, 100);
    let rows = db::report_sensitive_ops_top(&state.db, days, limit).await?;
    let data: Vec<SensitiveTopItem> = rows
        .into_iter()
        .map(|(u, c, t)| SensitiveTopItem {
            username: u,
            count: c,
            last_action_at: t,
        })
        .collect();
    Ok(Json(serde_json::json!({ "code": 0, "data": data })))
}

/// 敏感操作明细分页查询。
async fn sensitive_ops_list(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Query(q): Query<SensitiveListQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "audit:read")?;
    let days = q.days.unwrap_or(30).clamp(1, 365);
    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(20).clamp(1, 100);
    let (total, items) = db::report_sensitive_ops_list(&state.db, days, page, page_size).await?;
    Ok(Json(serde_json::json!({
        "code": 0,
        "data": SensitiveListResponse { total, page, page_size, items }
    })))
}

/// 合规健康度摘要。
async fn compliance_summary(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "audit:read")?;
    let expiry_days = db::get_setting(&state.db, "password_expiry_days")
        .await
        .ok()
        .flatten()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(0);
    let (total, weak, expired, inactive) =
        db::report_compliance_summary(&state.db, expiry_days).await?;
    Ok(Json(serde_json::json!({
        "code": 0,
        "data": ComplianceSummary {
            total_users: total,
            weak_password_count: weak,
            expired_password_count: expired,
            inactive_90d_count: inactive,
            password_expiry_days: expiry_days,
        }
    })))
}

/// 长期未登录用户。默认 90 天。
async fn inactive_users(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Query(q): Query<InactiveQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "audit:read")?;
    let days = q.days.unwrap_or(90).clamp(1, 365);
    let users = db::report_inactive_users(&state.db, days).await?;
    let data: Vec<UserInfo> = users.into_iter().map(UserInfo::from).collect();
    Ok(Json(serde_json::json!({ "code": 0, "data": data })))
}

/// 角色权限分配统计。
async fn role_assignment(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "audit:read")?;
    let rows = db::report_role_assignment(&state.db).await?;
    let data: Vec<RoleAssignmentItem> = rows
        .into_iter()
        .map(|(n, c)| RoleAssignmentItem {
            role_name: n,
            user_count: c,
        })
        .collect();
    Ok(Json(serde_json::json!({ "code": 0, "data": data })))
}
