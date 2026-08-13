//! CMDB 配置管理路由。
//!
//! 路由：
//!   GET    /api/cmdb/models                        列出所有 CI 模型            (asset:read)
//!   GET    /api/cmdb/models/:id                    获取模型 + 属性定义         (asset:read)
//!   GET    /api/cmdb/instances                     分页查询 CI 实例            (asset:read)
//!   GET    /api/cmdb/instances/:id                 获取实例详情                (asset:read)
//!   POST   /api/cmdb/instances                     创建 CI 实例               (asset:create)
//!   PUT    /api/cmdb/instances/:id                 更新 CI 实例               (asset:update)
//!   DELETE /api/cmdb/instances/:id                 删除 CI 实例               (asset:delete)
//!   GET    /api/cmdb/instances/:id/relations       查询实例关系               (asset:read)
//!   POST   /api/cmdb/relations                     创建 CI 关系               (asset:update)
//!   DELETE /api/cmdb/relations/:id                 删除 CI 关系               (asset:update)
//!   GET    /api/cmdb/stats                         各模型实例数统计            (asset:read)
//!   GET    /api/cmdb/sync/sources                  列出同步数据源              (asset:read)
//!   POST   /api/cmdb/sync/sources                  新增同步数据源              (system:update)
//!   POST   /api/cmdb/sync                          批量同步（蓝鲸 webhook 推送）(asset:create)
//!   POST   /api/cmdb/sync/pull                     手动拉取（从外部 API 拉取）  (asset:create)
//!   PUT    /api/cmdb/sync/sources/:code            更新数据源拉取配置           (system:update)
//!   DELETE /api/cmdb/sync/sources/:code            删除数据源                  (system:update)
//!   GET    /api/cmdb/sync/logs                     查询同步日志                (asset:read)

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{delete, get, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::audit;
use crate::auth;
use crate::db;
use crate::error::AppError;
use crate::routes::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/cmdb/models", get(list_models).post(create_model))
        .route("/api/cmdb/models/:id", get(get_model).put(update_model).delete(delete_model))
        .route("/api/cmdb/models/:id/attrs", get(list_model_attrs).post(create_model_attr))
        .route(
            "/api/cmdb/models/:model_id/attrs/:attr_id",
            put(update_model_attr).delete(delete_model_attr),
        )
        .route("/api/cmdb/instances", get(list_instances).post(create_instance))
        .route("/api/cmdb/instances/batch", axum::routing::post(batch_create_instances))
        .route(
            "/api/cmdb/instances/:id",
            get(get_instance).put(update_instance).delete(delete_instance),
        )
        .route("/api/cmdb/instances/:id/relations", get(list_relations))
        .route("/api/cmdb/relations", get(list_all_relations).post(create_relation))
        .route("/api/cmdb/relations/:id", delete(delete_relation))
        .route("/api/cmdb/relation-types", get(list_relation_types).post(create_relation_type))
        .route("/api/cmdb/relation-types/:id", put(update_relation_type).delete(delete_relation_type))
        .route("/api/cmdb/stats", get(cmdb_stats))
        .route("/api/cmdb/topology", get(topology))
        // 同步相关
        .route("/api/cmdb/sync/sources", get(list_sync_sources).post(create_sync_source))
        .route("/api/cmdb/sync", axum::routing::post(sync_instances))
        .route("/api/cmdb/sync/pull", axum::routing::post(pull_instances))
        .route("/api/cmdb/sync/sources/:code", axum::routing::put(update_sync_source).delete(delete_sync_source))
        .route("/api/cmdb/sync/logs", get(list_sync_logs))
}

// ---- CI 模型 ----

async fn list_models(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "asset:read")?;
    let models = db::list_ci_models(&state.db).await?;
    Ok(Json(serde_json::json!({ "code": 0, "data": models })))
}

/// 获取模型详情 + 属性定义列表。
async fn get_model(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "asset:read")?;
    let model = db::find_ci_model_by_id(&state.db, &id)
        .await?
        .ok_or_else(|| AppError::not_found("CI 模型不存在"))?;
    let attrs = db::list_ci_model_attrs(&state.db, &id).await?;
    Ok(Json(serde_json::json!({
        "code": 0,
        "data": { "model": model, "attributes": attrs }
    })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateModelRequest {
    code: String,
    name: String,
    icon: Option<String>,
    description: Option<String>,
    enabled: Option<bool>,
    sort_order: Option<i32>,
}

/// 创建 CI 模型（动态建模）。
async fn create_model(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Json(req): Json<CreateModelRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    auth::require_permission(&auth, "system:update")?;
    let ip = audit::extract_ip(&headers, Some(addr));

    let code = req.code.trim().to_string();
    let name = req.name.trim().to_string();
    if code.is_empty() || name.is_empty() {
        return Err(AppError::bad("code 和 name 不能为空"));
    }
    if !code.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
        return Err(AppError::bad("code 只能包含小写字母、数字、下划线"));
    }
    if code.len() > 64 {
        return Err(AppError::bad("code 长度不能超过 64"));
    }
    if db::find_ci_model_by_code(&state.db, &code).await?.is_some() {
        return Err(AppError::bad(&format!("模型编码 {} 已存在", code)));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let icon = req.icon.unwrap_or_else(|| "Monitor".to_string());
    let description = req.description.unwrap_or_default();
    let enabled = req.enabled.unwrap_or(true);
    let sort_order = req.sort_order.unwrap_or(99);

    db::create_ci_model(&state.db, &id, &code, &name, &icon, &description, enabled, sort_order).await?;

    let detail = serde_json::json!({ "id": id, "code": code, "name": name });
    let _ = audit::log_async(
        &state.db, &auth, "create_ci_model", "ci_model", &id, Some(&detail), &ip, "success",
    ).await;

    let model = db::find_ci_model_by_id(&state.db, &id)
        .await?
        .ok_or_else(|| AppError::internal("创建后回查失败"))?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "code": 0, "data": model }))))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateModelRequest {
    name: String,
    icon: Option<String>,
    description: Option<String>,
    enabled: Option<bool>,
    sort_order: Option<i32>,
}

/// 更新 CI 模型（code 不可改）。
async fn update_model(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateModelRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "system:update")?;
    let ip = audit::extract_ip(&headers, Some(addr));

    let existing = db::find_ci_model_by_id(&state.db, &id)
        .await?
        .ok_or_else(|| AppError::not_found("CI 模型不存在"))?;
    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::bad("name 不能为空"));
    }
    let icon = req.icon.unwrap_or(existing.icon);
    let description = req.description.unwrap_or(existing.description);
    let enabled = req.enabled.unwrap_or(existing.enabled != 0);
    let sort_order = req.sort_order.unwrap_or(existing.sort_order);

    db::update_ci_model(&state.db, &id, &name, &icon, &description, enabled, sort_order).await?;

    let detail = serde_json::json!({ "name": name, "enabled": enabled });
    let _ = audit::log_async(
        &state.db, &auth, "update_ci_model", "ci_model", &id, Some(&detail), &ip, "success",
    ).await;

    let model = db::find_ci_model_by_id(&state.db, &id)
        .await?
        .ok_or_else(|| AppError::internal("更新后回查失败"))?;
    Ok(Json(serde_json::json!({ "code": 0, "data": model })))
}

