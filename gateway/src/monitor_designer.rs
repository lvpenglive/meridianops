//! 第 3 期模板设计器入口。权限和审计在这里，Zabbix 调用在 zabbix-ctl。
//! 实例 `protected=true` 时，zabbix-ctl 拒绝直接写入。门户先提交标准变更工单，关闭后再执行。

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};
use sqlx::Row;

use crate::audit;
use crate::auth;
use crate::error::AppError;
use crate::license_routes;
use crate::monitor_routes::{ctl_request, ctl_request_approved};
use crate::routes::AppState;
use crate::ticket_routes::{self, CreateTicketReq};

const CHANGE_TEMPLATE: &str = "00000000-0000-0000-0000-000000000003";

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/monitor/catalog", get(catalog))
        .route(
            "/api/monitor/instances/:code/template-groups/:group_id",
            axum::routing::put(rename_template_group).delete(delete_template_group),
        )
        .route(
            "/api/monitor/instances/:code/host-groups/:group_id",
            axum::routing::put(rename_host_group).delete(delete_host_group),
        )
        .route(
            "/api/monitor/instances/:code/templates/export",
            axum::routing::post(export_templates),
        )
        .route(
            "/api/monitor/instances/:code/templates/import",
            axum::routing::post(import_templates),
        )
        .route(
            "/api/monitor/instances/:code/templates/:template_id",
            get(get_template)
                .put(update_template)
                .delete(delete_template),
        )
        .route(
            "/api/monitor/instances/:code/templates/:template_id/items",
            get(list_items).post(create_item),
        )
        .route(
            "/api/monitor/instances/:code/templates/:template_id/items/:item_id",
            axum::routing::put(update_item).delete(delete_item),
        )
        .route(
            "/api/monitor/instances/:code/templates/:template_id/triggers",
            get(list_triggers).post(create_trigger),
        )
        .route(
            "/api/monitor/instances/:code/templates/:template_id/triggers/:trigger_id",
            axum::routing::put(update_trigger).delete(delete_trigger),
        )
        .route(
            "/api/monitor/instances/:code/templates/:template_id/macros",
            get(template_macros).put(save_template_macros),
        )
        .route(
            "/api/monitor/instances/:code/templates/:template_id/discovery-rules",
            get(list_rules).post(create_rule),
        )
        .route(
            "/api/monitor/instances/:code/templates/:template_id/discovery-rules/:rule_id",
            axum::routing::put(update_rule).delete(delete_rule),
        )
        .route(
            "/api/monitor/instances/:code/templates/:template_id/discovery-rules/:rule_id/items",
            get(list_proto_items).post(create_proto_item),
        )
        .route(
            "/api/monitor/instances/:code/templates/:template_id/discovery-rules/:rule_id/items/:item_id",
            axum::routing::put(update_proto_item).delete(delete_proto_item),
        )
        .route(
            "/api/monitor/instances/:code/templates/:template_id/discovery-rules/:rule_id/triggers",
            get(list_proto_triggers).post(create_proto_trigger),
        )
        .route(
            "/api/monitor/instances/:code/templates/:template_id/discovery-rules/:rule_id/triggers/:trigger_id",
            axum::routing::put(update_proto_trigger).delete(delete_proto_trigger),
        )
        .route(
            "/api/monitor/instances/:code/template-changes",
            get(list_changes).post(submit_change),
        )
        .route(
            "/api/monitor/template-changes/:id/apply",
            axum::routing::post(apply_change),
        )
}

async fn catalog(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
) -> Result<Json<Value>, AppError> {
    permit_read(&state, &auth).await?;
    let payload = ctl_request(&state, reqwest::Method::GET, "/api/catalog", None, None).await?;
    Ok(Json(payload))
}

pub(crate) async fn create_template_group(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(code): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    write_direct(
        &state,
        &auth,
        &code,
        reqwest::Method::POST,
        &format!("/api/instances/{code}/template-groups"),
        Some(&body),
        "monitor_template_group_create",
    )
    .await
}

