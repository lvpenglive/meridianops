//! 系统设置路由：读取/更新 system_settings 表 + 暴露密码策略给前端。
//!
//! - GET  /api/system/settings        查询全部配置（system:read）
//! - PUT  /api/system/settings        批量更新配置（system:update）
//! - GET  /api/system/password-policy 查询当前密码策略描述（任意已登录用户）

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{ConnectInfo, State};
use axum::routing::get;
use axum::Json;
use axum::Router;
use serde::Deserialize;

use crate::audit;
use crate::auth;
use crate::db;
use crate::error::AppError;
use crate::routes::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/system/settings", get(list_settings).put(update_settings))
        .route("/api/system/password-policy", get(get_password_policy))
}

/// 查询全部系统配置（仅 system:read）。
async fn list_settings(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "system:read")?;
    let settings = db::list_all_settings(&state.db).await?;
    Ok(Json(serde_json::json!({
        "code": 0,
        "data": settings
    })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    /// 待更新的键值对（key → value）。仅更新提供的项，未提供项保持不变。
    pub settings: std::collections::HashMap<String, String>,
}

/// 批量更新系统配置（仅 system:update）。
/// 值会原样写入 system_settings（前端负责把布尔/数字转成字符串）。
async fn update_settings(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    auth: auth::AuthUser,
    Json(req_body): Json<UpdateSettingsRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "system:update")?;
    let ip = audit::extract_ip(&headers, Some(addr));

    if req_body.settings.is_empty() {
        return Err(AppError::bad("未提供任何配置项"));
    }
    // 已知 key 白名单（防止前端写入任意键）
    const ALLOWED_KEYS: &[&str] = &[
        "password_min_length",
        "password_require_uppercase",
        "password_require_lowercase",
        "password_require_digit",
        "password_require_special",
        "login_max_attempts",
        "login_lockout_minutes",
    ];
    let updated_by = auth.0.sub.clone();
    let mut entries: Vec<(String, String, String)> = Vec::new();
    let mut detail_map = serde_json::Map::new();
    for (key, value) in &req_body.settings {
        if !ALLOWED_KEYS.contains(&key.as_str()) {
            return Err(AppError::bad(&format!("不支持的配置项: {}", key)));
        }
        detail_map.insert(key.clone(), serde_json::Value::String(value.clone()));
        entries.push((key.clone(), value.clone(), updated_by.clone()));
    }

    db::upsert_settings(&state.db, &entries).await?;

    let detail = serde_json::Value::Object(detail_map);
    let _ = audit::log_async(
        &state.db,
        &auth,
        "update",
        "system_settings",
        "global",
        Some(&detail),
        &ip,
        "success",
    )
    .await;

    tracing::info!(by = %auth.0.sub, count = entries.len(), "system settings updated");
    Ok(Json(serde_json::json!({ "code": 0, "message": "配置已保存" })))
}

/// 查询当前密码策略（任意已登录用户）。
/// 个人中心修改密码页需要展示策略提示。
async fn get_password_policy(
    State(state): State<Arc<AppState>>,
    _auth: auth::AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let settings = db::list_all_settings(&state.db).await.unwrap_or_default();
    let map: std::collections::HashMap<String, String> = settings
        .into_iter()
        .map(|s| (s.setting_key, s.setting_value))
        .collect();
    let policy = auth::PasswordPolicy::from_settings(&map);
    Ok(Json(serde_json::json!({
        "code": 0,
        "data": {
            "minLength": policy.min_length,
            "requireUppercase": policy.require_uppercase,
            "requireLowercase": policy.require_lowercase,
            "requireDigit": policy.require_digit,
            "requireSpecial": policy.require_special,
            "description": policy.description(),
        }
    })))
}