/// 删除 CI 模型（有实例时拒绝）。
async fn delete_model(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "system:update")?;
    let ip = audit::extract_ip(&headers, Some(addr));

    let existing = db::find_ci_model_by_id(&state.db, &id)
        .await?
        .ok_or_else(|| AppError::not_found("CI 模型不存在"))?;

    let count = db::count_ci_instances_by_model_id(&state.db, &id).await?;
    if count > 0 {
        return Err(AppError::bad(&format!(
            "模型 {} 下还有 {} 个实例，请先迁移或清理后再删除", existing.code, count
        )));
    }

    db::delete_ci_model(&state.db, &id).await?;

    let detail = serde_json::json!({ "code": existing.code, "name": existing.name });
    let _ = audit::log_async(
        &state.db, &auth, "delete_ci_model", "ci_model", &id, Some(&detail), &ip, "success",
    ).await;

    Ok(Json(serde_json::json!({ "code": 0, "message": "ok" })))
}

/// 列出某模型的属性定义。
async fn list_model_attrs(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "asset:read")?;
    let attrs = db::list_ci_model_attrs(&state.db, &id).await?;
    Ok(Json(serde_json::json!({ "code": 0, "data": attrs })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateModelAttrRequest {
    code: String,
    name: String,
    value_type: Option<String>,
    default_value: Option<String>,
    /// 枚举选项（value_type=enum 时使用），JSON 字符串或数组
    options: Option<serde_json::Value>,
    is_required: Option<bool>,
    is_unique: Option<bool>,
    is_searchable: Option<bool>,
    sort_order: Option<i32>,
}

/// 创建模型属性。
async fn create_model_attr(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Path(model_id): Path<String>,
    Json(req): Json<CreateModelAttrRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    auth::require_permission(&auth, "system:update")?;
    let ip = audit::extract_ip(&headers, Some(addr));

    // 校验模型存在
    db::find_ci_model_by_id(&state.db, &model_id)
        .await?
        .ok_or_else(|| AppError::not_found("CI 模型不存在"))?;

    let code = req.code.trim().to_string();
    let name = req.name.trim().to_string();
    if code.is_empty() || name.is_empty() {
        return Err(AppError::bad("code 和 name 不能为空"));
    }
    if !code.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
        return Err(AppError::bad("code 只能包含小写字母、数字、下划线"));
    }
    if db::find_ci_model_attr_by_code(&state.db, &model_id, &code).await?.is_some() {
        return Err(AppError::bad(&format!("属性编码 {} 已存在", code)));
    }

    let value_type = req.value_type.unwrap_or_else(|| "string".to_string());
    let default_value = req.default_value.unwrap_or_default();
    let options_str = req.options.as_ref().map(|v| v.to_string());
    let is_required = req.is_required.unwrap_or(false);
    let is_unique = req.is_unique.unwrap_or(false);
    let is_searchable = req.is_searchable.unwrap_or(true);
    let sort_order = req.sort_order.unwrap_or(99);

    let attr_id = uuid::Uuid::new_v4().to_string();
    db::create_ci_model_attr(
        &state.db, &attr_id, &model_id, &code, &name, &value_type,
        &default_value, options_str.as_deref(),
        is_required, is_unique, is_searchable, sort_order,
    ).await?;

    let detail = serde_json::json!({ "modelId": model_id, "code": code, "name": name });
    let _ = audit::log_async(
        &state.db, &auth, "create_ci_model_attr", "ci_model_attr", &attr_id, Some(&detail), &ip, "success",
    ).await;

    let attr = db::find_ci_model_attr(&state.db, &attr_id)
        .await?
        .ok_or_else(|| AppError::internal("创建后回查失败"))?;
    Ok((StatusCode::CREATED, Json(serde_json::json!({ "code": 0, "data": attr }))))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateModelAttrRequest {
    name: String,
    value_type: Option<String>,
    default_value: Option<String>,
    options: Option<serde_json::Value>,
    is_required: Option<bool>,
    is_unique: Option<bool>,
    is_searchable: Option<bool>,
    sort_order: Option<i32>,
}

/// 更新模型属性（code 不可改）。
async fn update_model_attr(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Path((model_id, attr_id)): Path<(String, String)>,
    Json(req): Json<UpdateModelAttrRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "system:update")?;
    let ip = audit::extract_ip(&headers, Some(addr));

    let existing = db::find_ci_model_attr(&state.db, &attr_id)
        .await?
        .ok_or_else(|| AppError::not_found("模型属性不存在"))?;
    if existing.model_id != model_id {
        return Err(AppError::bad("属性不属于该模型"));
    }

    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::bad("name 不能为空"));
    }
    let value_type = req.value_type.unwrap_or(existing.value_type);
    let default_value = req.default_value.unwrap_or(existing.default_value);
    let options_str = req.options.as_ref().map(|v| v.to_string());
    let is_required = req.is_required.unwrap_or(existing.is_required != 0);
    let is_unique = req.is_unique.unwrap_or(existing.is_unique != 0);
    let is_searchable = req.is_searchable.unwrap_or(existing.is_searchable != 0);
    let sort_order = req.sort_order.unwrap_or(existing.sort_order);

    db::update_ci_model_attr(
        &state.db, &attr_id, &name, &value_type, &default_value,
        options_str.as_deref(), is_required, is_unique, is_searchable, sort_order,
    ).await?;

    let detail = serde_json::json!({ "modelId": model_id, "name": name });
    let _ = audit::log_async(
        &state.db, &auth, "update_ci_model_attr", "ci_model_attr", &attr_id, Some(&detail), &ip, "success",
    ).await;

    let attr = db::find_ci_model_attr(&state.db, &attr_id)
        .await?
        .ok_or_else(|| AppError::internal("更新后回查失败"))?;
    Ok(Json(serde_json::json!({ "code": 0, "data": attr })))
}

/// 删除模型属性。
async fn delete_model_attr(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Path((model_id, attr_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "system:update")?;
    let ip = audit::extract_ip(&headers, Some(addr));

    let existing = db::find_ci_model_attr(&state.db, &attr_id)
        .await?
        .ok_or_else(|| AppError::not_found("模型属性不存在"))?;
    if existing.model_id != model_id {
        return Err(AppError::bad("属性不属于该模型"));
    }

    db::delete_ci_model_attr(&state.db, &attr_id).await?;

    let detail = serde_json::json!({ "modelId": model_id, "code": existing.code });
    let _ = audit::log_async(
        &state.db, &auth, "delete_ci_model_attr", "ci_model_attr", &attr_id, Some(&detail), &ip, "success",
    ).await;

    Ok(Json(serde_json::json!({ "code": 0, "message": "ok" })))
}

// ---- 拓扑视图 ----

#[derive(Debug, Deserialize)]
struct TopologyQuery {
    #[serde(rename = "modelId")]
    model_id: Option<String>,
    status: Option<String>,
}

/// 查询拓扑：返回 nodes + links。
async fn topology(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Query(q): Query<TopologyQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "asset:read")?;
    let model_id = q.model_id.as_deref().and_then(|s| if s.is_empty() { None } else { Some(s) });
    let status = q.status.as_deref().and_then(|s| if s.is_empty() { None } else { Some(s) });

    let (nodes, links) = db::query_topology(&state.db, model_id, status).await?;

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": {
            "nodes": nodes,
            "links": links,
            "nodeCount": nodes.len(),
            "linkCount": links.len(),
        }
    })))
}

// ---- CI 实例 ----