pub(crate) async fn create_host_group(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(code): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    write_direct(
        &state,
        &auth,
        &code,
        reqwest::Method::POST,
        &format!("/api/instances/{code}/host-groups"),
        Some(&body),
        "monitor_host_group_create",
    )
    .await
}

async fn rename_template_group(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, group_id)): Path<(String, String)>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    require_zbx_id(&group_id)?;
    write_direct(
        &state,
        &auth,
        &code,
        reqwest::Method::PUT,
        &format!("/api/instances/{code}/template-groups/{group_id}"),
        Some(&body),
        "monitor_template_group_rename",
    )
    .await
}

async fn delete_template_group(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, group_id)): Path<(String, String)>,
) -> Result<Json<Value>, AppError> {
    require_zbx_id(&group_id)?;
    write_direct(
        &state,
        &auth,
        &code,
        reqwest::Method::DELETE,
        &format!("/api/instances/{code}/template-groups/{group_id}"),
        None,
        "monitor_template_group_delete",
    )
    .await
}

async fn rename_host_group(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, group_id)): Path<(String, String)>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    require_zbx_id(&group_id)?;
    write_direct(
        &state,
        &auth,
        &code,
        reqwest::Method::PUT,
        &format!("/api/instances/{code}/host-groups/{group_id}"),
        Some(&body),
        "monitor_host_group_rename",
    )
    .await
}

async fn delete_host_group(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, group_id)): Path<(String, String)>,
) -> Result<Json<Value>, AppError> {
    require_zbx_id(&group_id)?;
    write_direct(
        &state,
        &auth,
        &code,
        reqwest::Method::DELETE,
        &format!("/api/instances/{code}/host-groups/{group_id}"),
        None,
        "monitor_host_group_delete",
    )
    .await
}

pub(crate) async fn create_template(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(code): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    write_direct(
        &state,
        &auth,
        &code,
        reqwest::Method::POST,
        &format!("/api/instances/{code}/templates"),
        Some(&body),
        "monitor_template_create",
    )
    .await
}

async fn get_template(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, template_id)): Path<(String, String)>,
) -> Result<Json<Value>, AppError> {
    permit_read(&state, &auth).await?;
    require_code(&code)?;
    require_zbx_id(&template_id)?;
    let payload = ctl_request(
        &state,
        reqwest::Method::GET,
        &format!("/api/instances/{code}/templates/{template_id}"),
        None,
        None,
    )
    .await?;
    Ok(Json(payload))
}

async fn update_template(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, template_id)): Path<(String, String)>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    require_zbx_id(&template_id)?;
    write_direct(
        &state,
        &auth,
        &code,
        reqwest::Method::PUT,
        &format!("/api/instances/{code}/templates/{template_id}"),
        Some(&body),
        "monitor_template_update",
    )
    .await
}

async fn delete_template(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, template_id)): Path<(String, String)>,
) -> Result<Json<Value>, AppError> {
    require_zbx_id(&template_id)?;
    write_direct(
        &state,
        &auth,
        &code,
        reqwest::Method::DELETE,
        &format!("/api/instances/{code}/templates/{template_id}"),
        None,
        "monitor_template_delete",
    )
    .await
}

async fn list_items(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, template_id)): Path<(String, String)>,
) -> Result<Json<Value>, AppError> {
    read_child(&state, &auth, &code, &template_id, "items").await
}

async fn create_item(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, template_id)): Path<(String, String)>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    write_child(&state, &auth, &code, &template_id, "items", reqwest::Method::POST, None, Some(&body), "monitor_item_create").await
}

async fn update_item(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, template_id, item_id)): Path<(String, String, String)>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    write_child(
        &state,
        &auth,
        &code,
        &template_id,
        "items",
        reqwest::Method::PUT,
        Some(&item_id),
        Some(&body),
        "monitor_item_update",
    )
    .await
}

