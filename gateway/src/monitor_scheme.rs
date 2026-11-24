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
            "/api/monitor/instances/:code/hosts/massupdate",
            post(mass_update_hosts),
        )
        .route(
            "/api/monitor/instances/:code/hosts/delete",
            post(delete_hosts),
        )
        .route(
            "/api/monitor/instances/:code/hosts/:host_id",
            get(get_host_detail).put(update_host),
        )
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

async fn register_host(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(code): Path<String>,
    Json(mut body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    permit_operate(&state, &auth).await?;
    require_code(&code)?;

    let ci_id = body["ciId"]
        .as_str()
        .or_else(|| body["ci_id"].as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let mut hostname = body["hostname"].as_str().unwrap_or("").trim().to_string();
    let mut display_name = body["displayName"]
        .as_str()
        .or_else(|| body["display_name"].as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let has_interfaces = body["interfaces"]
        .as_array()
        .map(|a| !a.is_empty())
        .unwrap_or(false);
    let has_ip = !body["ip"].as_str().unwrap_or("").trim().is_empty();

    // 兼容旧「登记配置项」：仅传 ciId + 组/模板时从 CI 推导主机名/IP
    let mut linked_ci_id: Option<String> = None;
    if !ci_id.is_empty() {
        let ci = load_ci(&state, &ci_id).await?;
        ensure_ci_visible(&state, &auth, &ci).await?;
        linked_ci_id = Some(ci.id.clone());
        if hostname.is_empty() {
            hostname = attr_str(&ci.attributes, &["hostname"]).unwrap_or_else(|| ci.name.clone());
            body["hostname"] = json!(hostname);
        }
        if display_name.is_empty() {
            display_name = ci.name.clone();
            body["displayName"] = json!(display_name);
        }
        if !has_interfaces && !has_ip {
            let ip = attr_str(&ci.attributes, &["ip", "mgmt_ip"])
                .ok_or_else(|| AppError::bad("配置项没有管理 IP"))?;
            body["ip"] = json!(ip);
        }
    }

    hostname = body["hostname"].as_str().unwrap_or("").trim().to_string();
    if hostname.is_empty() {
        return Err(AppError::bad("主机名不能为空"));
    }
    let group_ids = body["groupIds"]
        .as_array()
        .or_else(|| body["group_ids"].as_array())
        .cloned()
        .unwrap_or_default();
    if group_ids.is_empty() {
        return Err(AppError::bad("请选择主机组"));
    }

    let result = ctl_request(
        &state,
        reqwest::Method::POST,
        &format!("/api/instances/{code}/hosts"),
        None,
        Some(&body),
    )
    .await?;
    let host_id = result["data"]["hostId"].as_str().unwrap_or("").to_string();
    let link_ip = result["data"]["ip"]
        .as_str()
        .unwrap_or_else(|| body["ip"].as_str().unwrap_or(""))
        .to_string();
    let link_hostname = body["hostname"].as_str().unwrap_or(&hostname).to_string();

    if !host_id.is_empty() {
        let link_proxy_id = body["proxyId"]
            .as_str()
            .or_else(|| body["proxy_id"].as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let link_proxy_group_id = body["proxyGroupId"]
            .as_str()
            .or_else(|| body["proxy_group_id"].as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty());
        if let Some(ref cid) = linked_ci_id {
            sqlx::query(
                "INSERT INTO zbx_host_links \
                 (instance_code, host_id, hostname, ip, ci_id, proxy_id, proxy_group_id, synced_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, UTC_TIMESTAMP()) \
                 ON DUPLICATE KEY UPDATE hostname = VALUES(hostname), ip = VALUES(ip), \
                 ci_id = VALUES(ci_id), proxy_id = VALUES(proxy_id), \
                 proxy_group_id = VALUES(proxy_group_id), synced_at = UTC_TIMESTAMP()",
            )
            .bind(&code)
            .bind(&host_id)
            .bind(&link_hostname)
            .bind(&link_ip)
            .bind(cid)
            .bind(link_proxy_id)
            .bind(link_proxy_group_id)
            .execute(&state.db)
            .await
            .map_err(|e| AppError::internal(format!("写入主机对照失败: {e}")))?;
        } else {
            // 无 CI 也写入对照，方便名册展示；ci_id 留空
            sqlx::query(
                "INSERT INTO zbx_host_links \
                 (instance_code, host_id, hostname, ip, proxy_id, proxy_group_id, synced_at) \
                 VALUES (?, ?, ?, ?, ?, ?, UTC_TIMESTAMP()) \
                 ON DUPLICATE KEY UPDATE hostname = VALUES(hostname), ip = VALUES(ip), \
                 proxy_id = VALUES(proxy_id), proxy_group_id = VALUES(proxy_group_id), \
                 synced_at = UTC_TIMESTAMP()",
            )
            .bind(&code)
            .bind(&host_id)
            .bind(&link_hostname)
            .bind(&link_ip)
            .bind(link_proxy_id)
            .bind(link_proxy_group_id)
            .execute(&state.db)
            .await
            .map_err(|e| AppError::internal(format!("写入主机对照失败: {e}")))?;
        }
    }
    audit_ok(
        &state,
        &auth,
        "monitor_host_register",
        &host_id,
        &json!({
            "instanceCode": code,
            "ciId": linked_ci_id,
            "hostname": link_hostname,
            "created": result["data"]["created"],
        }),
    )
    .await;
    Ok(Json(result))
}

async fn get_host_detail(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, host_id)): Path<(String, String)>,
) -> Result<Json<Value>, AppError> {
    permit_read(&state, &auth).await?;
    require_code(&code)?;
    require_zbx_id(&host_id)?;
    let result = ctl_request(
        &state,
        reqwest::Method::GET,
        &format!("/api/instances/{code}/hosts/{host_id}"),
        None,
        None,
    )
    .await?;
    Ok(Json(result))
}

async fn update_host(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, host_id)): Path<(String, String)>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    permit_operate(&state, &auth).await?;
    require_code(&code)?;
    require_zbx_id(&host_id)?;
    let result = ctl_request(
        &state,
        reqwest::Method::PUT,
        &format!("/api/instances/{code}/hosts/{host_id}"),
        None,
        Some(&body),
    )
    .await?;
    {
        let hostname = body["hostname"].as_str().map(str::trim).filter(|s| !s.is_empty());
        let ip = body["interfaces"]
            .as_array()
            .and_then(|a| a.first())
            .and_then(|i| i["ip"].as_str())
            .or_else(|| body["ip"].as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let has_proxy_field = body.get("proxyId").is_some()
            || body.get("proxy_id").is_some()
            || body.get("proxyGroupId").is_some()
            || body.get("proxy_group_id").is_some();
        if hostname.is_some() || ip.is_some() || has_proxy_field {
            let mut proxy_id: Option<Option<&str>> = None;
            let mut proxy_group_id: Option<Option<&str>> = None;
            if has_proxy_field {
                let pid = body["proxyId"]
                    .as_str()
                    .or_else(|| body["proxy_id"].as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty());
                let gid = body["proxyGroupId"]
                    .as_str()
                    .or_else(|| body["proxy_group_id"].as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty());
                if let Some(g) = gid {
                    proxy_group_id = Some(Some(g));
                    proxy_id = Some(None);
                } else if let Some(p) = pid {
                    proxy_id = Some(Some(p));
                    proxy_group_id = Some(None);
                } else {
                    // 显式清空通道
                    proxy_id = Some(None);
                    proxy_group_id = Some(None);
                }
            }
            let _ = sqlx::query(
                "UPDATE zbx_host_links SET \
                 hostname = COALESCE(?, hostname), \
                 ip = COALESCE(?, ip), \
                 proxy_id = CASE WHEN ? THEN ? ELSE proxy_id END, \
                 proxy_group_id = CASE WHEN ? THEN ? ELSE proxy_group_id END, \
                 synced_at = UTC_TIMESTAMP() \
                 WHERE instance_code = ? AND host_id = ?",
            )
            .bind(hostname)
            .bind(ip)
            .bind(proxy_id.is_some())
            .bind(proxy_id.flatten())
            .bind(proxy_group_id.is_some())
            .bind(proxy_group_id.flatten())
            .bind(&code)
            .bind(&host_id)
            .execute(&state.db)
            .await;
        }
    }
    audit_ok(
        &state,
        &auth,
        "monitor_host_update",
        &host_id,
        &json!({ "instanceCode": code }),
    )
    .await;
    Ok(Json(result))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HostIdsBody {
    host_ids: Vec<String>,
}

async fn delete_hosts(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(code): Path<String>,
    Json(body): Json<HostIdsBody>,
) -> Result<Json<Value>, AppError> {
    permit_operate(&state, &auth).await?;
    require_code(&code)?;
    if body.host_ids.is_empty() {
        return Err(AppError::bad("请选择主机"));
    }
    for id in &body.host_ids {
        require_zbx_id(id)?;
    }
    let payload = json!({ "hostIds": body.host_ids });
    let result = ctl_request(
        &state,
        reqwest::Method::POST,
        &format!("/api/instances/{code}/hosts/delete"),
        None,
        Some(&payload),
    )
    .await?;
    for id in &body.host_ids {
        let _ = sqlx::query("DELETE FROM zbx_host_links WHERE instance_code = ? AND host_id = ?")
            .bind(&code)
            .bind(id)
            .execute(&state.db)
            .await;
    }
    audit_ok(
        &state,
        &auth,
        "monitor_host_delete",
        &body.host_ids.join(","),
        &json!({ "instanceCode": code, "count": body.host_ids.len() }),
    )
    .await;
    Ok(Json(result))
}

async fn mass_update_hosts(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(code): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    permit_operate(&state, &auth).await?;
    require_code(&code)?;
    let host_ids = body["hostIds"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if host_ids.is_empty() {
        return Err(AppError::bad("请选择主机"));
    }
    for id in &host_ids {
        require_zbx_id(id.as_str().unwrap_or(""))?;
    }
    let result = ctl_request(
        &state,
        reqwest::Method::POST,
        &format!("/api/instances/{code}/hosts/massupdate"),
        None,
        Some(&body),
    )
    .await?;
    audit_ok(
        &state,
        &auth,
        "monitor_host_massupdate",
        &host_ids
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(","),
        &json!({ "instanceCode": code, "count": host_ids.len() }),
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
    if auth.0.role != Role::Admin {
        let allowed = allowed_host_ids(&state, &auth, &code).await?;
        for id in &host_ids {
            let id = id.as_str().unwrap_or("");
            if !allowed.contains(id) {
                return Err(AppError::forbidden("事件关联了权限范围外的主机"));
            }
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

/// 改监控方案、读方案详情用。管理员可操作任意主机；其他人只能操作已关联且归自己管的配置项。
async fn require_host(
    state: &AppState,
    auth: &auth::AuthUser,
    code: &str,
    host_id: &str,
) -> Result<(), AppError> {
    require_code(code)?;
    require_zbx_id(host_id)?;
    if auth.0.role == Role::Admin {
        return Ok(());
    }
    let allowed = allowed_host_ids(state, auth, code).await?;
    if allowed.contains(host_id) {
        return Ok(());
    }
    let linked: Option<String> = sqlx::query_scalar(
        "SELECT ci_id FROM zbx_host_links \
         WHERE instance_code = ? AND host_id = ? \
           AND ci_id IS NOT NULL AND ci_id <> '' \
         LIMIT 1",
    )
    .bind(code)
    .bind(host_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::internal(format!("查询主机对照失败: {e}")))?;
    if linked.is_some() {
        return Err(AppError::forbidden("主机不在可操作的配置项范围内"));
    }
    Err(AppError::forbidden("主机未关联配置项，不能修改监控方案"))
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
    agent_port: &str,
    server_active: &str,
    proxy_id: &str,
    proxy_group_id: &str,
    package_id: &str,
    install_path: &str,
    conf_suffix: &str,
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
    crate::monitor_routes::ensure_agent_job_credential(&state.db, "install", ci_ids).await?;
    let package_id = package_id.trim();
    if package_id.is_empty() {
        return Err(AppError::bad("请选择 Agent 安装包"));
    }
    let _ = crate::monitor_agent_pkg::resolve_package_path(&state.db, package_id).await?;

    let proxy_id = proxy_id.trim();
    let proxy_group_id = proxy_group_id.trim();
    let mut conf_suffix = conf_suffix.trim().to_string();
    if conf_suffix.is_empty() && !proxy_group_id.is_empty() {
        if let Some(mapped) =
            crate::monitor_agent_pkg::lookup_conf_suffix(&state.db, instance_code, proxy_group_id)
                .await?
        {
            conf_suffix = mapped;
        }
    }
    if !server.is_empty() && !valid_server(server) {
        return Err(AppError::bad("Agent 的 Server / Proxy 地址无效"));
    }
    if !server_active.is_empty() && !valid_server(server_active) {
        return Err(AppError::bad("ServerActive 地址无效"));
    }
    let agent_port = normalize_agent_port(agent_port)?;
    if server.is_empty() && conf_suffix.is_empty() {
        return Err(AppError::bad("请填写 Server，或选择带 conf 映射的 Proxy Group / 手工填模板后缀"));
    }
    if !proxy_id.is_empty() && !proxy_group_id.is_empty() {
        return Err(AppError::bad("不能同时指定 Proxy 与 Proxy Group"));
    }
    if !proxy_id.is_empty() && !proxy_id.chars().all(|c| c.is_ascii_digit()) {
        return Err(AppError::bad("Proxy ID 无效"));
    }
    if !proxy_group_id.is_empty() && !proxy_group_id.chars().all(|c| c.is_ascii_digit()) {
        return Err(AppError::bad("Proxy Group ID 无效"));
    }
    let install_path = {
        let p = install_path.trim();
        if p.is_empty() {
            crate::monitor_agent_pkg::default_install_path().to_string()
        } else {
            p.to_string()
        }
    };
    if !install_path.starts_with('/') || install_path.contains("..") || install_path.len() > 512 {
        return Err(AppError::bad("安装目录无效"));
    }
    for ci_id in ci_ids {
        let ci = load_ci(state, ci_id).await?;
        ensure_ci_visible(state, auth, &ci).await?;
        if attr_str(&ci.attributes, &["ip", "mgmt_ip"]).is_none() {
            return Err(AppError::bad(&format!("配置项 {} 没有管理 IP", ci.name)));
        }
    }
    let mut payload = json!({
        "instanceCode": instance_code,
        "action": "install",
        "ciIds": ci_ids,
        "groupIds": group_ids,
        "templateIds": template_ids,
        "server": server,
        "agentPort": agent_port,
        "serverActive": server_active,
        "packageId": package_id,
        "installPath": install_path,
        "confSuffix": conf_suffix,
        "concurrency": concurrency,
        "requestedBy": auth.username(),
    });
    if !proxy_group_id.is_empty() {
        payload["proxyGroupId"] = json!(proxy_group_id);
    } else if !proxy_id.is_empty() {
        payload["proxyId"] = json!(proxy_id);
    }
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
            "agentPort": agent_port,
            "proxyId": proxy_id,
            "proxyGroupId": proxy_group_id,
            "packageId": package_id,
            "installPath": install_path,
            "confSuffix": conf_suffix,
        }),
    )
    .await;
    Ok(Json(result))
}

/// 空则默认 10050；否则须为 1–65535。
fn normalize_agent_port(value: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Ok("10050".to_string());
    }
    let Ok(n) = value.parse::<u16>() else {
        return Err(AppError::bad("Agent 端口无效"));
    };
    if n == 0 {
        return Err(AppError::bad("Agent 端口无效"));
    }
    Ok(n.to_string())
}

/// 允许单地址，或 Proxy Group 的逗号/分号多地址（含端口、IPv6 方括号）。
fn valid_server(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value.len() <= 1024
        && value.chars().all(|c| {
            c.is_ascii_alphanumeric()
                || matches!(c, '.' | ':' | ',' | ';' | '-' | '_' | '[' | ']')
        })
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