#[derive(Debug, Deserialize)]
struct InstanceQuery {
    #[serde(rename = "modelId")]
    model_id: Option<String>,
    status: Option<String>,
    keyword: Option<String>,
    #[serde(rename = "departmentId")]
    department_id: Option<String>,
    page: Option<u64>,
    #[serde(rename = "pageSize")]
    page_size: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateInstanceRequest {
    model_id: String,
    name: String,
    status: Option<String>,
    department_id: Option<String>,
    owner_id: Option<String>,
    attributes: Option<serde_json::Value>,
    tags: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateInstanceRequest {
    name: String,
    status: Option<String>,
    department_id: Option<String>,
    owner_id: Option<String>,
    attributes: Option<serde_json::Value>,
    tags: Option<String>,
}

async fn list_instances(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Query(q): Query<InstanceQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "asset:read")?;
    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(20).clamp(1, 200);

    let model_id = q.model_id.as_deref().and_then(|s| if s.is_empty() { None } else { Some(s) });
    let status = q.status.as_deref().and_then(|s| if s.is_empty() { None } else { Some(s) });
    let keyword = q.keyword.as_deref().and_then(|s| if s.is_empty() { None } else { Some(s) });
    let department_id = q.department_id.as_deref().and_then(|s| if s.is_empty() { None } else { Some(s) });

    let (total, items) = db::query_ci_instances(
        &state.db, model_id, status, keyword, department_id, page, page_size,
    )
    .await?;

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": {
            "total": total,
            "page": page,
            "pageSize": page_size,
            "items": items,
        }
    })))
}

async fn get_instance(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "asset:read")?;
    let inst = db::find_ci_instance_by_id(&state.db, &id)
        .await?
        .ok_or_else(|| AppError::not_found("资产不存在"))?;
    Ok(Json(serde_json::json!({ "code": 0, "data": inst })))
}

async fn create_instance(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Json(req): Json<CreateInstanceRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    auth::require_permission(&auth, "asset:create")?;
    let ip = audit::extract_ip(&headers, Some(addr));

    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::bad("资产名称不能为空"));
    }
    // 校验模型存在
    db::find_ci_model_by_id(&state.db, &req.model_id)
        .await?
        .ok_or_else(|| AppError::bad("CI 模型不存在"))?;

    let status = req.status.unwrap_or_else(|| "running".to_string());
    let dept = req.department_id.as_deref().and_then(|s| if s.is_empty() { None } else { Some(s) });
    let owner = req.owner_id.as_deref().and_then(|s| if s.is_empty() { None } else { Some(s) });
    let attrs_json = req
        .attributes
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "{}".to_string());
    let tags = req.tags.unwrap_or_default();

    let id = db::create_ci_instance(
        &state.db, &req.model_id, &name, &status, dept, owner, &attrs_json, &tags,
    )
    .await?;

    let detail = serde_json::json!({
        "modelId": req.model_id,
        "name": name,
        "status": status,
    });
    let _ = audit::log_async(
        &state.db, &auth, "create_ci", "ci_instance", &id, Some(&detail), &ip, "success",
    )
    .await;

    let inst = db::find_ci_instance_by_id(&state.db, &id)
        .await?
        .ok_or_else(|| AppError::internal("创建后回查失败"))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "code": 0, "data": inst })),
    ))
}

async fn update_instance(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateInstanceRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "asset:update")?;
    let ip = audit::extract_ip(&headers, Some(addr));

    let existing = db::find_ci_instance_by_id(&state.db, &id)
        .await?
        .ok_or_else(|| AppError::not_found("资产不存在"))?;

    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::bad("资产名称不能为空"));
    }
    let status = req.status.unwrap_or(existing.status);
    let dept = req.department_id.as_deref().and_then(|s| if s.is_empty() { None } else { Some(s) });
    let owner = req.owner_id.as_deref().and_then(|s| if s.is_empty() { None } else { Some(s) });
    let attrs_json = req
        .attributes
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| existing.attributes.unwrap_or_else(|| "{}".to_string()));
    let tags = req.tags.unwrap_or(existing.tags);

    db::update_ci_instance(
        &state.db, &id, &name, &status, dept, owner, &attrs_json, &tags,
    )
    .await?;

    let detail = serde_json::json!({ "name": name, "status": status });
    let _ = audit::log_async(
        &state.db, &auth, "update_ci", "ci_instance", &id, Some(&detail), &ip, "success",
    )
    .await;

    let inst = db::find_ci_instance_by_id(&state.db, &id)
        .await?
        .ok_or_else(|| AppError::internal("更新后回查失败"))?;
    Ok(Json(serde_json::json!({ "code": 0, "data": inst })))
}

async fn delete_instance(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "asset:delete")?;
    let ip = audit::extract_ip(&headers, Some(addr));

    let existing = db::find_ci_instance_by_id(&state.db, &id)
        .await?
        .ok_or_else(|| AppError::not_found("资产不存在"))?;

    db::delete_ci_instance(&state.db, &id).await?;

    let detail = serde_json::json!({ "name": existing.name, "modelId": existing.model_id });
    let _ = audit::log_async(
        &state.db, &auth, "delete_ci", "ci_instance", &id, Some(&detail), &ip, "success",
    )
    .await;

    Ok(Json(serde_json::json!({ "code": 0, "message": "ok" })))
}

// ---- CI 关系 ----

#[derive(Debug, Deserialize)]
struct RelationQuery {
    #[serde(rename = "ciId")]
    ci_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateRelationRequest {
    source_id: String,
    target_id: String,
    relation_type: String,
}

/// 查询某 CI 实例的关系。无 ciId 参数时返回空（需指定）。
async fn list_relations(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "asset:read")?;
    let rels = db::list_ci_relations(&state.db, &id).await?;
    Ok(Json(serde_json::json!({ "code": 0, "data": rels })))
}

/// 查询全部关系（或按 ciId 筛选）。
async fn list_all_relations(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Query(q): Query<RelationQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "asset:read")?;
    let ci_id = q.ci_id.as_deref().and_then(|s| if s.is_empty() { None } else { Some(s) });
    let rels = match ci_id {
        Some(cid) => db::list_ci_relations(&state.db, cid).await?,
        None => {
            // 无 ciId 时返回空列表（避免全表扫描）
            Vec::new()
        }
    };
    Ok(Json(serde_json::json!({ "code": 0, "data": rels })))
}

async fn create_relation(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Json(req): Json<CreateRelationRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    auth::require_permission(&auth, "asset:update")?;
    let ip = audit::extract_ip(&headers, Some(addr));

    // 校验两端实例存在
    db::find_ci_instance_by_id(&state.db, &req.source_id)
        .await?
        .ok_or_else(|| AppError::bad("源 CI 实例不存在"))?;
    db::find_ci_instance_by_id(&state.db, &req.target_id)
        .await?
        .ok_or_else(|| AppError::bad("目标 CI 实例不存在"))?;
    if req.source_id == req.target_id {
        return Err(AppError::bad("不能与自己建立关系"));
    }

    let id = db::create_ci_relation(&state.db, &req.source_id, &req.target_id, &req.relation_type)
        .await?;

    let detail = serde_json::json!({
        "sourceId": req.source_id,
        "targetId": req.target_id,
        "relationType": req.relation_type,
    });
    let _ = audit::log_async(
        &state.db, &auth, "create_ci_relation", "ci_relation", &id, Some(&detail), &ip, "success",
    )
    .await;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "code": 0, "data": { "id": id } })),
    ))
}

async fn delete_relation(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "asset:update")?;
    let ip = audit::extract_ip(&headers, Some(addr));

    db::delete_ci_relation(&state.db, &id).await?;

    let _ = audit::log_async(
        &state.db, &auth, "delete_ci_relation", "ci_relation", &id, None, &ip, "success",
    )
    .await;

    Ok(Json(serde_json::json!({ "code": 0, "message": "ok" })))
}