async fn delete_item(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, template_id, item_id)): Path<(String, String, String)>,
) -> Result<Json<Value>, AppError> {
    write_child(
        &state,
        &auth,
        &code,
        &template_id,
        "items",
        reqwest::Method::DELETE,
        Some(&item_id),
        None,
        "monitor_item_delete",
    )
    .await
}

async fn list_triggers(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, template_id)): Path<(String, String)>,
) -> Result<Json<Value>, AppError> {
    read_child(&state, &auth, &code, &template_id, "triggers").await
}

async fn create_trigger(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, template_id)): Path<(String, String)>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    write_child(
        &state,
        &auth,
        &code,
        &template_id,
        "triggers",
        reqwest::Method::POST,
        None,
        Some(&body),
        "monitor_trigger_create",
    )
    .await
}

async fn update_trigger(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, template_id, trigger_id)): Path<(String, String, String)>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    write_child(
        &state,
        &auth,
        &code,
        &template_id,
        "triggers",
        reqwest::Method::PUT,
        Some(&trigger_id),
        Some(&body),
        "monitor_trigger_update",
    )
    .await
}

async fn delete_trigger(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, template_id, trigger_id)): Path<(String, String, String)>,
) -> Result<Json<Value>, AppError> {
    write_child(
        &state,
        &auth,
        &code,
        &template_id,
        "triggers",
        reqwest::Method::DELETE,
        Some(&trigger_id),
        None,
        "monitor_trigger_delete",
    )
    .await
}

async fn template_macros(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, template_id)): Path<(String, String)>,
) -> Result<Json<Value>, AppError> {
    read_child(&state, &auth, &code, &template_id, "macros").await
}

async fn save_template_macros(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, template_id)): Path<(String, String)>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    write_child(
        &state,
        &auth,
        &code,
        &template_id,
        "macros",
        reqwest::Method::PUT,
        None,
        Some(&body),
        "monitor_template_macro_update",
    )
    .await
}

async fn export_templates(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(code): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    permit_template(&state, &auth).await?;
    require_code(&code)?;
    let payload = ctl_request(
        &state,
        reqwest::Method::POST,
        &format!("/api/instances/{code}/templates/export"),
        None,
        Some(&body),
    )
    .await?;
    let bytes = payload["data"]["source"].as_str().map(|s| s.len()).unwrap_or(0);
    audit_ok(
        &state,
        &auth,
        "monitor_template_export",
        &code,
        &json!({ "templateIds": body.get("templateIds"), "bytes": bytes }),
    )
    .await;
    Ok(Json(payload))
}

async fn import_templates(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(code): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    write_direct(
        &state,
        &auth,
        &code,
        reqwest::Method::POST,
        &format!("/api/instances/{code}/templates/import"),
        Some(&body),
        "monitor_template_import",
    )
    .await
}

async fn list_rules(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, template_id)): Path<(String, String)>,
) -> Result<Json<Value>, AppError> {
    read_child(&state, &auth, &code, &template_id, "discovery-rules").await
}

async fn create_rule(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, template_id)): Path<(String, String)>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    write_child(&state, &auth, &code, &template_id, "discovery-rules", reqwest::Method::POST, None, Some(&body), "monitor_lld_create").await
}

async fn update_rule(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, template_id, rule_id)): Path<(String, String, String)>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    write_child(&state, &auth, &code, &template_id, "discovery-rules", reqwest::Method::PUT, Some(&rule_id), Some(&body), "monitor_lld_update").await
}

async fn delete_rule(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, template_id, rule_id)): Path<(String, String, String)>,
) -> Result<Json<Value>, AppError> {
    write_child(&state, &auth, &code, &template_id, "discovery-rules", reqwest::Method::DELETE, Some(&rule_id), None, "monitor_lld_delete").await
}

