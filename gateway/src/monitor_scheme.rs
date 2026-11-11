//! 第 1 期改方案：主机组、模板组、套用模板、主机宏、登记主机、确认问题。
//! Zabbix 调用仍在 zabbix-ctl。这里只做权限、配置项范围和审计。

use std::collections::HashSet;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;

use crate::audit;
use crate::auth::{self, Role};
use crate::error::AppError;
use crate::license_routes;
use crate::monitor_routes::ctl_request;
use crate::routes::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/monitor/instances/:code/template-groups",
            get(template_groups).post(crate::monitor_designer::create_template_group),
        )
        .route(
            "/api/monitor/instances/:code/host-groups",
            get(host_groups).post(crate::monitor_designer::create_host_group),
        )
        .route(
            "/api/monitor/instances/:code/host-groups/:group_id/hosts",
            get(host_group_hosts),
        )
        .route(
            "/api/monitor/instances/:code/templates",
            get(templates).post(crate::monitor_designer::create_template),
        )
        .route(
            "/api/monitor/instances/:code/hosts/:host_id/linkage",
            get(host_linkage),
        )
        .route(
            "/api/monitor/instances/:code/hosts/:host_id/templates",
            post(link_templates),
        )
        .route(
            "/api/monitor/instances/:code/hosts/:host_id/groups",
            post(link_groups),
        )
        .route(
            "/api/monitor/instances/:code/hosts/:host_id/macros",
            get(host_macros).put(save_macros),
        )
        .route("/api/monitor/instances/:code/hosts", post(register_host))
        .route(
            "/api/monitor/instances/:code/events/:event_id/ack",
            post(ack_event),
        )
}

async fn template_groups(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(code): Path<String>,
) -> Result<Json<Value>, AppError> {
    permit_read(&state, &auth).await?;
    require_code(&code)?;
    let payload = ctl_request(
        &state,
        reqwest::Method::GET,
        &format!("/api/instances/{code}/template-groups"),
        None,
        None,
    )
    .await?;
    Ok(Json(payload))
}

async fn host_groups(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(code): Path<String>,
) -> Result<Json<Value>, AppError> {
    permit_read(&state, &auth).await?;
    require_code(&code)?;
    let payload = ctl_request(
        &state,
        reqwest::Method::GET,
        &format!("/api/instances/{code}/host-groups"),
        None,
        None,
    )
    .await?;
    Ok(Json(payload))
}

#[derive(Debug, Deserialize)]
struct GroupQuery {
    #[serde(rename = "groupId")]
    group_id: Option<String>,
}

async fn templates(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(code): Path<String>,
    Query(q): Query<GroupQuery>,
) -> Result<Json<Value>, AppError> {
    permit_read(&state, &auth).await?;
    require_code(&code)?;
    let query = q
        .group_id
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|id| format!("groupId={id}"));
    if let Some(id) = q.group_id.as_deref().filter(|s| !s.is_empty()) {
        require_zbx_id(id)?;
    }
    let payload = ctl_request(
        &state,
        reqwest::Method::GET,
        &format!("/api/instances/{code}/templates"),
        query.as_deref(),
        None,
    )
    .await?;
    Ok(Json(payload))
}

async fn host_group_hosts(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, group_id)): Path<(String, String)>,
) -> Result<Json<Value>, AppError> {
    permit_read(&state, &auth).await?;
    require_code(&code)?;
    require_zbx_id(&group_id)?;
    let payload = ctl_request(
        &state,
        reqwest::Method::GET,
        &format!("/api/instances/{code}/host-groups/{group_id}/hosts"),
        None,
        None,
    )
    .await?;
    let allowed = allowed_host_ids(&state, &auth, &code).await?;
    let hosts = payload["data"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|h| {
            h["hostId"]
                .as_str()
                .map(|id| allowed.contains(id))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    Ok(Json(json!({ "code": 0, "data": hosts })))
}

async fn host_linkage(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, host_id)): Path<(String, String)>,
) -> Result<Json<Value>, AppError> {
    permit_read(&state, &auth).await?;
    require_host(&state, &auth, &code, &host_id).await?;
    let payload = ctl_request(
        &state,
        reqwest::Method::GET,
        &format!("/api/instances/{code}/hosts/{host_id}/linkage"),
        None,
        None,
    )
    .await?;
    Ok(Json(payload))
}

