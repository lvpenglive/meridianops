//! 监控纳管转发。浏览器只访问 Gateway，由这里校验权限后调用 zabbix-ctl。

use std::sync::Arc;

use axum::extract::{Path, RawQuery, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;

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
        .route(
            "/api/monitor/instances/:code/hosts/:host_id/items",
            get(host_items),
        )
        .route(
            "/api/monitor/instances/:code/hosts/:host_id/triggers",
            get(host_triggers),
        )
        .route(
            "/api/monitor/instances/:code/hosts/:host_id/install-path",
            put(update_install_path),
        )
        .route("/api/monitor/instances/:code/sync", post(sync_hosts))
        .route("/api/monitor/instances/:code/governance", get(governance))
        .route("/api/monitor/instances/:code/proxies", get(proxies))
        .route("/api/monitor/instances/:code/proxy-groups", get(proxy_groups))
        .route("/api/monitor/tasks", post(create_task))
        .route("/api/monitor/tasks/:id", get(get_task))
        .merge(crate::monitor_scheme::routes())
        .merge(crate::monitor_designer::routes())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateInstallPathBody {
    /// 绝对路径；空字符串表示清空，启停时回退全局默认目录
    install_path: String,
    #[serde(default)]
    hostname: Option<String>,
    #[serde(default)]
    ip: Option<String>,
}

fn validate_install_path(raw: &str) -> Result<Option<String>, AppError> {
    let p = raw.trim();
    if p.is_empty() {
        return Ok(None);
    }
    if !p.starts_with('/') || p.contains("..") || p.len() > 512 {
        return Err(AppError::bad("安装目录无效：须为绝对路径，且不含 .."));
    }
    Ok(Some(p.to_string()))
}

/// 更新对照表安装路径。非平台安装机也可在关联/同步后手工填写。
async fn update_install_path(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, host_id)): Path<(String, String)>,
    Json(body): Json<UpdateInstallPathBody>,
) -> Result<Json<Value>, AppError> {
    if !auth.has_permission("monitor:agent") && !auth.has_permission("monitor:operate") {
        return Err(AppError::forbidden("需要权限: monitor:agent 或 monitor:operate"));
    }
    license_routes::require_active_license(&state.db).await?;
    let code = code.trim();
    let host_id = host_id.trim();
    if code.is_empty() || host_id.is_empty() {
        return Err(AppError::bad("实例或主机无效"));
    }
    let path = validate_install_path(&body.install_path)?;
    let hostname = body
        .hostname
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("");
    let ip = body
        .ip
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("");

    let result = sqlx::query(
        "INSERT INTO zbx_host_links (instance_code, host_id, hostname, ip, install_path, synced_at) \
         VALUES (?, ?, ?, ?, ?, UTC_TIMESTAMP()) \
         ON DUPLICATE KEY UPDATE \
           install_path = VALUES(install_path), \
           hostname = IF(VALUES(hostname) <> '', VALUES(hostname), hostname), \
           ip = IF(VALUES(ip) <> '', VALUES(ip), ip), \
           synced_at = UTC_TIMESTAMP()",
    )
    .bind(code)
    .bind(host_id)
    .bind(hostname)
    .bind(ip)
    .bind(path.as_deref())
    .execute(&state.db)
    .await
    .map_err(|e| AppError::internal(format!("更新安装路径失败: {e}")))?;

    if result.rows_affected() == 0 {
        return Err(AppError::bad("更新安装路径失败"));
    }

    let row = sqlx::query(
        "SELECT install_path, hostname, ip, ci_id FROM zbx_host_links \
         WHERE instance_code = ? AND host_id = ?",
    )
    .bind(code)
    .bind(host_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| AppError::internal(format!("读取对照失败: {e}")))?;

    let saved: Option<String> = row.try_get("install_path").ok().flatten();
    let detail = json!({
        "instanceCode": code,
        "hostId": host_id,
        "installPath": saved,
    });
    audit::log_async(
        &state.db,
        &auth,
        "monitor_install_path_update",
        "zabbix_host",
        host_id,
        Some(&detail),
        "",
        "success",
    )
    .await;

    Ok(Json(json!({
        "code": 0,
        "data": {
            "hostId": host_id,
            "hostname": row.try_get::<String, _>("hostname").unwrap_or_default(),
            "ip": row.try_get::<String, _>("ip").unwrap_or_default(),
            "ciId": row.try_get::<Option<String>, _>("ci_id").ok().flatten(),
            "installPath": saved,
        }
    })))
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

async fn host_items(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, host_id)): Path<(String, String)>,
    RawQuery(query): RawQuery,
) -> Result<impl IntoResponse, AppError> {
    permit_read(&state, &auth).await?;
    proxy_get(
        &state,
        &format!("/api/instances/{code}/hosts/{host_id}/items"),
        query.as_deref(),
    )
    .await
}

