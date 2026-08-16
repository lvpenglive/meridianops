//! 字典管理路由
//!
//! 提供通用枚举值配置（知识库分类、告警级别等），支持动态增减选项。
//!
//! ## 端点
//!   GET    /api/dict/types                    列出所有字典类型               (dict:read)
//!   POST   /api/dict/types                    新建字典类型                   (dict:create)
//!   PUT    /api/dict/types/:code              编辑字典类型                   (dict:update)
//!   DELETE /api/dict/types/:code              删除字典类型（连带删项）       (dict:delete)
//!   GET    /api/dict/types/:code/items        列出某类型的有效项             (仅需登录)
//!   POST   /api/dict/types/:code/items        新建字典项                     (dict:create)
//!   PUT    /api/dict/items/:id                编辑字典项                     (dict:update)
//!   DELETE /api/dict/items/:id                删除字典项                     (dict:delete)

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{ConnectInfo, Path, State};
use axum::routing::{get, post};
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
        .route("/api/dict/types", get(list_dict_types).post(create_dict_type))
        .route(
            "/api/dict/types/:code",
            get(get_dict_type).put(update_dict_type).delete(delete_dict_type),
        )
        .route(
            "/api/dict/types/:code/items",
            get(list_dict_items).post(create_dict_item),
        )
        .route(
            "/api/dict/items/:id",
            axum::routing::put(update_dict_item).delete(delete_dict_item),
        )
}

// ---- 请求结构 ----

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateTypeRequest {
    code: String,
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    sort_order: i32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateTypeRequest {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    sort_order: Option<i32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateItemRequest {
    item_value: String,
    item_label: String,
    #[serde(default)]
    sort_order: i32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateItemRequest {
    item_label: String,
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    sort_order: Option<i32>,
}

// ---- 字典类型 handlers ----

/// GET /api/dict/types — 列出所有字典类型
async fn list_dict_types(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "dict:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let rows = sqlx::query(
        "SELECT code, name, description, enabled, sort_order, created_at, updated_at \
         FROM sys_dict_types ORDER BY sort_order, code",
    )
    .fetch_all(&state.db)
    .await?;

    let types: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "code": r.try_get::<String, _>("code").unwrap_or_default(),
                "name": r.try_get::<String, _>("name").unwrap_or_default(),
                "description": r.try_get::<Option<String>, _>("description").unwrap_or(None),
                "enabled": r.try_get::<i64, _>("enabled").unwrap_or(1) == 1,
                "sortOrder": r.try_get::<i64, _>("sort_order").unwrap_or(0),
                "createdAt": r.try_get::<String, _>("created_at").unwrap_or_default(),
                "updatedAt": r.try_get::<String, _>("updated_at").unwrap_or_default(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "code": 0, "data": types })))
}

/// GET /api/dict/types/:code — 获取单个字典类型（前端未直接使用，但保留为 RESTful 完整性）
async fn get_dict_type(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(code): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "dict:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let row = sqlx::query(
        "SELECT code, name, description, enabled, sort_order, created_at, updated_at \
         FROM sys_dict_types WHERE code = ?",
    )
    .bind(&code)
    .fetch_optional(&state.db)
    .await?;

    match row {
        Some(r) => Ok(Json(serde_json::json!({
            "code": 0,
            "data": {
                "code": r.try_get::<String, _>("code").unwrap_or_default(),
                "name": r.try_get::<String, _>("name").unwrap_or_default(),
                "description": r.try_get::<Option<String>, _>("description").unwrap_or(None),
                "enabled": r.try_get::<i64, _>("enabled").unwrap_or(1) == 1,
                "sortOrder": r.try_get::<i64, _>("sort_order").unwrap_or(0),
                "createdAt": r.try_get::<String, _>("created_at").unwrap_or_default(),
                "updatedAt": r.try_get::<String, _>("updated_at").unwrap_or_default(),
            }
        }))),
        None => Err(AppError::not_found("字典类型不存在")),
    }
}