// ---- CI 关系类型 ----

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateRelationTypeRequest {
    code: String,
    name: String,
    description: Option<String>,
    directional: Option<bool>,
    enabled: Option<bool>,
    sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateRelationTypeRequest {
    name: String,
    description: Option<String>,
    directional: Option<bool>,
    enabled: Option<bool>,
    sort_order: Option<i32>,
}

/// 查询所有关系类型（asset:read）。
async fn list_relation_types(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "asset:read")?;
    let types = db::list_ci_relation_types(&state.db).await?;
    Ok(Json(serde_json::json!({ "code": 0, "data": types })))
}

/// 创建关系类型（system:update）。
async fn create_relation_type(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Json(req): Json<CreateRelationTypeRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    auth::require_permission(&auth, "system:update")?;
    let ip = audit::extract_ip(&headers, Some(addr));

    // code 格式校验
    let code = req.code.trim().to_lowercase();
    if code.len() < 2 || code.len() > 32 || !code.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(AppError::bad("关系类型 code 只能包含字母、数字、下划线，长度 2-32"));
    }

    let id = format!("reltype-{}", code);
    db::create_ci_relation_type(
        &state.db,
        &id,
        &code,
        &req.name,
        &req.description.unwrap_or_default(),
        req.directional.unwrap_or(true),
        req.enabled.unwrap_or(true),
        req.sort_order.unwrap_or(0),
    )
    .await
    .map_err(|e| {
        if e.to_string().contains("Duplicate") {
            AppError::bad(&format!("关系类型 code '{}' 已存在", code))
        } else {
            AppError::internal(e.to_string())
        }
    })?;

    let detail = serde_json::json!({ "code": code, "name": req.name });
    let _ = audit::log_async(
        &state.db, &auth, "create_ci_relation_type", "ci_relation_type", &id, Some(&detail), &ip, "success",
    )
    .await;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "code": 0, "data": { "id": id } })),
    ))
}

/// 更新关系类型（system:update）。
async fn update_relation_type(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateRelationTypeRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "system:update")?;
    let ip = audit::extract_ip(&headers, Some(addr));

    db::update_ci_relation_type(
        &state.db,
        &id,
        &req.name,
        &req.description.unwrap_or_default(),
        req.directional.unwrap_or(true),
        req.enabled.unwrap_or(true),
        req.sort_order.unwrap_or(0),
    )
    .await?;

    let detail = serde_json::json!({ "name": req.name });
    let _ = audit::log_async(
        &state.db, &auth, "update_ci_relation_type", "ci_relation_type", &id, Some(&detail), &ip, "success",
    )
    .await;

    Ok(Json(serde_json::json!({ "code": 0, "message": "ok" })))
}

/// 删除关系类型（system:update）。有关联关系时拒绝。
async fn delete_relation_type(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "system:update")?;
    let ip = audit::extract_ip(&headers, Some(addr));

    let deleted = db::delete_ci_relation_type(&state.db, &id).await?;
    if !deleted {
        return Err(AppError::not_found("关系类型不存在"));
    }

    let _ = audit::log_async(
        &state.db, &auth, "delete_ci_relation_type", "ci_relation_type", &id, None, &ip, "success",
    )
    .await;

    Ok(Json(serde_json::json!({ "code": 0, "message": "ok" })))
}

// ---- 统计 ----

async fn cmdb_stats(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "asset:read")?;
    let models = db::list_ci_models(&state.db).await?;
    let total = db::count_ci_instances_total(&state.db).await?;

    let mut by_model = Vec::new();
    for m in &models {
        let count = db::count_ci_instances_by_model(&state.db, &m.id).await?;
        by_model.push(serde_json::json!({
            "modelId": m.id,
            "modelCode": m.code,
            "modelName": m.name,
            "icon": m.icon,
            "count": count,
        }));
    }

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": {
            "total": total,
            "modelCount": models.len(),
            "byModel": by_model,
        }
    })))
}

// ============ 同步：外部 CMDB（蓝鲸）数据接入 ============

/// 列出所有同步数据源。
async fn list_sync_sources(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "asset:read")?;
    let sources = db::list_sync_sources(&state.db).await?;
    Ok(Json(serde_json::json!({ "code": 0, "data": sources })))
}

/// 同步日志查询参数。
#[derive(Debug, Deserialize)]
struct SyncLogQuery {
    #[serde(rename = "sourceCode")]
    source_code: Option<String>,
    #[serde(rename = "batchId")]
    batch_id: Option<String>,
    status: Option<String>,
    #[serde(rename = "instanceId")]
    instance_id: Option<String>,
    page: Option<u64>,
    #[serde(rename = "pageSize")]
    page_size: Option<u64>,
}

/// 查询同步日志。
async fn list_sync_logs(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Query(q): Query<SyncLogQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "asset:read")?;
    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(20).clamp(1, 200);

    let source_code = q.source_code.as_deref().and_then(|s| if s.is_empty() { None } else { Some(s) });
    let batch_id = q.batch_id.as_deref().and_then(|s| if s.is_empty() { None } else { Some(s) });
    let status = q.status.as_deref().and_then(|s| if s.is_empty() { None } else { Some(s) });
    let instance_id = q.instance_id.as_deref().and_then(|s| if s.is_empty() { None } else { Some(s) });

    let (total, items) = db::query_sync_logs(&state.db, source_code, batch_id, status, instance_id, page, page_size).await?;

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": {
            "total": total,
            "page": page,
            "pageSize": page_size,
            "items": items,
        }
    })))
}

/// 同步请求体。
///
/// 蓝鲸 webhook 推送格式（示例）：
/// ```json
/// {
///   "source": "blueking",
///   "modelCode": "host",
///   "items": [
///     { "bk_host_id": 1001, "bk_host_name": "web-01", "bk_host_innerip": "10.0.0.1", ... }
///   ]
/// }
/// ```
///
/// 通用格式（任意外部系统）：
/// ```json
/// {
///   "source": "external_cmdb",
///   "modelCode": "host",
///   "items": [
///     { "external_id": "xxx", "name": "web-01", "attributes": {...}, "status": "running" }
///   ]
/// }
/// ```
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncRequest {
    /// 数据来源编码（如 blueking）
    source: String,
    /// CI 模型编码（如 host）
    model_code: String,
    /// 待同步的数据项（每项一条 CI 实例）
    items: Vec<serde_json::Value>,
}

