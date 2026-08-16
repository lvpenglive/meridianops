//! 角色管理 + 权限点路由。
//!
//! 路由：
//!   GET    /api/roles              列出所有角色        (role:read)
//!   POST   /api/roles              创建角色            (role:create)
//!   GET    /api/roles/:id          角色详情            (role:read)
//!   PUT    /api/roles/:id          更新角色            (role:update)
//!   DELETE /api/roles/:id          删除角色            (role:delete, 内置不可删)
//!   GET    /api/roles/:id/permissions  查看角色权限    (role:read)
//!   PUT    /api/roles/:id/permissions  设置角色权限    (role:assign_permission)
//!   GET    /api/permissions        列出所有权限点      (role:read)

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{ConnectInfo, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;

use crate::audit;
use crate::auth;
use crate::db;
use crate::error::AppError;
use crate::routes::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/roles", get(list_roles).post(create_role))
        .route("/api/roles/:id", get(get_role).put(update_role).delete(delete_role))
        .route(
            "/api/roles/:id/permissions",
            get(list_role_permissions).put(set_role_permissions),
        )
        .route("/api/permissions", get(list_permissions))
}

// ---- 请求体 ----

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRoleRequest {
    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRoleRequest {
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetPermissionsRequest {
    pub permission_ids: Vec<String>,
}

// ---- Handlers ----

/// 列出所有角色。
async fn list_roles(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "role:read")?;
    crate::license_routes::require_active_license(&state.db).await?;
    let roles = db::list_roles(&state.db).await?;
    Ok(Json(serde_json::json!({ "code": 0, "data": roles })))
}

/// 创建角色。name 唯一，不可与内置角色重名。
async fn create_role(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Json(req): Json<CreateRoleRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    auth::require_permission(&auth, "role:create")?;
    crate::license_routes::require_active_license(&state.db).await?;
    let ip = audit::extract_ip(&headers, Some(addr));

    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::bad("角色名不能为空"));
    }
    // 内置角色名占用检查
    if matches!(name.as_str(), "admin" | "operator" | "viewer") {
        return Err(AppError::bad("角色名与内置角色冲突"));
    }
    if db::count_role_by_name(&state.db, &name, None).await? > 0 {
        return Err(AppError {
            status: StatusCode::CONFLICT,
            code: 409,
            message: format!("角色名 '{}' 已存在", name),
        });
    }

    let display_name = req.display_name.unwrap_or_default();
    let description = req.description.unwrap_or_default();
    let enabled = req.enabled.unwrap_or(true);
    let id = db::create_role(&state.db, &name, &display_name, &description, enabled).await?;
    let role = db::find_role_by_id(&state.db, &id)
        .await?
        .ok_or_else(|| AppError::internal("创建后回查失败"))?;

    let detail = serde_json::json!({"name": name, "displayName": display_name});
    let _ = audit::log_async(
        &state.db, &auth, "create", "role", &id, Some(&detail), &ip, "success",
    ).await;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "code": 0, "data": role })),
    ))
}

/// 角色详情。
async fn get_role(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "role:read")?;
    crate::license_routes::require_active_license(&state.db).await?;
    let role = db::find_role_by_id(&state.db, &id)
        .await?
        .ok_or_else(|| AppError::not_found("角色不存在"))?;
    Ok(Json(serde_json::json!({ "code": 0, "data": role })))
}

/// 更新角色可变字段（名称不可改）。
async fn update_role(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateRoleRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "role:update")?;
    crate::license_routes::require_active_license(&state.db).await?;
    let ip = audit::extract_ip(&headers, Some(addr));

    let existing = db::find_role_by_id(&state.db, &id)
        .await?
        .ok_or_else(|| AppError::not_found("角色不存在"))?;
    // 须在 String 字段 move 之前读取 enabled（is_enabled 仅借用 i8）
    let enabled = req.enabled.unwrap_or(existing.is_enabled());
    let display_name = req.display_name.unwrap_or(existing.display_name);
    let description = req.description.unwrap_or(existing.description);

    db::update_role(&state.db, &id, &display_name, &description, enabled).await?;

    let detail = serde_json::json!({"displayName": display_name, "enabled": enabled});
    let _ = audit::log_async(
        &state.db, &auth, "update", "role", &id, Some(&detail), &ip, "success",
    ).await;

    let role = db::find_role_by_id(&state.db, &id)
        .await?
        .ok_or_else(|| AppError::internal("更新后回查失败"))?;
    Ok(Json(serde_json::json!({ "code": 0, "data": role })))
}

/// 删除角色。内置角色不可删，有用户引用时不可删。
async fn delete_role(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "role:delete")?;
    crate::license_routes::require_active_license(&state.db).await?;
    let ip = audit::extract_ip(&headers, Some(addr));

    let role = db::find_role_by_id(&state.db, &id)
        .await?
        .ok_or_else(|| AppError::not_found("角色不存在"))?;
    if role.is_built_in() {
        return Err(AppError::bad("内置角色不可删除"));
    }
    let user_count = db::count_users_by_role(&state.db, &id).await?;
    if user_count > 0 {
        return Err(AppError::bad(&format!("该角色仍有 {} 个用户引用，无法删除", user_count)));
    }

    db::delete_role(&state.db, &id).await?;
    let _ = audit::log_async(
        &state.db, &auth, "delete", "role", &id, None, &ip, "success",
    ).await;

    Ok(Json(serde_json::json!({ "code": 0, "message": "ok" })))
}

/// 查看角色已分配的权限点列表。
async fn list_role_permissions(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "role:read")?;
    crate::license_routes::require_active_license(&state.db).await?;
    let perms = db::list_permissions_by_role(&state.db, &id).await?;
    Ok(Json(serde_json::json!({ "code": 0, "data": perms })))
}

/// 批量设置角色权限（全量覆盖）。
async fn set_role_permissions(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Path(id): Path<String>,
    Json(req): Json<SetPermissionsRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "role:assign_permission")?;
    crate::license_routes::require_active_license(&state.db).await?;
    let ip = audit::extract_ip(&headers, Some(addr));

    // 确认角色存在
    db::find_role_by_id(&state.db, &id)
        .await?
        .ok_or_else(|| AppError::not_found("角色不存在"))?;

    db::set_role_permissions(&state.db, &id, &req.permission_ids).await?;

    let detail = serde_json::json!({"permissionCount": req.permission_ids.len()});
    let _ = audit::log_async(
        &state.db, &auth, "assign_permission", "role", &id, Some(&detail), &ip, "success",
    ).await;

    Ok(Json(serde_json::json!({ "code": 0, "message": "ok" })))
}

/// 列出所有权限点（按模块分组）。
async fn list_permissions(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "role:read")?;
    crate::license_routes::require_active_license(&state.db).await?;
    let perms = db::list_permissions(&state.db).await?;
    Ok(Json(serde_json::json!({ "code": 0, "data": perms })))
}