/// POST /api/dict/types — 新建字典类型
async fn create_dict_type(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    Json(req): Json<CreateTypeRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "dict:create")?;
    crate::license_routes::require_active_license(&state.db).await?;

    if req.code.trim().is_empty() || req.name.trim().is_empty() {
        return Err(AppError::bad("类型编码和名称不能为空"));
    }

    let now = chrono::Utc::now().to_rfc3339();
    let result = sqlx::query(
        "INSERT INTO sys_dict_types (code, name, description, enabled, sort_order, created_at, updated_at) \
         VALUES (?, ?, ?, 1, ?, ?, ?)",
    )
    .bind(req.code.trim())
    .bind(req.name.trim())
    .bind(req.description.as_deref())
    .bind(req.sort_order)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await;

    if let Err(sqlx::Error::Database(ref e)) = result {
        if e.message().contains("Duplicate entry") {
            return Err(AppError::bad("类型编码已存在"));
        }
    }
    result?;

    let detail = serde_json::json!({ "code": req.code, "name": req.name });
    let ip = audit::extract_ip(&headers, Some(addr));
    audit::log_async(
        &state.db, &auth, "create_dict_type", "sys_dict_types",
        &req.code, Some(&detail), &ip, "success",
    ).await;

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": { "code": req.code }
    })))
}

/// PUT /api/dict/types/:code — 编辑字典类型
async fn update_dict_type(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    Path(code): Path<String>,
    Json(req): Json<UpdateTypeRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "dict:update")?;
    crate::license_routes::require_active_license(&state.db).await?;

    if req.name.trim().is_empty() {
        return Err(AppError::bad("名称不能为空"));
    }

    let now = chrono::Utc::now().to_rfc3339();
    let result = sqlx::query(
        "UPDATE sys_dict_types SET name = ?, description = COALESCE(?, description), \
         enabled = COALESCE(?, enabled), sort_order = COALESCE(?, sort_order), updated_at = ? \
         WHERE code = ?",
    )
    .bind(req.name.trim())
    .bind(req.description.as_deref())
    .bind(req.enabled.map(|b| b as i64))
    .bind(req.sort_order)
    .bind(&now)
    .bind(&code)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("字典类型不存在"));
    }

    let detail = serde_json::json!({ "code": code, "name": req.name });
    let ip = audit::extract_ip(&headers, Some(addr));
    audit::log_async(
        &state.db, &auth, "update_dict_type", "sys_dict_types",
        &code, Some(&detail), &ip, "success",
    ).await;

    Ok(Json(serde_json::json!({ "code": 0, "data": { "code": code } })))
}

/// DELETE /api/dict/types/:code — 删除字典类型（连带删除其下所有字典项）
async fn delete_dict_type(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    Path(code): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "dict:delete")?;
    crate::license_routes::require_active_license(&state.db).await?;

    // 先删字典项，再删类型
    sqlx::query("DELETE FROM sys_dict_items WHERE type_code = ?")
        .bind(&code)
        .execute(&state.db)
        .await?;

    let result = sqlx::query("DELETE FROM sys_dict_types WHERE code = ?")
        .bind(&code)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("字典类型不存在"));
    }

    let ip = audit::extract_ip(&headers, Some(addr));
    audit::log_async(
        &state.db, &auth, "delete_dict_type", "sys_dict_types",
        &code, None, &ip, "success",
    ).await;

    Ok(Json(serde_json::json!({ "code": 0, "data": null })))
}

// ---- 字典项 handlers ----