async fn host_macros(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, host_id)): Path<(String, String)>,
) -> Result<Json<Value>, AppError> {
    permit_read(&state, &auth).await?;
    require_host(&state, &auth, &code, &host_id).await?;
    let payload = ctl_request(
        &state,
        reqwest::Method::GET,
        &format!("/api/instances/{code}/hosts/{host_id}/macros"),
        None,
        None,
    )
    .await?;
    Ok(Json(payload))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LinkBody {
    action: String,
    template_ids: Option<Vec<String>>,
    group_ids: Option<Vec<String>>,
}

async fn link_templates(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, host_id)): Path<(String, String)>,
    Json(body): Json<LinkBody>,
) -> Result<Json<Value>, AppError> {
    permit_operate(&state, &auth).await?;
    require_host(&state, &auth, &code, &host_id).await?;
    require_action(&body.action)?;
    let ids = body.template_ids.clone().unwrap_or_default();
    if ids.is_empty() {
        return Err(AppError::bad("模板列表为空"));
    }
    let payload = json!({ "action": body.action, "templateIds": ids });
    let result = ctl_request(
        &state,
        reqwest::Method::POST,
        &format!("/api/instances/{code}/hosts/{host_id}/templates"),
        None,
        Some(&payload),
    )
    .await?;
    audit_ok(
        &state,
        &auth,
        "monitor_template_link",
        &host_id,
        &json!({ "instanceCode": code, "action": body.action, "templateIds": ids }),
    )
    .await;
    Ok(Json(result))
}

async fn link_groups(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, host_id)): Path<(String, String)>,
    Json(body): Json<LinkBody>,
) -> Result<Json<Value>, AppError> {
    permit_operate(&state, &auth).await?;
    require_host(&state, &auth, &code, &host_id).await?;
    require_action(&body.action)?;
    let ids = body.group_ids.clone().unwrap_or_default();
    if ids.is_empty() {
        return Err(AppError::bad("主机组列表为空"));
    }
    let payload = json!({ "action": body.action, "groupIds": ids });
    let result = ctl_request(
        &state,
        reqwest::Method::POST,
        &format!("/api/instances/{code}/hosts/{host_id}/groups"),
        None,
        Some(&payload),
    )
    .await?;
    audit_ok(
        &state,
        &auth,
        "monitor_group_link",
        &host_id,
        &json!({ "instanceCode": code, "action": body.action, "groupIds": ids }),
    )
    .await;
    Ok(Json(result))
}

#[derive(Debug, Deserialize)]
struct MacroBody {
    macros: Vec<Value>,
}

async fn save_macros(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, host_id)): Path<(String, String)>,
    Json(body): Json<MacroBody>,
) -> Result<Json<Value>, AppError> {
    permit_operate(&state, &auth).await?;
    require_host(&state, &auth, &code, &host_id).await?;
    let payload = json!({ "macros": body.macros });
    let result = ctl_request(
        &state,
        reqwest::Method::PUT,
        &format!("/api/instances/{code}/hosts/{host_id}/macros"),
        None,
        Some(&payload),
    )
    .await?;
    audit_ok(
        &state,
        &auth,
        "monitor_macro_update",
        &host_id,
        &json!({ "instanceCode": code, "count": body.macros.len() }),
    )
    .await;
    Ok(Json(result))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegisterBody {
    ci_id: String,
    group_ids: Vec<String>,
    #[serde(default)]
    template_ids: Vec<String>,
}

async fn register_host(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(code): Path<String>,
    Json(body): Json<RegisterBody>,
) -> Result<Json<Value>, AppError> {
    permit_operate(&state, &auth).await?;
    require_code(&code)?;
    let ci = load_ci(&state, &body.ci_id).await?;
    ensure_ci_visible(&state, &auth, &ci).await?;
    let ip = attr_str(&ci.attributes, &["ip", "mgmt_ip"]);
    let hostname = attr_str(&ci.attributes, &["hostname"]).unwrap_or_else(|| ci.name.clone());
    let ip = ip.ok_or_else(|| AppError::bad("配置项没有管理 IP"))?;
    if hostname.trim().is_empty() {
        return Err(AppError::bad("配置项没有主机名"));
    }
    let payload = json!({
        "hostname": hostname,
        "displayName": ci.name,
        "ip": ip,
        "groupIds": body.group_ids,
        "templateIds": body.template_ids,
    });
    let result = ctl_request(
        &state,
        reqwest::Method::POST,
        &format!("/api/instances/{code}/hosts"),
        None,
        Some(&payload),
    )
    .await?;
    let host_id = result["data"]["hostId"].as_str().unwrap_or("").to_string();
    if !host_id.is_empty() {
        sqlx::query(
            "INSERT INTO zbx_host_links (instance_code, host_id, hostname, ip, ci_id, synced_at) \
             VALUES (?, ?, ?, ?, ?, UTC_TIMESTAMP()) \
             ON DUPLICATE KEY UPDATE hostname = VALUES(hostname), ip = VALUES(ip), \
             ci_id = VALUES(ci_id), synced_at = UTC_TIMESTAMP()",
        )
        .bind(&code)
        .bind(&host_id)
        .bind(&hostname)
        .bind(&ip)
        .bind(&ci.id)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::internal(format!("写入主机对照失败: {e}")))?;
    }
    audit_ok(
        &state,
        &auth,
        "monitor_host_register",
        &host_id,
        &json!({ "instanceCode": code, "ciId": ci.id, "created": result["data"]["created"] }),
    )
    .await;
    Ok(Json(result))
}

