//! 运维工具：第三方平台外链。打开方式为浏览器新标签；单点登录后续再做。

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{ConnectInfo, Path, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use sqlx::Row;
use uuid::Uuid;

use crate::audit;
use crate::auth;
use crate::error::AppError;
use crate::routes::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/ops-tools", get(list_tools).post(create_tool))
        .route(
            "/api/ops-tools/:id",
            get(get_tool).put(update_tool).delete(delete_tool),
        )
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ToolBody {
    name: String,
    url: String,
    #[serde(default)]
    description: String,
    #[serde(default = "default_icon")]
    icon: String,
    #[serde(default)]
    sort_order: i32,
    #[serde(default = "default_true")]
    enabled: bool,
}

fn default_icon() -> String {
    "Link".into()
}

fn default_true() -> bool {
    true
}

fn row_to_json(r: &sqlx::mysql::MySqlRow) -> serde_json::Value {
    serde_json::json!({
        "id": r.try_get::<String, _>("id").unwrap_or_default(),
        "name": r.try_get::<String, _>("name").unwrap_or_default(),
        "url": r.try_get::<String, _>("url").unwrap_or_default(),
        "description": r.try_get::<String, _>("description").unwrap_or_default(),
        "icon": r.try_get::<String, _>("icon").unwrap_or_else(|_| "Link".into()),
        "sortOrder": r.try_get::<i32, _>("sort_order").unwrap_or(0),
        "enabled": r.try_get::<i8, _>("enabled").unwrap_or(0) == 1,
        "createdAt": r.try_get::<chrono::NaiveDateTime, _>("created_at")
            .ok()
            .map(|t| t.format("%Y-%m-%dT%H:%M:%S").to_string()),
        "updatedAt": r.try_get::<chrono::NaiveDateTime, _>("updated_at")
            .ok()
            .map(|t| t.format("%Y-%m-%dT%H:%M:%S").to_string()),
    })
}

fn validate_body(body: &ToolBody) -> Result<(), AppError> {
    let name = body.name.trim();
    let url = body.url.trim();
    if name.is_empty() {
        return Err(AppError::bad("名称不能为空"));
    }
    if name.len() > 128 {
        return Err(AppError::bad("名称过长"));
    }
    if url.is_empty() {
        return Err(AppError::bad("URL 不能为空"));
    }
    if url.len() > 1024 {
        return Err(AppError::bad("URL 过长"));
    }
    let lower = url.to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return Err(AppError::bad("URL 须以 http:// 或 https:// 开头"));
    }
    Ok(())
}

/// 列表：普通用户只看启用项；有 manage 权限可见全部。
async fn list_tools(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "ops_tool:read")?;
    let manage = auth.has_permission("ops_tool:manage");
    let rows = if manage {
        sqlx::query(
            "SELECT id, name, url, description, icon, sort_order, enabled, created_at, updated_at \
             FROM ops_tools ORDER BY sort_order ASC, name ASC",
        )
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query(
            "SELECT id, name, url, description, icon, sort_order, enabled, created_at, updated_at \
             FROM ops_tools WHERE enabled = 1 ORDER BY sort_order ASC, name ASC",
        )
        .fetch_all(&state.db)
        .await?
    };
    let list: Vec<_> = rows.iter().map(row_to_json).collect();
    Ok(Json(serde_json::json!({ "code": 0, "data": list })))
}

async fn get_tool(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "ops_tool:read")?;
    let row = sqlx::query(
        "SELECT id, name, url, description, icon, sort_order, enabled, created_at, updated_at \
         FROM ops_tools WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::not_found("工具不存在"))?;
    if row.try_get::<i8, _>("enabled").unwrap_or(0) != 1 && !auth.has_permission("ops_tool:manage")
    {
        return Err(AppError::not_found("工具不存在"));
    }
    Ok(Json(serde_json::json!({ "code": 0, "data": row_to_json(&row) })))
}

async fn create_tool(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<ToolBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "ops_tool:manage")?;
    validate_body(&body)?;
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO ops_tools (id, name, url, description, icon, sort_order, enabled) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(body.name.trim())
    .bind(body.url.trim())
    .bind(body.description.trim())
    .bind(if body.icon.trim().is_empty() {
        "Link"
    } else {
        body.icon.trim()
    })
    .bind(body.sort_order)
    .bind(if body.enabled { 1i8 } else { 0 })
    .execute(&state.db)
    .await
    .map_err(|e| {
        if e.to_string().contains("Duplicate") {
            AppError::bad("名称已存在")
        } else {
            AppError::from(e)
        }
    })?;
    let detail = serde_json::json!({ "name": body.name.trim(), "url": body.url.trim() });
    let ip = audit::extract_ip(&headers, Some(addr));
    audit::log_async(
        &state.db,
        &auth,
        "ops_tool.create",
        "ops_tool",
        &id,
        Some(&detail),
        &ip,
        "success",
    )
    .await;
    Ok(Json(serde_json::json!({ "code": 0, "data": { "id": id } })))
}

async fn update_tool(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<ToolBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "ops_tool:manage")?;
    validate_body(&body)?;
    let result = sqlx::query(
        "UPDATE ops_tools SET name = ?, url = ?, description = ?, icon = ?, sort_order = ?, enabled = ? \
         WHERE id = ?",
    )
    .bind(body.name.trim())
    .bind(body.url.trim())
    .bind(body.description.trim())
    .bind(if body.icon.trim().is_empty() {
        "Link"
    } else {
        body.icon.trim()
    })
    .bind(body.sort_order)
    .bind(if body.enabled { 1i8 } else { 0 })
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        if e.to_string().contains("Duplicate") {
            AppError::bad("名称已存在")
        } else {
            AppError::from(e)
        }
    })?;
    if result.rows_affected() == 0 {
        return Err(AppError::not_found("工具不存在"));
    }
    let detail = serde_json::json!({ "name": body.name.trim(), "url": body.url.trim() });
    let ip = audit::extract_ip(&headers, Some(addr));
    audit::log_async(
        &state.db,
        &auth,
        "ops_tool.update",
        "ops_tool",
        &id,
        Some(&detail),
        &ip,
        "success",
    )
    .await;
    Ok(Json(serde_json::json!({ "code": 0, "data": { "id": id } })))
}

async fn delete_tool(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "ops_tool:manage")?;
    let result = sqlx::query("DELETE FROM ops_tools WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::not_found("工具不存在"));
    }
    let ip = audit::extract_ip(&headers, Some(addr));
    audit::log_async(
        &state.db,
        &auth,
        "ops_tool.delete",
        "ops_tool",
        &id,
        None,
        &ip,
        "success",
    )
    .await;
    Ok(Json(serde_json::json!({ "code": 0, "data": true })))
}