/// 批量同步入口。
///
/// 工作流程：
/// 1. 校验 source 已在 sync_sources 表中注册且 enabled
/// 2. 校验 model_code 对应的 CI 模型存在
/// 3. 遍历 items，按 source 不同走不同的字段映射（蓝鲸走 map_blueking_host）
/// 4. 调 upsert_ci_instance 幂等写入
/// 5. 每条都写 sync_logs 明细
/// 6. 更新 sync_sources 的最近同步状态
async fn sync_instances(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Json(req): Json<SyncRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "asset:create")?;
    let ip = audit::extract_ip(&headers, Some(addr));

    // 1. 校验数据源
    let src = db::find_sync_source_by_code(&state.db, &req.source)
        .await?
        .ok_or_else(|| AppError::bad(&format!("数据源 {} 未注册", req.source)))?;
    if !src.is_enabled() {
        return Err(AppError::bad(&format!("数据源 {} 已禁用", req.source)));
    }

    // 2. 校验模型编码：根据 model_code 找到 model_id
    let models = db::list_ci_models(&state.db).await?;
    let model = models
        .iter()
        .find(|m| m.code == req.model_code)
        .ok_or_else(|| AppError::bad(&format!("CI 模型编码 {} 不存在", req.model_code)))?;
    let model_id = model.id.clone();

    // 3. 生成批次 ID
    let batch_id = uuid::Uuid::new_v4().to_string();
    let total = req.items.len();
    let mut success_count = 0i32;
    let mut failed_count = 0i32;
    let mut skipped_count = 0i32;

    // 4. 逐条处理
    for item in &req.items {
        // 根据 source 走不同映射
        let mapped = if req.source == "blueking" {
            db::map_blueking_host(item)
        } else {
            // 通用格式：直接读取标准字段
            map_generic_instance(item, &req.model_code)
        };

        let mapped = match mapped {
            Some(m) => m,
            None => {
                failed_count += 1;
                let _ = db::insert_sync_log(
                    &state.db, &req.source, &batch_id, "upsert", &req.model_code,
                    "", None, "", "failed", "字段映射失败：缺少必填字段",
                    Some(item),
                ).await;
                continue;
            }
        };

        let attrs_json = mapped.attributes.to_string();
        let action = "upsert";

        match db::upsert_ci_instance(
            &state.db, &model_id, &mapped.name, &mapped.status,
            &attrs_json, &req.source, &mapped.external_id,
        ).await {
            Ok((instance_id, is_new)) => {
                success_count += 1;
                let _ = db::insert_sync_log(
                    &state.db, &req.source, &batch_id, action, &req.model_code,
                    &mapped.external_id, Some(&instance_id), &mapped.name,
                    "success", if is_new { "created" } else { "updated" },
                    Some(item),
                ).await;
            }
            Err(e) => {
                failed_count += 1;
                let _ = db::insert_sync_log(
                    &state.db, &req.source, &batch_id, action, &req.model_code,
                    &mapped.external_id, None, &mapped.name,
                    "failed", &format!("入库失败: {}", e),
                    Some(item),
                ).await;
            }
        }
    }

    let _ = skipped_count; // 占位，后续可扩展跳过逻辑

    // 5. 更新数据源同步状态
    let sync_status = if failed_count == 0 { "success" } else if success_count == 0 { "failed" } else { "partial" };
    let _ = db::update_sync_source_status(&state.db, &req.source, success_count, sync_status).await;

    // 6. 审计记录（同步批次级别）
    let detail = serde_json::json!({
        "source": req.source,
        "modelCode": req.model_code,
        "batchId": batch_id,
        "total": total,
        "success": success_count,
        "failed": failed_count,
    });
    let _ = audit::log_async(
        &state.db, &auth, "sync_ci", "ci_instance", &batch_id, Some(&detail), &ip, sync_status,
    ).await;

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": {
            "batchId": batch_id,
            "total": total,
            "success": success_count,
            "failed": failed_count,
            "status": sync_status,
        }
    })))
}

/// 通用格式字段映射（非蓝鲸系统用）。
/// 期望 item 格式：{ "external_id": "...", "name": "...", "status": "...", "attributes": {...} }
fn map_generic_instance(item: &serde_json::Value, model_code: &str) -> Option<db::MappedInstance> {
    let external_id = item.get("external_id")?.as_str()?.to_string();
    let name = item.get("name")?.as_str()?.to_string();
    let status = item
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("running")
        .to_string();
    let attributes = item.get("attributes").cloned().unwrap_or(serde_json::json!({}));

    Some(db::MappedInstance {
        external_id,
        model_code: model_code.to_string(),
        name,
        status,
        attributes,
    })
}

// ==================== 拉取同步 ====================

/// 拉取请求参数。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PullRequest {
    /// 数据源编码（如 blueking）
    source: String,
    /// CI 模型编码（可选，默认用 pull_config 中的 modelCode）
    model_code: Option<String>,
}

/// 拉取配置（从 pull_config JSON 解析）。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PullConfig {
    /// HTTP 方法：GET / POST（默认 GET）
    #[serde(default = "default_method")]
    method: String,
    /// API 路径（拼接到 api_url 后面）
    #[serde(default)]
    path: String,
    /// 请求参数（GET=query params, POST=body）
    #[serde(default)]
    params: serde_json::Value,
    /// 额外请求头
    #[serde(default)]
    headers: serde_json::Value,
    /// 响应中数据项的 JSON path（如 data.info / data.list）
    #[serde(default = "default_response_path")]
    response_path: String,
    /// 默认 CI 模型编码
    #[serde(default)]
    model_code: String,
}

fn default_method() -> String { "GET".to_string() }
fn default_response_path() -> String { "data".to_string() }

/// 更新数据源拉取配置。
async fn update_sync_source(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Path(code): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "system:update")?;
    let ip = audit::extract_ip(&headers, Some(addr));

    let api_url = req.get("apiUrl").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let api_token = req.get("apiToken").and_then(|v| v.as_str()).unwrap_or("").to_string();
    // pullConfig 可能传 JSON 字符串（前端 JSON.stringify）或 JSON 对象，统一归一化为对象字符串
    let pull_config = match req.get("pullConfig") {
        Some(serde_json::Value::String(s)) => {
            // 前端传字符串：先解析为 Value 再 to_string，避免双重引号
            serde_json::from_str::<serde_json::Value>(s)
                .map(|v| v.to_string())
                .unwrap_or_else(|_| "{}".to_string())
        }
        Some(v) => v.to_string(),
        None => "{}".to_string(),
    };
    let pull_cron = req.get("pullCron").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let pull_enabled = req.get("pullEnabled").and_then(|v| v.as_bool()).unwrap_or(false);

    // 校验 pull_config 是合法 JSON 对象
    if serde_json::from_str::<serde_json::Value>(&pull_config).is_err() {
        return Err(AppError::bad("pullConfig 不是合法 JSON"));
    }

    db::update_sync_source_pull_config(
        &state.db, &code, &api_url, &api_token,
        &pull_config, &pull_cron, pull_enabled,
    ).await?;

    let detail = serde_json::json!({
        "sourceCode": code,
        "apiUrl": api_url,
        "pullCron": pull_cron,
        "pullEnabled": pull_enabled,
    });
    let _ = audit::log_async(
        &state.db, &auth, "update_sync_source", "sync_sources", &code, Some(&detail), &ip, "success",
    ).await;

    Ok(Json(serde_json::json!({ "code": 0, "data": true })))
}

/// 新增同步数据源。
async fn create_sync_source(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "system:update")?;
    let ip = audit::extract_ip(&headers, Some(addr));

    let code = req.get("code").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    let name = req.get("name").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
    if code.is_empty() || name.is_empty() {
        return Err(AppError::bad("code 和 name 不能为空"));
    }
    // code 只允许小写字母/数字/下划线，避免路由或 SQL 异常
    if !code.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
        return Err(AppError::bad("code 只能包含小写字母、数字、下划线"));
    }
    if code.len() > 32 {
        return Err(AppError::bad("code 长度不能超过 32"));
    }
    // 重复 code 校验
    if db::find_sync_source_by_code(&state.db, &code).await?.is_some() {
        return Err(AppError::bad(&format!("数据源 code={} 已存在", code)));
    }

    let api_url = req.get("apiUrl").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let api_token = req.get("apiToken").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let webhook_secret = req.get("webhookSecret").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let source_type = req.get("sourceType").and_then(|v| v.as_str()).unwrap_or("webhook").to_string();
    // 拉取配置归一化
    let pull_config = match req.get("pullConfig") {
        Some(serde_json::Value::String(s)) => serde_json::from_str::<serde_json::Value>(s)
            .map(|v| v.to_string())
            .unwrap_or_else(|_| "{}".to_string()),
        Some(v) => v.to_string(),
        None => "{}".to_string(),
    };
    if serde_json::from_str::<serde_json::Value>(&pull_config).is_err() {
        return Err(AppError::bad("pullConfig 不是合法 JSON"));
    }
    let pull_cron = req.get("pullCron").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let pull_enabled = req.get("pullEnabled").and_then(|v| v.as_bool()).unwrap_or(false);

    let id = uuid::Uuid::new_v4().to_string();
    db::create_sync_source(
        &state.db, &id, &code, &name, &source_type,
        &api_url, &api_token, &webhook_secret,
        &pull_config, &pull_cron, pull_enabled,
    ).await?;

    let detail = serde_json::json!({
        "code": code, "name": name, "sourceType": source_type,
        "pullEnabled": pull_enabled, "pullCron": pull_cron,
    });
    let _ = audit::log_async(
        &state.db, &auth, "create_sync_source", "sync_sources", &code, Some(&detail), &ip, "success",
    ).await;

    Ok(Json(serde_json::json!({ "code": 0, "data": { "id": id, "code": code } })))
}