/// GET /api/dict/types/:code/items — 列出某类型的有效项（供下拉框用）
/// 仅需登录，不需 dict:read 权限
async fn list_dict_items(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(code): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    // 仅验证登录，不检查 dict:read 权限
    let _ = &auth;
    crate::license_routes::require_active_license(&state.db).await?;

    let rows = sqlx::query(
        "SELECT id, type_code, item_value, item_label, enabled, sort_order \
         FROM sys_dict_items WHERE type_code = ? AND enabled = 1 \
         ORDER BY sort_order, item_value",
    )
    .bind(&code)
    .fetch_all(&state.db)
    .await?;

    let items: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.try_get::<String, _>("id").unwrap_or_default(),
                "value": r.try_get::<String, _>("item_value").unwrap_or_default(),
                "label": r.try_get::<String, _>("item_label").unwrap_or_default(),
                "sortOrder": r.try_get::<i64, _>("sort_order").unwrap_or(0),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "code": 0, "data": items })))
}

/// POST /api/dict/types/:code/items — 新建字典项
async fn create_dict_item(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    Path(code): Path<String>,
    Json(req): Json<CreateItemRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "dict:create")?;
    crate::license_routes::require_active_license(&state.db).await?;

    if req.item_value.trim().is_empty() || req.item_label.trim().is_empty() {
        return Err(AppError::bad("字典项值和标签不能为空"));
    }

    // 验证类型存在
    let exists = sqlx::query("SELECT 1 FROM sys_dict_types WHERE code = ?")
        .bind(&code)
        .fetch_optional(&state.db)
        .await?;
    if exists.is_none() {
        return Err(AppError::not_found("字典类型不存在"));
    }

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let result = sqlx::query(
        "INSERT INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at) \
         VALUES (?, ?, ?, ?, 1, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&code)
    .bind(req.item_value.trim())
    .bind(req.item_label.trim())
    .bind(req.sort_order)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await;

    if let Err(sqlx::Error::Database(ref e)) = result {
        if e.message().contains("Duplicate entry") {
            return Err(AppError::bad("字典项值已存在"));
        }
    }
    result?;

    let detail = serde_json::json!({ "typeCode": code, "value": req.item_value, "label": req.item_label });
    let ip = audit::extract_ip(&headers, Some(addr));
    audit::log_async(
        &state.db, &auth, "create_dict_item", "sys_dict_items",
        &id, Some(&detail), &ip, "success",
    ).await;

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": { "id": id }
    })))
}

/// PUT /api/dict/items/:id — 编辑字典项
async fn update_dict_item(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<UpdateItemRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "dict:update")?;
    crate::license_routes::require_active_license(&state.db).await?;

    if req.item_label.trim().is_empty() {
        return Err(AppError::bad("标签不能为空"));
    }

    let now = chrono::Utc::now().to_rfc3339();
    let result = sqlx::query(
        "UPDATE sys_dict_items SET item_label = ?, \
         enabled = COALESCE(?, enabled), sort_order = COALESCE(?, sort_order), updated_at = ? \
         WHERE id = ?",
    )
    .bind(req.item_label.trim())
    .bind(req.enabled.map(|b| b as i64))
    .bind(req.sort_order)
    .bind(&now)
    .bind(&id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("字典项不存在"));
    }

    let detail = serde_json::json!({ "id": id, "label": req.item_label });
    let ip = audit::extract_ip(&headers, Some(addr));
    audit::log_async(
        &state.db, &auth, "update_dict_item", "sys_dict_items",
        &id, Some(&detail), &ip, "success",
    ).await;

    Ok(Json(serde_json::json!({ "code": 0, "data": { "id": id } })))
}

/// DELETE /api/dict/items/:id — 删除字典项
async fn delete_dict_item(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "dict:delete")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let result = sqlx::query("DELETE FROM sys_dict_items WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("字典项不存在"));
    }

    let ip = audit::extract_ip(&headers, Some(addr));
    audit::log_async(
        &state.db, &auth, "delete_dict_item", "sys_dict_items",
        &id, None, &ip, "success",
    ).await;

    Ok(Json(serde_json::json!({ "code": 0, "data": null })))
}
