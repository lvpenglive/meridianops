//! API 令牌管理路由（外部系统对接，如蓝鲸 CMDB webhook）。
//!
//! 路由：
//!   GET    /api/api-tokens/permissions     当前用户可用的权限点（新建对话框用） (登录即可)
//!   GET    /api/api-tokens                 令牌列表（管理员看全部，普通用户看自己）(登录即可)
//!   POST   /api/api-tokens                 新建令牌（scopes 必须是自身权限的子集）       (登录即可)
//!   POST   /api/api-tokens/:id/revoke      吊销令牌（admin 可吊销任意，普通用户只能自己的）(登录即可)
//!   PUT    /api/api-tokens/:id/expiry      更新有效期（同上）                              (登录即可)
//!   DELETE /api/api-tokens/:id             删除令牌（需 system:update，仅 admin 级）   (system:update)

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;

use crate::audit;
use crate::auth::{self, AuthUser};
use crate::db;
use crate::error::AppError;
use crate::routes::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/api-tokens/permissions", get(list_my_permissions))
        .route("/api/api-tokens", get(list_tokens).post(create_token))
        .route("/api/api-tokens/:id/revoke", post(revoke_token))
        .route("/api/api-tokens/:id/expiry", put(update_expiry))
        .route("/api/api-tokens/:id", delete(delete_token))
}

// ---- 辅助 ----

fn is_admin(auth: &AuthUser) -> bool {
    auth.0.role == auth::Role::Admin
}

/// 列表：管理员看全部，普通用户看自己。
async fn list_tokens(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let owner_filter = if is_admin(&auth) { None } else { Some(auth.0.uid.as_str()) };
    let list = db::list_api_tokens(&state.db, owner_filter).await?;
    Ok(Json(serde_json::json!({ "code": 0, "data": list })))
}

/// 返回当前用户可授予的权限（即自身 permissions），用于新建对话框 scope 选择。
/// 同时返回所有权限点（管理员可授予全部）。
async fn list_my_permissions(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    // 普通用户：只能给自身权限子集；admin 取库里所有权限点
    let my_perms: Vec<String> = if is_admin(&auth) {
        db::list_permissions(&state.db).await?.into_iter().map(|p| p.code).collect()
    } else {
        auth.0.permissions.clone()
    };
    // 分类：按冒号前分组
    let mut grouped: std::collections::BTreeMap<String, Vec<String>> = std::collections::BTreeMap::new();
    for perm in &my_perms {
        let group = perm.split(':').next().unwrap_or("other").to_string();
        grouped.entry(group).or_default().push(perm.clone());
    }
    let groups: Vec<serde_json::Value> = grouped
        .into_iter()
        .map(|(k, v)| serde_json::json!({ "group": k, "items": v }))
        .collect();

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": {
            "allPerms": my_perms,
            "groups": groups,
            "role": auth.0.role.to_string(),
        }
    })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateTokenReq {
    name: String,
    scopes: Vec<String>,
    /// 有效期方式："days" / "hours" / "never" / "custom"
    ttl_type: Option<String>,
    /// 当 ttl_type = days/hours 时使用
    ttl_value: Option<i32>,
    /// 当 ttl_type = custom 时使用，RFC3339
    expires_at: Option<String>,
    /// 角色：默认 operator（不影响权限裁剪，只影响 display）
    role: Option<String>,
}