async fn list_proto_items(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, template_id, rule_id)): Path<(String, String, String)>,
) -> Result<Json<Value>, AppError> {
    read_under_rule(&state, &auth, &code, &template_id, &rule_id, "items", None).await
}

async fn create_proto_item(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, template_id, rule_id)): Path<(String, String, String)>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    write_under_rule(&state, &auth, &code, &template_id, &rule_id, "items", reqwest::Method::POST, None, Some(&body), "monitor_lld_item_create").await
}

async fn update_proto_item(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, template_id, rule_id, item_id)): Path<(String, String, String, String)>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    write_under_rule(&state, &auth, &code, &template_id, &rule_id, "items", reqwest::Method::PUT, Some(&item_id), Some(&body), "monitor_lld_item_update").await
}

async fn delete_proto_item(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, template_id, rule_id, item_id)): Path<(String, String, String, String)>,
) -> Result<Json<Value>, AppError> {
    write_under_rule(&state, &auth, &code, &template_id, &rule_id, "items", reqwest::Method::DELETE, Some(&item_id), None, "monitor_lld_item_delete").await
}

async fn list_proto_triggers(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, template_id, rule_id)): Path<(String, String, String)>,
) -> Result<Json<Value>, AppError> {
    read_under_rule(&state, &auth, &code, &template_id, &rule_id, "triggers", None).await
}

async fn create_proto_trigger(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, template_id, rule_id)): Path<(String, String, String)>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    write_under_rule(&state, &auth, &code, &template_id, &rule_id, "triggers", reqwest::Method::POST, None, Some(&body), "monitor_lld_trigger_create").await
}

async fn update_proto_trigger(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, template_id, rule_id, trigger_id)): Path<(String, String, String, String)>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    write_under_rule(&state, &auth, &code, &template_id, &rule_id, "triggers", reqwest::Method::PUT, Some(&trigger_id), Some(&body), "monitor_lld_trigger_update").await
}

async fn delete_proto_trigger(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((code, template_id, rule_id, trigger_id)): Path<(String, String, String, String)>,
) -> Result<Json<Value>, AppError> {
    write_under_rule(&state, &auth, &code, &template_id, &rule_id, "triggers", reqwest::Method::DELETE, Some(&trigger_id), None, "monitor_lld_trigger_delete").await
}

async fn read_under_rule(
    state: &AppState,
    auth: &auth::AuthUser,
    code: &str,
    template_id: &str,
    rule_id: &str,
    child: &str,
    _object_id: Option<&str>,
) -> Result<Json<Value>, AppError> {
    permit_read(state, auth).await?;
    require_code(code)?;
    require_zbx_id(template_id)?;
    require_zbx_id(rule_id)?;
    let payload = ctl_request(
        state,
        reqwest::Method::GET,
        &format!("/api/instances/{code}/templates/{template_id}/discovery-rules/{rule_id}/{child}"),
        None,
        None,
    )
    .await?;
    Ok(Json(payload))
}

async fn write_under_rule(
    state: &AppState,
    auth: &auth::AuthUser,
    code: &str,
    template_id: &str,
    rule_id: &str,
    child: &str,
    method: reqwest::Method,
    object_id: Option<&str>,
    body: Option<&Value>,
    action: &str,
) -> Result<Json<Value>, AppError> {
    require_zbx_id(template_id)?;
    require_zbx_id(rule_id)?;
    if let Some(id) = object_id {
        require_zbx_id(id)?;
    }
    let path = match object_id {
        Some(id) => format!("/api/instances/{code}/templates/{template_id}/discovery-rules/{rule_id}/{child}/{id}"),
        None => format!("/api/instances/{code}/templates/{template_id}/discovery-rules/{rule_id}/{child}"),
    };
    write_direct(state, auth, code, method, &path, body, action).await
}