#[derive(Debug, Deserialize)]
struct AckBody {
    #[serde(default)]
    message: String,
    severity: Option<u8>,
    #[serde(default)]
    close: bool,
}

async fn ack_event(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, event_id)): Path<(String, String)>,
    Json(body): Json<AckBody>,
) -> Result<Json<Value>, AppError> {
    permit_operate(&state, &auth).await?;
    require_code(&code)?;
    require_zbx_id(&event_id)?;
    let lookup = ctl_request(
        &state,
        reqwest::Method::GET,
        &format!("/api/instances/{code}/events/{event_id}"),
        None,
        None,
    )
    .await?;
    let host_ids = lookup["data"]["hostIds"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if host_ids.is_empty() {
        return Err(AppError::bad("事件没有关联主机"));
    }
    let allowed = allowed_host_ids(&state, &auth, &code).await?;
    for id in &host_ids {
        let id = id.as_str().unwrap_or("");
        if !allowed.contains(id) {
            return Err(AppError::forbidden("事件关联了权限范围外的主机"));
        }
    }
    let payload = json!({
        "message": body.message,
        "severity": body.severity,
        "close": body.close,
    });
    let result = ctl_request(
        &state,
        reqwest::Method::POST,
        &format!("/api/instances/{code}/events/{event_id}/ack"),
        None,
        Some(&payload),
    )
    .await?;
    audit_ok(
        &state,
        &auth,
        "monitor_event_ack",
        &event_id,
        &json!({ "instanceCode": code, "close": body.close, "severity": body.severity }),
    )
    .await;
    Ok(Json(result))
}

struct CiRow {
    id: String,
    name: String,
    department_id: Option<String>,
    owner_id: Option<String>,
    attributes: Value,
}

async fn load_ci(state: &AppState, ci_id: &str) -> Result<CiRow, AppError> {
    if ci_id.trim().is_empty() {
        return Err(AppError::bad("配置项不能为空"));
    }
    let row = sqlx::query(
        "SELECT id, name, department_id, owner_id, attributes FROM ci_instances WHERE id = ?",
    )
    .bind(ci_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::internal(format!("查询配置项失败: {e}")))?
    .ok_or_else(|| AppError::not_found("配置项不存在"))?;
    let attributes = row
        .try_get::<serde_json::Value, _>("attributes")
        .unwrap_or(json!({}));
    Ok(CiRow {
        id: row.try_get("id").unwrap_or_default(),
        name: row.try_get("name").unwrap_or_default(),
        department_id: row.try_get("department_id").ok().flatten(),
        owner_id: row.try_get("owner_id").ok().flatten(),
        attributes,
    })
}

fn attr_str(attributes: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(value) = attributes.get(*key).and_then(|v| v.as_str()) {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

async fn ensure_ci_visible(state: &AppState, auth: &auth::AuthUser, ci: &CiRow) -> Result<(), AppError> {
    if auth.0.role == Role::Admin {
        return Ok(());
    }
    let department = user_department(state, &auth.0.uid).await?;
    if ci.department_id.as_deref() == department.as_deref() && department.is_some() {
        return Ok(());
    }
    if ci.owner_id.as_deref() == Some(auth.0.uid.as_str()) {
        return Ok(());
    }
    let owned: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM ci_instance_owners WHERE instance_id = ? AND user_id = ? LIMIT 1",
    )
    .bind(&ci.id)
    .bind(&auth.0.uid)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::internal(format!("查询负责人失败: {e}")))?;
    if owned.is_some() {
        return Ok(());
    }
    Err(AppError::forbidden("配置项不在可操作范围内"))
}

async fn require_host(
    state: &AppState,
    auth: &auth::AuthUser,
    code: &str,
    host_id: &str,
) -> Result<(), AppError> {
    require_code(code)?;
    require_zbx_id(host_id)?;
    let allowed = allowed_host_ids(state, auth, code).await?;
    if !allowed.contains(host_id) {
        return Err(AppError::forbidden("主机不在可操作的配置项范围内"));
    }
    Ok(())
}

async fn allowed_host_ids(
    state: &AppState,
    auth: &auth::AuthUser,
    code: &str,
) -> Result<HashSet<String>, AppError> {
    let admin = auth.0.role == Role::Admin;
    let department = if admin {
        None
    } else {
        user_department(state, &auth.0.uid).await?
    };
    let rows = sqlx::query(
        "SELECT l.host_id \
         FROM zbx_host_links l \
         JOIN ci_instances c ON c.id = l.ci_id \
         WHERE l.instance_code = ? \
           AND l.ci_id IS NOT NULL AND l.ci_id <> '' \
           AND ( \
             ? = 1 \
             OR c.department_id = ? \
             OR c.owner_id = ? \
             OR EXISTS ( \
               SELECT 1 FROM ci_instance_owners o \
               WHERE o.instance_id = c.id AND o.user_id = ? \
             ) \
           )",
    )
    .bind(code)
    .bind(if admin { 1 } else { 0 })
    .bind(department.unwrap_or_default())
    .bind(&auth.0.uid)
    .bind(&auth.0.uid)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::internal(format!("查询可操作主机失败: {e}")))?;
    Ok(rows
        .into_iter()
        .filter_map(|r| r.try_get::<String, _>("host_id").ok())
        .collect())
}

