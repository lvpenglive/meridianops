//! 监控纳管转发。浏览器只访问 Gateway，由这里校验权限后调用 zabbix-ctl。

use std::sync::Arc;

use axum::extract::{Path, RawQuery, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::audit;
use crate::auth;
use crate::error::AppError;
use crate::license_routes;
use crate::routes::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/monitor/instances", get(instances))
        .route("/api/monitor/instances/:code/hosts", get(hosts))
        .route(
            "/api/monitor/instances/:code/hosts/:host_id/problems",
            get(problems),
        )
        .route(
            "/api/monitor/instances/:code/hosts/:host_id/metrics",
            get(metrics),
        )
        .route("/api/monitor/instances/:code/sync", post(sync_hosts))
        .route("/api/monitor/instances/:code/governance", get(governance))
        .route("/api/monitor/tasks", post(create_task))
        .route("/api/monitor/tasks/:id", get(get_task))
}

async fn instances(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
) -> Result<impl IntoResponse, AppError> {
    permit_read(&state, &auth).await?;
    proxy_get(&state, "/api/instances", None).await
}

async fn hosts(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(code): Path<String>,
    RawQuery(query): RawQuery,
) -> Result<impl IntoResponse, AppError> {
    permit_read(&state, &auth).await?;
    proxy_get(
        &state,
        &format!("/api/instances/{code}/hosts"),
        query.as_deref(),
    )
    .await
}

async fn problems(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, host_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {
    permit_read(&state, &auth).await?;
    proxy_get(
        &state,
        &format!("/api/instances/{code}/hosts/{host_id}/problems"),
        None,
    )
    .await
}

async fn metrics(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, host_id)): Path<(String, String)>,
    RawQuery(query): RawQuery,
) -> Result<impl IntoResponse, AppError> {
    permit_read(&state, &auth).await?;
    proxy_get(
        &state,
        &format!("/api/instances/{code}/hosts/{host_id}/metrics"),
        query.as_deref(),
    )
    .await
}

async fn sync_hosts(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    permit_read(&state, &auth).await?;
    proxy_post(&state, &format!("/api/instances/{code}/sync"), &json!({})).await
}

async fn governance(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    permit_read(&state, &auth).await?;
    proxy_get(&state, &format!("/api/instances/{code}/governance"), None).await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateTaskBody {
    instance_code: String,
    action: String,
    host_ids: Vec<String>,
    target_version: Option<String>,
    concurrency: Option<i32>,
}

async fn create_task(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Json(body): Json<CreateTaskBody>,
) -> Result<impl IntoResponse, AppError> {
    match body.action.as_str() {
        "start" | "stop" | "restart" => {
            auth::require_permission(&auth, "monitor:operate")?;
        }
        "upgrade" | "rollback" => {
            auth::require_permission(&auth, "monitor:agent")?;
        }
        _ => return Err(AppError::bad("不支持的动作")),
    }
    license_routes::require_active_license(&state.db).await?;
    if body.host_ids.is_empty() {
        return Err(AppError::bad("主机列表为空"));
    }

    // 只允许已同步且关联了配置项的主机执行作业类动作
    let placeholders = body
        .host_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT host_id, ci_id FROM zbx_host_links \
         WHERE instance_code = ? AND host_id IN ({placeholders})"
    );
    let mut q = sqlx::query(&sql).bind(&body.instance_code);
    for id in &body.host_ids {
        q = q.bind(id);
    }
    let rows = q
        .fetch_all(&state.db)
        .await
        .map_err(|e| AppError::internal(format!("查询主机对照失败: {e}")))?;
    use sqlx::Row;
    let mut linked = std::collections::HashSet::new();
    let mut missing_ci = Vec::new();
    for r in &rows {
        let hid: String = r.try_get("host_id").unwrap_or_default();
        let ci: Option<String> = r.try_get("ci_id").ok().flatten();
        linked.insert(hid.clone());
        if ci.as_deref().unwrap_or("").is_empty() {
            missing_ci.push(hid);
        }
    }
    let not_synced: Vec<_> = body
        .host_ids
        .iter()
        .filter(|id| !linked.contains(id.as_str()))
        .cloned()
        .collect();
    if !not_synced.is_empty() {
        let msg = format!(
            "以下主机尚未同步对照，请先点「同步对照」: {}",
            not_synced.join(",")
        );
        return Err(AppError::bad(&msg));
    }
    if !missing_ci.is_empty() {
        let msg = format!(
            "以下主机未关联配置项，无法执行作业: {}",
            missing_ci.join(",")
        );
        return Err(AppError::bad(&msg));
    }

    let payload = json!({
        "instanceCode": body.instance_code,
        "action": body.action,
        "hostIds": body.host_ids,
        "targetVersion": body.target_version,
        "concurrency": body.concurrency.unwrap_or(10),
        "requestedBy": auth.0.sub,
    });
    let resp = proxy_post(&state, "/api/tasks", &payload).await?;

    let detail = json!({
        "action": body.action,
        "instanceCode": body.instance_code,
        "hostCount": body.host_ids.len(),
    });
    audit::log_async(
        &state.db,
        &auth,
        "monitor_task_create",
        "zbx_tasks",
        "",
        Some(&detail),
        "",
        "success",
    )
    .await;
    Ok(resp)
}

async fn get_task(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    permit_read(&state, &auth).await?;
    proxy_get(&state, &format!("/api/tasks/{id}"), None).await
}

async fn permit_read(state: &AppState, auth: &auth::AuthUser) -> Result<(), AppError> {
    auth::require_permission(auth, "monitor:read")?;
    license_routes::require_active_license(&state.db).await?;
    Ok(())
}

async fn proxy_get(
    state: &AppState,
    path: &str,
    query: Option<&str>,
) -> Result<impl IntoResponse, AppError> {
    let (base, token) = ctl_endpoint(state)?;
    let mut url = format!("{base}{path}");
    if let Some(q) = query.filter(|s| !s.is_empty()) {
        url.push('?');
        url.push_str(q);
    }
    let response = state
        .client
        .get(url)
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| AppError::internal(format!("调用 zabbix-ctl 失败: {e}")))?;
    forward(response).await
}

async fn proxy_post(
    state: &AppState,
    path: &str,
    body: &Value,
) -> Result<impl IntoResponse, AppError> {
    let (base, token) = ctl_endpoint(state)?;
    let url = format!("{base}{path}");
    let response = state
        .client
        .post(url)
        .header("Authorization", format!("Bearer {token}"))
        .json(body)
        .send()
        .await
        .map_err(|e| AppError::internal(format!("调用 zabbix-ctl 失败: {e}")))?;
    forward(response).await
}

fn ctl_endpoint(state: &AppState) -> Result<(String, String), AppError> {
    let base = state.config.zabbix_ctl.base_url.trim_end_matches('/').to_string();
    if base.is_empty() {
        return Err(AppError::bad("未配置 zabbix-ctl 地址"));
    }
    let token = std::env::var("MERIDIANOPS_ZABBIX_CTL_TOKEN").unwrap_or_default();
    if token.is_empty() {
        return Err(AppError::bad("未配置 MERIDIANOPS_ZABBIX_CTL_TOKEN"));
    }
    Ok((base, token))
}

async fn forward(response: reqwest::Response) -> Result<impl IntoResponse, AppError> {
    let status = response.status();
    let body: Value = response
        .json()
        .await
        .map_err(|e| AppError::internal(format!("解析 zabbix-ctl 响应失败: {e}")))?;
    let code = StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    Ok((code, Json(body)))
}