async fn read_child(
    state: &AppState,
    auth: &auth::AuthUser,
    code: &str,
    template_id: &str,
    child: &str,
) -> Result<Json<Value>, AppError> {
    permit_read(state, auth).await?;
    require_code(code)?;
    require_zbx_id(template_id)?;
    let payload = ctl_request(
        state,
        reqwest::Method::GET,
        &format!("/api/instances/{code}/templates/{template_id}/{child}"),
        None,
        None,
    )
    .await?;
    Ok(Json(payload))
}

async fn write_child(
    state: &AppState,
    auth: &auth::AuthUser,
    code: &str,
    template_id: &str,
    child: &str,
    method: reqwest::Method,
    object_id: Option<&str>,
    body: Option<&Value>,
    action: &str,
) -> Result<Json<Value>, AppError> {
    require_zbx_id(template_id)?;
    if let Some(id) = object_id {
        require_zbx_id(id)?;
    }
    let path = match object_id {
        Some(id) => format!("/api/instances/{code}/templates/{template_id}/{child}/{id}"),
        None => format!("/api/instances/{code}/templates/{template_id}/{child}"),
    };
    write_direct(state, auth, code, method, &path, body, action).await
}

async fn write_direct(
    state: &AppState,
    auth: &auth::AuthUser,
    code: &str,
    method: reqwest::Method,
    path: &str,
    body: Option<&Value>,
    action: &str,
) -> Result<Json<Value>, AppError> {
    permit_template(state, auth).await?;
    require_code(code)?;
    let pre = if method == reqwest::Method::DELETE {
        before_snapshot(state, code, path).await
    } else {
        None
    };
    let payload = ctl_request(state, method, path, None, body).await?;
    let snapshot = if pre.is_some() {
        pre
    } else if let Some(id) = template_id_from_path(path)
        .or_else(|| payload["data"]["templateId"].as_str().map(|s| s.to_string()))
    {
        export_template(state, code, &id).await
    } else {
        None
    };
    let detail = json!({
        "path": path,
        "request": summarize_body(body),
        "snapshotBytes": snapshot.as_ref().map(|s| s.len()).unwrap_or(0),
        "snapshot": snapshot,
    });
    audit_ok(state, auth, action, code, &detail).await;
    Ok(Json(payload))
}

async fn list_changes(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(code): Path<String>,
) -> Result<Json<Value>, AppError> {
    permit_read(&state, &auth).await?;
    require_code(&code)?;
    let rows = sqlx::query(
        "SELECT id, ticket_id, title, http_method, resource_path, status, requested_by, created_at, applied_at
         FROM monitor_template_changes WHERE instance_code = ? ORDER BY created_at DESC LIMIT 50",
    )
    .bind(&code)
    .fetch_all(&state.db)
    .await
    .map_err(|e| AppError::internal(format!("查询模板变更失败: {e}")))?;
    let items = rows
        .iter()
        .map(|row| {
            json!({
                "id": row.try_get::<String, _>("id").unwrap_or_default(),
                "ticketId": row.try_get::<String, _>("ticket_id").unwrap_or_default(),
                "title": row.try_get::<String, _>("title").unwrap_or_default(),
                "method": row.try_get::<String, _>("http_method").unwrap_or_default(),
                "path": row.try_get::<String, _>("resource_path").unwrap_or_default(),
                "status": row.try_get::<String, _>("status").unwrap_or_default(),
                "requestedBy": row.try_get::<String, _>("requested_by").unwrap_or_default(),
                "createdAt": row.try_get::<chrono::NaiveDateTime, _>("created_at").ok().map(|t| t.to_string()),
                "appliedAt": row.try_get::<Option<chrono::NaiveDateTime>, _>("applied_at").ok().flatten().map(|t| t.to_string()),
            })
        })
        .collect::<Vec<_>>();
    Ok(Json(json!({ "code": 0, "data": items })))
}