/// 删除同步数据源。
/// 有 CI 实例或同步日志关联时拒绝删除（需先清理数据）。
async fn delete_sync_source(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Path(code): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "system:update")?;
    let ip = audit::extract_ip(&headers, Some(addr));

    // 存在性校验
    let src = db::find_sync_source_by_code(&state.db, &code)
        .await?
        .ok_or_else(|| AppError::not_found("数据源不存在"))?;

    // 关联数据校验：有 CI 实例或同步日志时拒绝
    let ci_count = db::count_ci_instances_by_source(&state.db, &code).await?;
    if ci_count > 0 {
        return Err(AppError::bad(&format!(
            "数据源 {} 下还有 {} 个 CI 实例，请先迁移或清理后再删除", code, ci_count
        )));
    }
    let log_count = db::count_sync_logs_by_source(&state.db, &code).await?;
    if log_count > 0 {
        return Err(AppError::bad(&format!(
            "数据源 {} 下还有 {} 条同步日志，请先清理后再删除", code, log_count
        )));
    }

    db::delete_sync_source(&state.db, &code).await?;

    let detail = serde_json::json!({ "code": code, "name": src.name });
    let _ = audit::log_async(
        &state.db, &auth, "delete_sync_source", "sync_sources", &code, Some(&detail), &ip, "success",
    ).await;

    Ok(Json(serde_json::json!({ "code": 0, "data": true })))
}

/// 手动拉取：从外部 API 获取数据并同步到 CMDB。
///
/// 工作流程：
/// 1. 读取 sync_source 配置（api_url / api_token / pull_config）
/// 2. 解析 pull_config 获取请求方法/路径/参数/响应路径
/// 3. HTTP 调用外部 API
/// 4. 从响应中提取数据项列表
/// 5. 走与 push 相同的 map → upsert → sync_log 流程
async fn pull_instances(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Json(req): Json<PullRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "asset:create")?;
    let ip = audit::extract_ip(&headers, Some(addr));

    // 1. 读取数据源配置
    let src = db::find_sync_source_by_code(&state.db, &req.source)
        .await?
        .ok_or_else(|| AppError::bad(&format!("数据源 {} 未注册", req.source)))?;
    if !src.is_enabled() {
        return Err(AppError::bad(&format!("数据源 {} 已禁用", req.source)));
    }
    if src.api_url.is_empty() {
        return Err(AppError::bad("数据源未配置 api_url，无法拉取"));
    }

    // 2. 解析 pull_config
    let config_str = src.pull_config.as_deref().unwrap_or("{}");
    let config: PullConfig = serde_json::from_str(config_str)
        .map_err(|e| AppError::bad(&format!("pull_config 解析失败: {}", e)))?;
    let model_code = req.model_code
        .or(if config.model_code.is_empty() { None } else { Some(config.model_code.clone()) })
        .ok_or_else(|| AppError::bad("未指定 modelCode（请求体或 pull_config 均未提供）"))?;

    // 3. 校验模型编码
    let models = db::list_ci_models(&state.db).await?;
    let model = models.iter()
        .find(|m| m.code == model_code)
        .ok_or_else(|| AppError::bad(&format!("CI 模型编码 {} 不存在", model_code)))?;
    let model_id = model.id.clone();

    // 4. 构建 HTTP 请求
    let full_url = if config.path.is_empty() {
        src.api_url.clone()
    } else {
        format!("{}{}", src.api_url.trim_end_matches('/'), config.path)
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| AppError::internal(&format!("HTTP 客户端创建失败: {}", e)))?;

    let mut request = match config.method.to_uppercase().as_str() {
        "POST" => {
            let mut r = client.post(&full_url);
            if !src.api_token.is_empty() {
                r = r.header("Authorization", format!("Bearer {}", src.api_token));
            }
            if !config.params.is_null() {
                r = r.json(&config.params);
            }
            r
        }
        _ => {
            let mut r = client.get(&full_url);
            if !src.api_token.is_empty() {
                r = r.header("Authorization", format!("Bearer {}", src.api_token));
            }
            if let Some(params) = config.params.as_object() {
                r = r.query(&params.iter()
                    .map(|(k, v)| (k.clone(), v.to_string().trim_matches('"').to_string()))
                    .collect::<Vec<_>>());
            }
            r
        }
    };

    // 添加额外 headers
    if let Some(extra) = config.headers.as_object() {
        for (k, v) in extra {
            if let Some(s) = v.as_str() {
                request = request.header(k, s);
            }
        }
    }

    // 5. 发送请求
    let resp = request.send().await
        .map_err(|e| AppError::bad(&format!("调用外部 API 失败: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::bad(&format!("外部 API 返回 {} : {}", status, &body[..body.len().min(500)])));
    }

    let resp_json: serde_json::Value = resp.json().await
        .map_err(|e| AppError::bad(&format!("解析外部 API 响应失败: {}", e)))?;

    // 6. 从响应中提取数据项列表
    let items = extract_items_by_path(&resp_json, &config.response_path);
    let total = items.len();

    // 7. 逐条处理（复用与 push 相同的 upsert 逻辑）
    let batch_id = uuid::Uuid::new_v4().to_string();
    let mut success_count = 0i32;
    let mut failed_count = 0i32;

    for item in &items {
        let mapped = if req.source == "blueking" {
            db::map_blueking_host(item)
        } else {
            map_generic_instance(item, &model_code)
        };

        let mapped = match mapped {
            Some(m) => m,
            None => {
                failed_count += 1;
                let _ = db::insert_sync_log(
                    &state.db, &req.source, &batch_id, "upsert", &model_code,
                    "", None, "", "failed", "字段映射失败：缺少必填字段",
                    Some(item),
                ).await;
                continue;
            }
        };

        let attrs_json = mapped.attributes.to_string();
        match db::upsert_ci_instance(
            &state.db, &model_id, &mapped.name, &mapped.status,
            &attrs_json, &req.source, &mapped.external_id,
        ).await {
            Ok((instance_id, is_new)) => {
                success_count += 1;
                let _ = db::insert_sync_log(
                    &state.db, &req.source, &batch_id, "upsert", &model_code,
                    &mapped.external_id, Some(&instance_id), &mapped.name,
                    "success", if is_new { "created" } else { "updated" },
                    Some(item),
                ).await;
            }
            Err(e) => {
                failed_count += 1;
                let _ = db::insert_sync_log(
                    &state.db, &req.source, &batch_id, "upsert", &model_code,
                    &mapped.external_id, None, &mapped.name,
                    "failed", &format!("入库失败: {}", e),
                    Some(item),
                ).await;
            }
        }
    }

    // 8. 更新数据源状态
    let sync_status = if failed_count == 0 { "success" } else if success_count == 0 { "failed" } else { "partial" };
    let _ = db::update_sync_source_status(&state.db, &req.source, success_count, sync_status).await;

    // 9. 审计
    let detail = serde_json::json!({
        "source": req.source,
        "modelCode": model_code,
        "batchId": batch_id,
        "mode": "pull",
        "total": total,
        "success": success_count,
        "failed": failed_count,
    });
    let _ = audit::log_async(
        &state.db, &auth, "pull_ci", "ci_instance", &batch_id, Some(&detail), &ip, sync_status,
    ).await;

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": {
            "batchId": batch_id,
            "mode": "pull",
            "total": total,
            "success": success_count,
            "failed": failed_count,
            "status": sync_status,
        }
    })))
}