async fn user_department(state: &AppState, uid: &str) -> Result<Option<String>, AppError> {
    let department: Option<Option<String>> =
        sqlx::query_scalar("SELECT department_id FROM users WHERE id = ?")
            .bind(uid)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| AppError::internal(format!("查询用户部门失败: {e}")))?;
    Ok(department.flatten().filter(|s| !s.is_empty()))
}

async fn permit_read(state: &AppState, auth: &auth::AuthUser) -> Result<(), AppError> {
    auth::require_permission(auth, "monitor:read")?;
    license_routes::require_active_license(&state.db).await?;
    Ok(())
}

async fn permit_operate(state: &AppState, auth: &auth::AuthUser) -> Result<(), AppError> {
    auth::require_permission(auth, "monitor:operate")?;
    license_routes::require_active_license(&state.db).await?;
    Ok(())
}

fn require_code(code: &str) -> Result<(), AppError> {
    if code.is_empty()
        || !code
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(AppError::bad("实例编码无效"));
    }
    Ok(())
}

fn require_zbx_id(id: &str) -> Result<(), AppError> {
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
        return Err(AppError::bad("标识无效"));
    }
    Ok(())
}

fn require_action(action: &str) -> Result<(), AppError> {
    if action == "link" || action == "unlink" {
        Ok(())
    } else {
        Err(AppError::bad("action 只能是 link 或 unlink"))
    }
}

pub(crate) async fn start_install(
    state: &AppState,
    auth: &auth::AuthUser,
    instance_code: &str,
    ci_ids: &[String],
    group_ids: &[String],
    template_ids: &[String],
    server: &str,
    server_active: &str,
    concurrency: i32,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(auth, "monitor:agent")?;
    license_routes::require_active_license(&state.db).await?;
    require_code(instance_code)?;
    if ci_ids.is_empty() {
        return Err(AppError::bad("请选择配置项"));
    }
    if group_ids.is_empty() || template_ids.is_empty() {
        return Err(AppError::bad("请选择主机组和模板"));
    }
    if !valid_server(server) {
        return Err(AppError::bad("Zabbix Server 地址无效"));
    }
    if !server_active.is_empty() && !valid_server(server_active) {
        return Err(AppError::bad("ServerActive 地址无效"));
    }
    for ci_id in ci_ids {
        let ci = load_ci(state, ci_id).await?;
        ensure_ci_visible(state, auth, &ci).await?;
        if attr_str(&ci.attributes, &["ip", "mgmt_ip"]).is_none() {
            return Err(AppError::bad(&format!("配置项 {} 没有管理 IP", ci.name)));
        }
    }
    let payload = json!({
        "instanceCode": instance_code,
        "action": "install",
        "ciIds": ci_ids,
        "groupIds": group_ids,
        "templateIds": template_ids,
        "server": server,
        "serverActive": server_active,
        "concurrency": concurrency,
        "requestedBy": auth.0.sub,
    });
    let result = ctl_request(state, reqwest::Method::POST, "/api/tasks", None, Some(&payload)).await?;
    let task_id = result["data"]["id"].as_str().unwrap_or("");
    audit_ok(
        state,
        auth,
        "monitor_agent_install",
        task_id,
        &json!({
            "instanceCode": instance_code,
            "ciCount": ci_ids.len(),
            "server": server,
        }),
    )
    .await;
    Ok(Json(result))
}

fn valid_server(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value.len() <= 255
        && !value.chars().any(|c| c.is_whitespace() || c == '\'' || c == '"' || c == ';')
}

async fn audit_ok(state: &AppState, auth: &auth::AuthUser, action: &str, target: &str, detail: &Value) {
    audit::log_async(
        &state.db,
        auth,
        action,
        "zabbix_host",
        target,
        Some(detail),
        "",
        "success",
    )
    .await;
}