async fn submit_change(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(code): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    permit_template(&state, &auth).await?;
    require_code(&code)?;
    let title = body["title"].as_str().unwrap_or("").trim();
    if title.is_empty() || title.chars().count() > 120 {
        return Err(AppError::bad("变更标题不能为空且不超过 120 字"));
    }
    let method = body["method"].as_str().unwrap_or("").trim().to_ascii_uppercase();
    if !matches!(method.as_str(), "POST" | "PUT" | "DELETE") {
        return Err(AppError::bad("变更方法无效"));
    }
    let resource = body["resource"].as_str().unwrap_or("").trim();
    let path = format!("/api/instances/{code}{resource}");
    if !designer_path(&code, &path) {
        return Err(AppError::bad("变更路径不在模板设计器范围内"));
    }
    let payload = body.get("body").cloned().unwrap_or_else(|| json!({}));
    let stored = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
    if stored.len() > 2_000_000 {
        return Err(AppError::bad("变更内容超过 2MB"));
    }
    let assignee = if auth.0.uid.is_empty() {
        None
    } else {
        Some(auth.0.uid.clone())
    };
    let created = ticket_routes::create_ticket(
        State(state.clone()),
        auth.clone(),
        Json(CreateTicketReq {
            ticketType: "change".to_string(),
            title: format!("监控模板变更：{title}"),
            description: Some(format!("{method} {path}")),
            priority: 3,
            category: Some("监控模板".to_string()),
            assigneeId: assignee,
            templateId: Some(CHANGE_TEMPLATE.to_string()),
            alertIds: Vec::new(),
            extra: json!({ "instanceCode": code, "resource": path }),
        }),
    )
    .await?;
    let ticket_id = created.0["data"]["id"].as_str().unwrap_or("").to_string();
    let ticket_no = created.0["data"]["ticketNo"].as_str().unwrap_or("").to_string();
    if ticket_id.is_empty() {
        return Err(AppError::internal("变更工单未返回编号"));
    }
    let change_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO monitor_template_changes
         (id, instance_code, ticket_id, title, http_method, resource_path, body, status, requested_by, created_at)
         VALUES (?,?,?,?,?,?,?,?,?,NOW())",
    )
    .bind(&change_id)
    .bind(&code)
    .bind(&ticket_id)
    .bind(title)
    .bind(&method)
    .bind(&path)
    .bind(&stored)
    .bind("pending")
    .bind(&auth.0.sub)
    .execute(&state.db)
    .await
    .map_err(|e| AppError::internal(format!("保存模板变更失败: {e}")))?;
    audit_ok(
        &state,
        &auth,
        "monitor_template_change_submit",
        &code,
        &json!({ "changeId": change_id, "ticketId": ticket_id, "path": path }),
    )
    .await;
    Ok(Json(json!({
        "code": 0,
        "data": { "id": change_id, "ticketId": ticket_id, "ticketNo": ticket_no }
    })))
}