async fn host_triggers(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, host_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {
    permit_read(&state, &auth).await?;
    proxy_get(
        &state,
        &format!("/api/instances/{code}/hosts/{host_id}/triggers"),
        None,
    )
    .await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncHostsBody {
    #[serde(default)]
    host_ids: Vec<String>,
}

async fn sync_hosts(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(code): Path<String>,
    Json(body): Json<SyncHostsBody>,
) -> Result<impl IntoResponse, AppError> {
    permit_read(&state, &auth).await?;
    // 全量对照可能超过默认 30s HTTP 超时；指定 hostIds 时很快
    let (base, token) = ctl_endpoint(&state)?;
    let url = format!("{base}/api/instances/{code}/sync");
    let payload = json!({ "hostIds": body.host_ids });
    let response = state
        .client
        .post(url)
        .timeout(std::time::Duration::from_secs(300))
        .header("Authorization", format!("Bearer {token}"))
        .json(&payload)
        .send()
        .await
        .map_err(|e| AppError::internal(format!("调用 zabbix-ctl 失败: {e}")))?;
    forward(response).await
}

async fn governance(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    permit_read(&state, &auth).await?;
    proxy_get(&state, &format!("/api/instances/{code}/governance"), None).await
}

async fn proxies(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    permit_read(&state, &auth).await?;
    proxy_get(&state, &format!("/api/instances/{code}/proxies"), None).await
}

async fn proxy_groups(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    permit_read(&state, &auth).await?;
    proxy_get(&state, &format!("/api/instances/{code}/proxy-groups"), None).await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateTaskBody {
    instance_code: String,
    action: String,
    #[serde(default)]
    host_ids: Vec<String>,
    target_version: Option<String>,
    concurrency: Option<i32>,
    #[serde(default)]
    ci_ids: Vec<String>,
    #[serde(default)]
    group_ids: Vec<String>,
    #[serde(default)]
    template_ids: Vec<String>,
    server: Option<String>,
    agent_port: Option<String>,
    server_active: Option<String>,
    proxy_id: Option<String>,
    proxy_group_id: Option<String>,
    package_id: Option<String>,
    install_path: Option<String>,
    conf_suffix: Option<String>,
    /// 验证通过后禁用主机，避免立刻告警
    disable_after_install: Option<bool>,
}

async fn create_task(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Json(body): Json<CreateTaskBody>,
) -> Result<axum::response::Response, AppError> {
    if body.action == "install" {
        let response = crate::monitor_scheme::start_install(
            &state,
            &auth,
            &body.instance_code,
            &body.ci_ids,
            &body.group_ids,
            &body.template_ids,
            body.server.as_deref().unwrap_or(""),
            body.agent_port.as_deref().unwrap_or(""),
            body.server_active.as_deref().unwrap_or(""),
            body.proxy_id.as_deref().unwrap_or(""),
            body.proxy_group_id.as_deref().unwrap_or(""),
            body.package_id.as_deref().unwrap_or(""),
            body.install_path.as_deref().unwrap_or(""),
            body.conf_suffix.as_deref().unwrap_or(""),
            body.disable_after_install.unwrap_or(false),
            body.concurrency.unwrap_or(10),
        )
        .await?;
        return Ok(response.into_response());
    }
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
    let mut ci_ids = Vec::new();
    for r in &rows {
        let hid: String = r.try_get("host_id").unwrap_or_default();
        let ci: Option<String> = r.try_get("ci_id").ok().flatten();
        linked.insert(hid.clone());
        if ci.as_deref().unwrap_or("").is_empty() {
            missing_ci.push(hid);
        } else if let Some(cid) = ci {
            ci_ids.push(cid);
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
            "以下主机尚未关联配置项，请先点「关联资源」: {}",
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
    ensure_agent_job_credential(&state.db, &body.action, &ci_ids).await?;

    let mut target_version = body.target_version.clone();
    let package_id = body.package_id.as_deref().map(str::trim).filter(|s| !s.is_empty());
    if matches!(body.action.as_str(), "upgrade" | "rollback") {
        let Some(pkg) = package_id else {
            return Err(AppError::bad("请选择 Agent 包"));
        };
        let _ = crate::monitor_agent_pkg::resolve_package_path(&state.db, pkg).await?;
        if target_version.as_deref().unwrap_or("").is_empty() {
            target_version = Some(
                crate::monitor_agent_pkg::resolve_package_version(&state.db, pkg).await?,
            );
        }
    }

    let mut payload = json!({
        "instanceCode": body.instance_code,
        "action": body.action,
        "hostIds": body.host_ids,
        "targetVersion": target_version,
        "concurrency": body.concurrency.unwrap_or(10),
        "requestedBy": auth.0.sub,
    });
    if let Some(pkg) = package_id {
        payload["packageId"] = json!(pkg);
    }
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
    Ok(resp.into_response())
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

pub(crate) async fn ctl_request(
    state: &AppState,
    method: reqwest::Method,
    path: &str,
    query: Option<&str>,
    body: Option<&Value>,
) -> Result<Value, AppError> {
    ctl_request_inner(state, method, path, query, body, false).await
}

pub(crate) async fn ctl_request_approved(
    state: &AppState,
    method: reqwest::Method,
    path: &str,
    body: Option<&Value>,
) -> Result<Value, AppError> {
    ctl_request_inner(state, method, path, None, body, true).await
}

async fn ctl_request_inner(
    state: &AppState,
    method: reqwest::Method,
    path: &str,
    query: Option<&str>,
    body: Option<&Value>,
    approved: bool,
) -> Result<Value, AppError> {
    let (base, token) = ctl_endpoint(state)?;
    let mut url = format!("{base}{path}");
    if let Some(q) = query.filter(|s| !s.is_empty()) {
        url.push('?');
        url.push_str(q);
    }
    let mut request = state
        .client
        .request(method, url)
        .header("Authorization", format!("Bearer {token}"));
    if approved {
        request = request.header("X-Monitor-Approved", "1");
    }
    if let Some(body) = body {
        request = request.json(body);
    }
    let response = request
        .send()
        .await
        .map_err(|e| AppError::internal(format!("调用 zabbix-ctl 失败: {e}")))?;
    let status = response.status();
    let payload: Value = response
        .json()
        .await
        .map_err(|e| AppError::internal(format!("解析 zabbix-ctl 响应失败: {e}")))?;
    if !status.is_success() {
        let message = payload["message"].as_str().unwrap_or("zabbix-ctl 调用失败");
        return Err(match status.as_u16() {
            400 => AppError::bad(message),
            401 => AppError::forbidden(message),
            404 => AppError::not_found(message),
            _ => AppError::internal(message),
        });
    }
    Ok(payload)
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
    let token = state.config.zabbix_ctl.resolved_service_token();
    if token.is_empty() {
        return Err(AppError::bad(
            "未配置 zabbix-ctl 令牌（toml [zabbix_ctl].service_token 或环境变量 MERIDIANOPS_ZABBIX_CTL_TOKEN）",
        ));
    }
    Ok((base, token))
}

/// 作业剧本若为 SSH 执行：剧本有默认凭据，或本批所有配置项均已绑定凭据。
pub(crate) async fn ensure_agent_job_credential(
    pool: &sqlx::MySqlPool,
    action: &str,
    ci_ids: &[String],
) -> Result<(), AppError> {
    let name = match action {
        "install" => "Zabbix Agent 安装",
        "start" => "Zabbix Agent 启动",
        "stop" => "Zabbix Agent 停止",
        "restart" => "Zabbix Agent 重启",
        "upgrade" => "Zabbix Agent 升级",
        "rollback" => "Zabbix Agent 回滚",
        _ => return Ok(()),
    };
    use sqlx::Row;
    let row = sqlx::query(
        "SELECT executor_type, credential_id FROM job_definitions \
         WHERE name = ? AND enabled = 1 ORDER BY id DESC LIMIT 1",
    )
    .bind(name)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::internal(format!("查询作业定义失败: {e}")))?;
    let Some(row) = row else {
        return Err(AppError::bad(&format!(
            "未找到作业「{name}」，请确认迁移已执行且剧本已启用"
        )));
    };
    let executor: String = row
        .try_get::<Option<String>, _>("executor_type")
        .ok()
        .flatten()
        .unwrap_or_else(|| "ssh".into());
    if executor != "ssh" {
        return Ok(());
    }
    let cred: Option<i64> = row.try_get("credential_id").ok().flatten();
    if cred.is_some() {
        return Ok(());
    }
    let map = crate::db::ci_ssh_credential_map(pool, ci_ids)
        .await
        .map_err(|e| AppError::internal(format!("查询配置项凭据失败: {e}")))?;
    let missing: Vec<&str> = ci_ids
        .iter()
        .filter(|id| map.get(*id).copied().flatten().is_none())
        .map(|s| s.as_str())
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    let sample: Vec<&str> = missing.iter().copied().take(5).collect();
    let more = if missing.len() > 5 {
        format!(" 等 {} 台", missing.len())
    } else {
        String::new()
    };
    Err(AppError::bad(&format!(
        "作业「{name}」未配置默认凭据，且以下配置项未绑定 SSH 凭据：{}{}。请到资产详情绑定，或在作业中心为该剧本配置默认凭据",
        sample.join(", "),
        more
    )))
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