async fn create_token(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(req): Json<CreateTokenReq>,
) -> Result<Json<serde_json::Value>, AppError> {
    if req.name.trim().is_empty() {
        return Err(AppError::bad("令牌名称不能为空"));
    }
    if req.scopes.is_empty() {
        return Err(AppError::bad("至少选择一个权限范围"));
    }

    // 1. 权限上限校验：授予 scopes 必须是自身权限子集（admin 走库里全量校验）
    let allowed_perms: Vec<String> = if is_admin(&auth) {
        db::list_permissions(&state.db)
            .await?
            .into_iter()
            .map(|p| p.code)
            .collect()
    } else {
        auth.0.permissions.clone()
    };
    for s in &req.scopes {
        if !allowed_perms.iter().any(|p| p == s || p == "*") {
            let msg = format!("不能授予超出自身的权限: {}", s);
            return Err(AppError::bad(&msg));
        }
    }

    // 2. 计算 expires_at RFC3339
    let expires_at_opt = match req.ttl_type.as_deref().unwrap_or("never") {
        "never" => None,
        "hours" => {
            let v = req.ttl_value.unwrap_or(24) as i64;
            if v <= 0 { return Err(AppError::bad("hours 必须为正整数")); }
            let dt = Utc::now() + Duration::hours(v);
            Some(dt.to_rfc3339())
        }
        "days" => {
            let v = req.ttl_value.unwrap_or(7) as i64;
            if v <= 0 { return Err(AppError::bad("days 必须为正整数")); }
            let dt = Utc::now() + Duration::days(v);
            Some(dt.to_rfc3339())
        }
        "custom" => {
            let ts = req.expires_at.as_deref().ok_or_else(|| AppError::bad("custom 模式需要 expires_at"))?;
            let _ = DateTime::parse_from_rfc3339(ts).map_err(|_| AppError::bad("expires_at 格式必须是 RFC3339"))?;
            Some(ts.to_string())
        }
        other => {
            let msg = format!("未知 ttl_type: {}", other);
            return Err(AppError::bad(&msg));
        }
    };

    // 3. 角色：默认 operator
    let role = req.role.clone().unwrap_or_else(|| "operator".to_string());
    if !matches!(role.as_str(), "admin" | "operator" | "viewer") {
        return Err(AppError::bad("role 必须是 admin / operator / viewer"));
    }
    // 非管理员不能创建 admin 角色 token
    if role == "admin" && !is_admin(&auth) {
        return Err(AppError::bad("仅管理员可创建 admin 级令牌"));
    }

    let (id, plain_token) = db::create_api_token(
        &state.db,
        req.name.trim(),
        &auth.0.uid,
        &req.scopes,
        &role,
        expires_at_opt.as_deref(),
    )
    .await?;

    // 审计
    let detail = serde_json::json!({
        "name": req.name,
        "scopes": req.scopes,
        "role": role,
        "expiresAt": expires_at_opt,
    });
    audit::log_async(&state.db, &auth, "create_api_token", "api_tokens", &id, Some(&detail), "", "success").await;

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": {
            "id": id,
            "token": plain_token,                 // **仅此次返回明文，之后只显示脱敏**
            "expiresAt": expires_at_opt,
        }
    })))
}

/// 吊销令牌。
async fn revoke_token(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    // 权限检查：普通用户只能吊销自己的
    let owner_filter = if is_admin(&auth) { None } else { Some(auth.0.uid.as_str()) };
    let ok = db::revoke_api_token(&state.db, &id, owner_filter).await?;
    if !ok {
        return Err(AppError::not_found("令牌不存在、已吊销或无权限"));
    }
    let detail = serde_json::json!({ "id": id });
    audit::log_async(&state.db, &auth, "revoke_api_token", "api_tokens", &id, Some(&detail), "", "success").await;
    Ok(Json(serde_json::json!({ "code": 0, "data": true })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateExpiryReq {
    ttl_type: String,
    ttl_value: Option<i32>,
    expires_at: Option<String>,
}

/// 更新有效期（延长或设为永不过期）。
async fn update_expiry(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    auth: AuthUser,
    Json(req): Json<UpdateExpiryReq>,
) -> Result<Json<serde_json::Value>, AppError> {
    // 计算
    let expires_at_opt = match req.ttl_type.as_str() {
        "never" => None,
        "hours" => {
            let v = req.ttl_value.unwrap_or(24) as i64;
            if v <= 0 { return Err(AppError::bad("hours 必须为正整数")); }
            Some((Utc::now() + Duration::hours(v)).to_rfc3339())
        }
        "days" => {
            let v = req.ttl_value.unwrap_or(7) as i64;
            if v <= 0 { return Err(AppError::bad("days 必须为正整数")); }
            Some((Utc::now() + Duration::days(v)).to_rfc3339())
        }
        "custom" => {
            let ts = req.expires_at.as_deref().ok_or_else(|| AppError::bad("custom 模式需要 expires_at"))?;
            let _ = DateTime::parse_from_rfc3339(ts).map_err(|_| AppError::bad("expires_at 格式必须是 RFC3339"))?;
            Some(ts.to_string())
        }
        other => {
            let msg = format!("未知 ttl_type: {}", other);
            return Err(AppError::bad(&msg));
        }
    };

    let owner_filter = if is_admin(&auth) { None } else { Some(auth.0.uid.as_str()) };
    let ok = db::update_api_token_expiry(&state.db, &id, expires_at_opt.as_deref(), owner_filter).await?;
    if !ok {
        return Err(AppError::not_found("令牌不存在或无权限"));
    }
    let detail = serde_json::json!({ "expiresAt": expires_at_opt });
    audit::log_async(&state.db, &auth, "update_api_token_expiry", "api_tokens", &id, Some(&detail), "", "success").await;
    Ok(Json(serde_json::json!({ "code": 0, "data": true })))
}

/// 删除（彻底删行，仅管理员，需 system:update 权限）。
async fn delete_token(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "system:update")?;
    let ok = db::delete_api_token(&state.db, &id).await?;
    if !ok {
        return Err(AppError::not_found("令牌不存在"));
    }
    let detail = serde_json::json!({ "id": id });
    audit::log_async(&state.db, &auth, "delete_api_token", "api_tokens", &id, Some(&detail), "", "success").await;
    Ok(Json(serde_json::json!({ "code": 0, "data": true })))
}
