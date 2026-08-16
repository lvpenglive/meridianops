//! 个人工作台路由：聚合统计 + 最近活动，作为登录后默认首页。
//!
//! - GET /api/dashboard  返回当前用户视角的工作台数据（任意已登录用户可访问）
//!
//! 聚合内容：
//! - stats: 用户/角色/部门总数 + 今日操作数 + 今日登录成功数
//! - recentActivities: 全局最近 10 条审计日志
//! - myActivities: 当前用户最近 5 条审计日志

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::routing::get;
use axum::Json;
use axum::Router;
use serde::Deserialize;

use crate::auth;
use crate::db;
use crate::error::AppError;
use crate::routes::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/api/dashboard", get(get_dashboard))
}

#[derive(Debug, Deserialize)]
pub struct DashboardQuery {
    /// "今日"起始时间（RFC3339）。前端按本地时区计算今日 00:00 后传入。
    /// 为空时后端用 UTC 今日 00:00 兜底。
    pub since: Option<String>,
}

/// 默认 since：UTC 今日 00:00。前端未传 since 时使用。
fn default_since_utc() -> String {
    let today = chrono::Utc::now().date_naive();
    let dt = today.and_hms_opt(0, 0, 0).unwrap_or_default();
    chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc).to_rfc3339()
}

async fn get_dashboard(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Query(q): Query<DashboardQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    // 任意已登录用户可访问，无权限码要求
    crate::license_routes::require_active_license(&state.db).await?;
    let since = q.since.unwrap_or_else(default_since_utc);

    // 并发执行所有聚合查询，降低延迟
    let (users, roles, depts, today_ops, today_logins, recent, mine) = tokio::try_join!(
        db::count_users_summary(&state.db),
        db::count_roles_total(&state.db),
        db::count_departments_total(&state.db),
        db::count_audit_logs_since(&state.db, Some(&since)),
        db::count_login_success_since(&state.db, Some(&since)),
        db::list_recent_audit_logs(&state.db, 10),
        db::list_recent_audit_logs_by_actor(&state.db, auth.username(), 5),
    )?;

    let (total_users, enabled_users) = users;

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": {
            "stats": {
                "totalUsers": total_users,
                "enabledUsers": enabled_users,
                "totalRoles": roles,
                "totalDepartments": depts,
                "todayOps": today_ops,
                "todayLogins": today_logins,
            },
            "recentActivities": recent,
            "myActivities": mine,
        }
    })))
}
