//! 部门管理路由（树形）。
//!
//! 路由：
//!   GET    /api/departments        列出所有部门（扁平，前端构建树）  (dept:read)
//!   POST   /api/departments        创建部门                          (dept:create)
//!   PUT    /api/departments/:id    更新部门                          (dept:update)
//!   DELETE /api/departments/:id    删除部门                          (dept:delete, 有子部门/用户时不可删)

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
        .route("/api/departments", get(list_departments).post(create_department))
        .route("/api/departments/:id", get(get_department).put(update_department).delete(delete_department))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDepartmentRequest {
    pub name: String,
    pub parent_id: Option<String>,
    pub sort_order: Option<i32>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDepartmentRequest {
    pub name: Option<String>,
    pub parent_id: Option<String>,
    pub sort_order: Option<i32>,
    pub enabled: Option<bool>,
}

/// 列出所有部门（扁平列表，前端按 parent_id 构建树）。
async fn list_departments(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "dept:read")?;
    crate::license_routes::require_active_license(&state.db).await?;
    let depts = db::list_departments(&state.db).await?;
    Ok(Json(serde_json::json!({ "code": 0, "data": depts })))
}

/// 部门详情。
async fn get_department(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "dept:read")?;
    crate::license_routes::require_active_license(&state.db).await?;
    let dept = db::find_department_by_id(&state.db, &id)
        .await?
        .ok_or_else(|| AppError::not_found("部门不存在"))?;
    Ok(Json(serde_json::json!({ "code": 0, "data": dept })))
}

/// 创建部门。parent_id 为空表示根部门。
async fn create_department(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Json(req): Json<CreateDepartmentRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    auth::require_permission(&auth, "dept:create")?;
    crate::license_routes::require_active_license(&state.db).await?;
    let ip = audit::extract_ip(&headers, Some(addr));

    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::bad("部门名不能为空"));
    }
    // parent_id 校验：非空时必须存在
    if let Some(pid) = &req.parent_id {
        if !pid.is_empty() {
            if db::find_department_by_id(&state.db, pid).await?.is_none() {
                return Err(AppError::bad("父部门不存在"));
            }
        }
    }
    // 防止自引用（创建时 id 还没生成，跳过）

    let parent_id = req.parent_id.as_deref().and_then(|s| if s.is_empty() { None } else { Some(s) });
    let sort_order = req.sort_order.unwrap_or(0);
    let enabled = req.enabled.unwrap_or(true);
    let id = db::create_department(&state.db, &name, parent_id, sort_order, enabled).await?;
    let dept = db::find_department_by_id(&state.db, &id)
        .await?
        .ok_or_else(|| AppError::internal("创建后回查失败"))?;

    let detail = serde_json::json!({"name": name, "parentId": parent_id});
    let _ = audit::log_async(
        &state.db, &auth, "create", "department", &id, Some(&detail), &ip, "success",
    ).await;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "code": 0, "data": dept })),
    ))
}

/// 更新部门。parent_id 可改（但不允许设为自己或自己的子孙，防环）。
async fn update_department(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateDepartmentRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "dept:update")?;
    crate::license_routes::require_active_license(&state.db).await?;
    let ip = audit::extract_ip(&headers, Some(addr));

    let existing = db::find_department_by_id(&state.db, &id)
        .await?
        .ok_or_else(|| AppError::not_found("部门不存在"))?;

    let sort_order = req.sort_order.unwrap_or(existing.sort_order);
    let enabled = req.enabled.unwrap_or(existing.is_enabled());

    // parent_id 处理：提供了就用新的（空串表示根部门），None 保留原值
    // 须在 name move 之前读取 existing.parent_id
    let parent_id: Option<&str> = match &req.parent_id {
        Some(pid) => {
            if pid.is_empty() {
                None // 改为根部门
            } else if pid == &id {
                return Err(AppError::bad("不能将部门的父级设为自己"));
            } else {
                if db::find_department_by_id(&state.db, pid).await?.is_none() {
                    return Err(AppError::bad("父部门不存在"));
                }
                Some(pid.as_str())
            }
        }
        None => existing.parent_id.as_deref(),
    };
    let name = req.name.unwrap_or(existing.name);

    db::update_department(&state.db, &id, &name, parent_id, sort_order, enabled).await?;

    let detail = serde_json::json!({"name": name, "enabled": enabled});
    let _ = audit::log_async(
        &state.db, &auth, "update", "department", &id, Some(&detail), &ip, "success",
    ).await;

    let dept = db::find_department_by_id(&state.db, &id)
        .await?
        .ok_or_else(|| AppError::internal("更新后回查失败"))?;
    Ok(Json(serde_json::json!({ "code": 0, "data": dept })))
}

/// 删除部门。有子部门或用户引用时不可删。
async fn delete_department(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "dept:delete")?;
    crate::license_routes::require_active_license(&state.db).await?;
    let ip = audit::extract_ip(&headers, Some(addr));

    db::find_department_by_id(&state.db, &id)
        .await?
        .ok_or_else(|| AppError::not_found("部门不存在"))?;

    let child_count = db::count_departments_by_parent(&state.db, Some(&id)).await?;
    if child_count > 0 {
        return Err(AppError::bad(&format!("该部门下有 {} 个子部门，无法删除", child_count)));
    }
    let user_count = db::count_users_by_department(&state.db, &id).await?;
    if user_count > 0 {
        return Err(AppError::bad(&format!("该部门下有 {} 个用户，无法删除", user_count)));
    }

    db::delete_department(&state.db, &id).await?;
    let _ = audit::log_async(
        &state.db, &auth, "delete", "department", &id, None, &ip, "success",
    ).await;

    Ok(Json(serde_json::json!({ "code": 0, "message": "ok" })))
}