/// 按 JSON path 从响应中提取数据项列表。
/// 支持点分路径如 "data.info" / "data.list" / "data"。
fn extract_items_by_path(resp: &serde_json::Value, path: &str) -> Vec<serde_json::Value> {
    let mut current = resp;
    for part in path.split('.') {
        if part.is_empty() { continue; }
        current = match current.get(part) {
            Some(v) => v,
            None => return vec![],
        };
    }
    match current {
        serde_json::Value::Array(arr) => arr.clone(),
        _ => vec![],
    }
}

/// 执行拉取（供定时任务调用，不需要认证）。
pub async fn do_pull(pool: &crate::db::DbPool, source_code: &str, model_code: &str) -> anyhow::Result<(usize, i32, i32)> {
    let src = db::find_sync_source_by_code(pool, source_code)
        .await?
        .ok_or_else(|| anyhow::anyhow!("数据源 {} 未注册", source_code))?;

    if src.api_url.is_empty() {
        anyhow::bail!("数据源未配置 api_url");
    }

    let config_str = src.pull_config.as_deref().unwrap_or("{}");
    let config: PullConfig = serde_json::from_str(config_str)?;
    let mc = if model_code.is_empty() { config.model_code.clone() } else { model_code.to_string() };
    if mc.is_empty() {
        anyhow::bail!("未指定 modelCode");
    }

    let models = db::list_ci_models(pool).await?;
    let model = models.iter()
        .find(|m| m.code == mc)
        .ok_or_else(|| anyhow::anyhow!("CI 模型编码 {} 不存在", mc))?;
    let model_id = model.id.clone();

    let full_url = if config.path.is_empty() {
        src.api_url.clone()
    } else {
        format!("{}{}", src.api_url.trim_end_matches('/'), config.path)
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let mut request = match config.method.to_uppercase().as_str() {
        "POST" => {
            let mut r = client.post(&full_url);
            if !src.api_token.is_empty() {
                r = r.header("Authorization", format!("Bearer {}", src.api_token));
            }
            if !config.params.is_null() {
                r = r.json(&config.params);
            }
            r
        }
        _ => {
            let mut r = client.get(&full_url);
            if !src.api_token.is_empty() {
                r = r.header("Authorization", format!("Bearer {}", src.api_token));
            }
            if let Some(params) = config.params.as_object() {
                r = r.query(&params.iter()
                    .map(|(k, v)| (k.clone(), v.to_string().trim_matches('"').to_string()))
                    .collect::<Vec<_>>());
            }
            r
        }
    };

    if let Some(extra) = config.headers.as_object() {
        for (k, v) in extra {
            if let Some(s) = v.as_str() {
                request = request.header(k, s);
            }
        }
    }

    let resp = request.send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("外部 API 返回 {} : {}", status, &body[..body.len().min(500)]);
    }

    let resp_json: serde_json::Value = resp.json().await?;
    let items = extract_items_by_path(&resp_json, &config.response_path);
    let total = items.len();

    let batch_id = uuid::Uuid::new_v4().to_string();
    let mut success_count = 0i32;
    let mut failed_count = 0i32;

    for item in &items {
        let mapped = if source_code == "blueking" {
            db::map_blueking_host(item)
        } else {
            map_generic_instance(item, &mc)
        };

        let mapped = match mapped {
            Some(m) => m,
            None => {
                failed_count += 1;
                let _ = db::insert_sync_log(
                    pool, source_code, &batch_id, "upsert", &mc,
                    "", None, "", "failed", "字段映射失败", Some(item),
                ).await;
                continue;
            }
        };

        let attrs_json = mapped.attributes.to_string();
        match db::upsert_ci_instance(
            pool, &model_id, &mapped.name, &mapped.status,
            &attrs_json, source_code, &mapped.external_id,
        ).await {
            Ok((instance_id, is_new)) => {
                success_count += 1;
                let _ = db::insert_sync_log(
                    pool, source_code, &batch_id, "upsert", &mc,
                    &mapped.external_id, Some(&instance_id), &mapped.name,
                    "success", if is_new { "created" } else { "updated" },
                    Some(item),
                ).await;
            }
            Err(e) => {
                failed_count += 1;
                let _ = db::insert_sync_log(
                    pool, source_code, &batch_id, "upsert", &mc,
                    &mapped.external_id, None, &mapped.name,
                    "failed", &format!("入库失败: {}", e), Some(item),
                ).await;
            }
        }
    }

    let sync_status = if failed_count == 0 { "success" } else if success_count == 0 { "failed" } else { "partial" };
    let _ = db::update_sync_source_status(pool, source_code, success_count, sync_status).await;

    tracing::info!(
        "定时拉取完成 source={} batch={} total={} success={} failed={}",
        source_code, batch_id, total, success_count, failed_count
    );

    Ok((total, success_count, failed_count))
}

/// 定时拉取调度循环：每 60 秒检查一次，匹配 cron 表达式则执行拉取。
pub async fn pull_scheduler_loop(pool: crate::db::DbPool) {
    tracing::info!("定时拉取调度器已启动");
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;

        let sources = match db::list_pull_enabled_sources(&pool).await {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("查询定时拉取数据源失败: {}", e);
                continue;
            }
        };

        let now = chrono::Utc::now();
        for src in &sources {
            if !cron_matches(&src.pull_cron, &now) {
                continue;
            }
            // 解析 model_code
            let config_str = src.pull_config.as_deref().unwrap_or("{}");
            let model_code = serde_json::from_str::<serde_json::Value>(config_str)
                .ok()
                .and_then(|v| v.get("modelCode")?.as_str().map(|s| s.to_string()))
                .unwrap_or_default();

            tracing::info!("定时拉取触发 source={} cron={}", src.code, src.pull_cron);
            if let Err(e) = do_pull(&pool, &src.code, &model_code).await {
                tracing::warn!("定时拉取失败 source={}: {}", src.code, e);
            }
        }
    }
}

/// 简单 5 字段 cron 匹配（minute hour day-of-month month day-of-week）。
/// 支持 * / 数字 / 逗号分隔 / 步进（*/N）。
fn cron_matches(expr: &str, now: &chrono::DateTime<chrono::Utc>) -> bool {
    let fields: Vec<&str> = expr.trim().split_whitespace().collect();
    if fields.len() != 5 {
        return false;
    }
    let m = now.format("%M").to_string().parse::<u32>().unwrap_or(0);
    let h = now.format("%H").to_string().parse::<u32>().unwrap_or(0);
    let dom = now.format("%d").to_string().parse::<u32>().unwrap_or(0);
    let mon = now.format("%m").to_string().parse::<u32>().unwrap_or(0);
    let dow = now.format("%w").to_string().parse::<u32>().unwrap_or(0); // 0=Sunday

    cron_field_matches(fields[0], m)
        && cron_field_matches(fields[1], h)
        && cron_field_matches(fields[2], dom)
        && cron_field_matches(fields[3], mon)
        && cron_field_matches(fields[4], dow)
}