async fn apply_change(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    permit_template(&state, &auth).await?;
    if uuid::Uuid::parse_str(&id).is_err() {
        return Err(AppError::bad("变更编号无效"));
    }
    let row = sqlx::query(
        "SELECT instance_code, ticket_id, http_method, resource_path, body, status
         FROM monitor_template_changes WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::internal(format!("查询模板变更失败: {e}")))?
    .ok_or_else(|| AppError::not_found("模板变更不存在"))?;
    let status: String = row.try_get("status").unwrap_or_default();
    if status != "pending" {
        return Err(AppError::bad("该变更已经执行"));
    }
    let ticket_id: String = row.try_get("ticket_id").unwrap_or_default();
    let ticket_status: Option<String> = sqlx::query_scalar(
        "SELECT status FROM tickets WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(&ticket_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| AppError::internal(format!("查询变更工单失败: {e}")))?;
    if ticket_status.as_deref() != Some("closed") {
        return Err(AppError::bad("变更工单尚未关闭，不能写入 Zabbix"));
    }
    let code: String = row.try_get("instance_code").unwrap_or_default();
    let method_name: String = row.try_get("http_method").unwrap_or_default();
    let path: String = row.try_get("resource_path").unwrap_or_default();
    let raw: Option<String> = row.try_get("body").ok().flatten();
    require_code(&code)?;
    if !designer_path(&code, &path) {
        return Err(AppError::bad("变更路径不在模板设计器范围内"));
    }
    let method = match method_name.as_str() {
        "POST" => reqwest::Method::POST,
        "PUT" => reqwest::Method::PUT,
        "DELETE" => reqwest::Method::DELETE,
        _ => return Err(AppError::bad("变更方法无效")),
    };
    let parsed = raw
        .as_deref()
        .map(serde_json::from_str::<Value>)
        .transpose()
        .map_err(|_| AppError::bad("变更内容无法解析"))?;
    let snapshot = before_snapshot(&state, &code, &path).await;
    let payload = ctl_request_approved(&state, method, &path, parsed.as_ref()).await?;
    sqlx::query(
        "UPDATE monitor_template_changes SET status = 'applied', applied_at = NOW(), snapshot = ? WHERE id = ?",
    )
    .bind(snapshot.as_deref())
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(|e| AppError::internal(format!("更新模板变更失败: {e}")))?;
    audit_ok(
        &state,
        &auth,
        "monitor_template_change_apply",
        &code,
        &json!({
            "changeId": id,
            "ticketId": ticket_id,
            "path": path,
            "snapshotBytes": snapshot.as_ref().map(|s| s.len()).unwrap_or(0),
        }),
    )
    .await;
    Ok(Json(payload))
}

fn designer_path(code: &str, path: &str) -> bool {
    let Some(rest) = path.strip_prefix(&format!("/api/instances/{code}/")) else {
        return false;
    };
    if rest.contains("..") || rest.contains('?') || rest.contains('#') {
        return false;
    }
    rest.starts_with("template-groups")
        || rest.starts_with("host-groups")
        || rest.starts_with("templates")
}

fn summarize_body(body: Option<&Value>) -> Value {
    let Some(body) = body else {
        return json!({});
    };
    if body.get("source").and_then(|v| v.as_str()).is_some() {
        return json!({ "sourceBytes": body["source"].as_str().map(|s| s.len()).unwrap_or(0) });
    }
    body.clone()
}

async fn before_snapshot(state: &AppState, code: &str, path: &str) -> Option<String> {
    if path.ends_with("/export") || path.ends_with("/import") {
        return None;
    }
    let id = template_id_from_path(path)?;
    export_template(state, code, &id).await
}

fn template_id_from_path(path: &str) -> Option<String> {
    let rest = path.split("/templates/").nth(1)?;
    let id = rest.split('/').next().unwrap_or("");
    if !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()) {
        Some(id.to_string())
    } else {
        None
    }
}

async fn export_template(state: &AppState, code: &str, template_id: &str) -> Option<String> {
    let payload = ctl_request(
        state,
        reqwest::Method::POST,
        &format!("/api/instances/{code}/templates/export"),
        None,
        Some(&json!({ "templateIds": [template_id] })),
    )
    .await
    .ok()?;
    let source = payload["data"]["source"].as_str()?.to_string();
    if source.len() > 60_000 {
        let mut end = 60_000;
        while !source.is_char_boundary(end) {
            end -= 1;
        }
        Some(format!("{}…", &source[..end]))
    } else {
        Some(source)
    }
}

async fn permit_read(state: &AppState, auth: &auth::AuthUser) -> Result<(), AppError> {
    auth::require_permission(auth, "monitor:read")?;
    license_routes::require_active_license(&state.db).await?;
    Ok(())
}

async fn permit_template(state: &AppState, auth: &auth::AuthUser) -> Result<(), AppError> {
    auth::require_permission(auth, "monitor:template")?;
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

async fn audit_ok(state: &AppState, auth: &auth::AuthUser, action: &str, target: &str, detail: &Value) {
    audit::log_async(&state.db, auth, action, "zbx_template", target, Some(detail), "", "success").await;
}
