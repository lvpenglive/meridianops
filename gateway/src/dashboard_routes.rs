//! 个人工作台路由：聚合统计 + 最近活动，作为登录后默认首页。
//!
//! - GET /api/dashboard  返回当前用户视角的工作台数据（任意已登录用户可访问）
//!
//! 聚合内容：
//! - stats: 用户/角色/部门总数 + 今日操作数 + 今日登录成功数
//! - opsStats: 资产总数/CI模型数/作业定义/今日作业执行/同步数据源
//! - modelStats: 模型分布（code/name/count/icon），用于资产分布图
//! - recentJobRuns: 最近 5 条作业执行摘要
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
    let (
        users, roles, depts, today_ops, today_logins, recent, mine,
        ci_total, model_stats, job_def_summary, job_runs_today, sync_summary, recent_job_runs,
    ) = tokio::try_join!(
        db::count_users_summary(&state.db),
        db::count_roles_total(&state.db),
        db::count_departments_total(&state.db),
        db::count_audit_logs_since(&state.db, Some(&since)),
        db::count_login_success_since(&state.db, Some(&since)),
        db::list_recent_audit_logs(&state.db, 10),
        db::list_recent_audit_logs_by_actor(&state.db, auth.username(), 5),
        db::count_ci_instances_total(&state.db),
        db::list_ci_model_stats(&state.db),
        db::count_job_definitions_summary(&state.db),
        db::count_job_runs_today(&state.db, &since),
        db::count_sync_sources_summary(&state.db),
        db::list_recent_job_runs(&state.db, 5),
    )?;

    let (total_users, enabled_users) = users;
    let (total_job_defs, enabled_job_defs) = job_def_summary;
    let (today_job_runs, today_job_success) = job_runs_today;
    let (total_sync_sources, enabled_sync_sources) = sync_summary;

    // 模型分布（top 8 + 其他）
    let model_list: Vec<serde_json::Value> = {
        let mut items: Vec<(String, String, i64, String)> = model_stats
            .0
            .iter()
            .map(|m| (m.code.clone(), m.name.clone(), m.count, m.icon.clone()))
            .collect();
        // 按实例数降序
        items.sort_by(|a, b| b.2.cmp(&a.2));
        let top_n = 8;
        let (head, tail) = if items.len() > top_n {
            items.split_at(top_n)
        } else {
            (&items[..], &[][..])
        };
        let other_count: i64 = tail.iter().map(|(_, _, c, _)| *c).sum();
        let mut list: Vec<serde_json::Value> = head
            .iter()
            .map(|(code, name, count, icon)| {
                serde_json::json!({
                    "code": code,
                    "name": name,
                    "count": count,
                    "icon": icon,
                })
            })
            .collect();
        if other_count > 0 {
            list.push(serde_json::json!({
                "code": "_other",
                "name": "其他模型",
                "count": other_count,
                "icon": "MoreFilled",
            }));
        }
        list
    };

    // 最近作业执行列表
    let job_runs_list: Vec<serde_json::Value> = recent_job_runs
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "jobName": r.job_name,
                "triggerMode": r.trigger_mode,
                "overallStatus": r.overall_status,
                "targetCount": r.target_count,
                "successCount": r.success_count,
                "failedCount": r.failed_count,
                "startedBy": r.started_by,
                "startedAt": r.started_at,
                "finishedAt": r.finished_at,
            })
        })
        .collect();

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
            "opsStats": {
                "totalAssets": ci_total,
                "totalModels": model_stats.0.len() as i64,
                "totalJobDefs": total_job_defs,
                "enabledJobDefs": enabled_job_defs,
                "todayJobRuns": today_job_runs,
                "todayJobSuccess": today_job_success,
                "totalSyncSources": total_sync_sources,
                "enabledSyncSources": enabled_sync_sources,
            },
            "modelStats": model_list,
            "recentJobRuns": job_runs_list,
            "recentActivities": recent,
            "myActivities": mine,
        }
    })))
}