fn cron_field_matches(field: &str, val: u32) -> bool {
    for part in field.split(',') {
        if part == "*" {
            return true;
        }
        if let Some(step_str) = part.strip_prefix("*/") {
            if let Ok(step) = step_str.parse::<u32>() {
                if step > 0 && val % step == 0 {
                    return true;
                }
            }
            continue;
        }
        if let Some(range) = part.find('-') {
            let (start_s, end_s) = part.split_at(range);
            let end_s = &end_s[1..];
            if let (Ok(start), Ok(end)) = (start_s.parse::<u32>(), end_s.parse::<u32>()) {
                if val >= start && val <= end {
                    return true;
                }
            }
            continue;
        }
        if let Ok(n) = part.parse::<u32>() {
            if val == n {
                return true;
            }
        }
    }
    false
}

// ---- 批量导入 CI 实例 ----

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BatchCreateInstancesRequest {
    model_id: String,
    items: Vec<BatchInstanceItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BatchInstanceItem {
    name: String,
    status: Option<String>,
    tags: Option<String>,
    attributes: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BatchImportError {
    row: usize,
    name: String,
    message: String,
}

/// 批量导入 CI 实例（Excel/CSV 解析后由前端提交）。
/// 采用 partial success 模式：单行错误不影响其他行。
/// 限制单次最多 1000 条。
async fn batch_create_instances(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Json(req): Json<BatchCreateInstancesRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "asset:create")?;
    let ip = audit::extract_ip(&headers, Some(addr));

    // 校验模型
    db::find_ci_model_by_id(&state.db, &req.model_id)
        .await?
        .ok_or_else(|| AppError::bad("CI 模型不存在"))?;

    // 获取属性定义用于校验与类型转换
    let attrs = db::list_ci_model_attrs(&state.db, &req.model_id).await?;

    let total = req.items.len();
    if total == 0 {
        return Err(AppError::bad("导入数据不能为空"));
    }
    if total > 1000 {
        return Err(AppError::bad("单次导入不超过 1000 条"));
    }

    let mut success_count = 0i32;
    let mut errors: Vec<BatchImportError> = Vec::new();

    for (idx, item) in req.items.iter().enumerate() {
        let row_no = idx + 1;
        let name = item.name.trim().to_string();
        if name.is_empty() {
            errors.push(BatchImportError {
                row: row_no,
                name: String::new(),
                message: "名称不能为空".into(),
            });
            continue;
        }

        // 校验 + 规范化属性
        let attrs_value = match normalize_attributes(&item.attributes, &attrs) {
            Ok(v) => v,
            Err(msg) => {
                errors.push(BatchImportError {
                    row: row_no,
                    name: name.clone(),
                    message: msg,
                });
                continue;
            }
        };
        let attrs_json = attrs_value.to_string();
        let status = item.status.clone().unwrap_or_else(|| "running".to_string());
        let tags = item.tags.clone().unwrap_or_default();

        match db::create_ci_instance(
            &state.db,
            &req.model_id,
            &name,
            &status,
            None,
            None,
            &attrs_json,
            &tags,
        )
        .await
        {
            Ok(_id) => success_count += 1,
            Err(e) => errors.push(BatchImportError {
                row: row_no,
                name: name.clone(),
                message: format!("入库失败: {}", e),
            }),
        }
    }

    let failed_count = (total as i32) - success_count;
    let status_str = if failed_count == 0 {
        "success"
    } else if success_count == 0 {
        "failed"
    } else {
        "partial"
    };

    // 审计
    let detail = serde_json::json!({
        "modelId": req.model_id,
        "mode": "batch_import",
        "total": total,
        "success": success_count,
        "failed": failed_count,
    });
    let _ = audit::log_async(
        &state.db,
        &auth,
        "batch_import_ci",
        "ci_instance",
        "",
        Some(&detail),
        &ip,
        status_str,
    )
    .await;

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": {
            "total": total,
            "success": success_count,
            "failed": failed_count,
            "status": status_str,
            "errors": errors,
        }
    })))
}

/// 校验并规范化属性值：
/// - 必填校验
/// - 类型转换（number→数字, boolean→布尔, enum→校验选项）
/// - 只保留模型定义的属性，忽略未知字段
fn normalize_attributes(
    raw: &Option<serde_json::Value>,
    attrs: &[db::CiModelAttr],
) -> Result<serde_json::Value, String> {
    let mut result = serde_json::Map::new();
    let raw_obj = match raw {
        Some(serde_json::Value::Object(m)) => m.clone(),
        Some(serde_json::Value::Null) | None => serde_json::Map::new(),
        Some(_) => return Err("attributes 必须是对象".into()),
    };

    for attr in attrs {
        let raw_val = raw_obj.get(&attr.code);
        // 判空
        let is_empty = match raw_val {
            None => true,
            Some(serde_json::Value::Null) => true,
            Some(serde_json::Value::String(s)) => s.trim().is_empty(),
            Some(_) => false,
        };
        if attr.is_required != 0 && is_empty {
            return Err(format!("属性「{}」不能为空", attr.name));
        }
        if is_empty {
            continue; // 空值不写入
        }

        let val = raw_val.unwrap();
        let normalized = match attr.value_type.as_str() {
            "number" => match val {
                serde_json::Value::Number(n) => serde_json::Value::Number(n.clone()),
                serde_json::Value::String(s) => {
                    let s = s.trim();
                    match s.parse::<f64>() {
                        Ok(n) => serde_json::Number::from_f64(n)
                            .map(serde_json::Value::Number)
                            .unwrap_or(serde_json::Value::Null),
                        Err(_) => return Err(format!("属性「{}」不是有效数字", attr.name)),
                    }
                }
                _ => return Err(format!("属性「{}」类型应为数字", attr.name)),
            },
            "boolean" => match val {
                serde_json::Value::Bool(b) => serde_json::Value::Bool(*b),
                serde_json::Value::String(s) => match s.trim().to_lowercase().as_str() {
                    "true" | "1" | "yes" | "y" | "是" => serde_json::Value::Bool(true),
                    "false" | "0" | "no" | "n" | "否" => serde_json::Value::Bool(false),
                    _ => return Err(format!("属性「{}」不是有效布尔值", attr.name)),
                },
                serde_json::Value::Number(n) => {
                    serde_json::Value::Bool(n.as_f64().map(|f| f != 0.0).unwrap_or(false))
                }
                _ => return Err(format!("属性「{}」类型应为布尔", attr.name)),
            },
            "enum" => {
                let s = match val {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string().trim_matches('"').to_string(),
                };
                // 校验选项
                if let Some(options_str) = &attr.options {
                    let options: Vec<String> = serde_json::from_str(options_str).unwrap_or_default();
                    if !options.is_empty() && !options.contains(&s) {
                        return Err(format!(
                            "属性「{}」值「{}」不在允许选项 {} 中",
                            attr.name,
                            s,
                            options.join("/")
                        ));
                    }
                }
                serde_json::Value::String(s)
            }
            _ => {
                // string/date/json → 统一存字符串
                match val {
                    serde_json::Value::String(s) => serde_json::Value::String(s.clone()),
                    other => serde_json::Value::String(other.to_string()),
                }
            }
        };
        result.insert(attr.code.clone(), normalized);
    }

    Ok(serde_json::Value::Object(result))
}


