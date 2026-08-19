//! 系统设置路由：读取/更新 system_settings 表 + 暴露密码策略给前端。
//!
//! - GET  /api/system/settings        查询全部配置（system:read）
//! - PUT  /api/system/settings        批量更新配置（system:update）
//! - GET  /api/system/password-policy 查询当前密码策略描述（任意已登录用户）
//! - GET  /api/system/alert-ingress   查询告警接入配置（system:read）
//! - PUT  /api/system/alert-ingress   更新告警接入配置（system:update）

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
        .route(
            "/api/system/alert-ingress",
            get(get_alert_ingress).put(update_alert_ingress),
        )
}

/// 查询全部系统配置（仅 system:read）。
async fn list_settings(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "system:read")?;
    crate::license_routes::require_active_license(&state.db).await?;
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
    crate::license_routes::require_active_license(&state.db).await?;
    let ip = audit::extract_ip(&headers, Some(addr));

    if req_body.settings.is_empty() {
        return Err(AppError::bad("未提供任何配置项"));
    }
    // 已知 key 白名单（防止前端写入任意键）
    // 注意：alert_ingress_token / alert_ingress_enabled 由专用端点 /api/system/alert-ingress 管理
    //       （需同步 alerts_runtime 内存），通用 settings 接口不处理这两项。
    const ALLOWED_KEYS: &[&str] = &[
        "password_min_length",
        "password_require_uppercase",
        "password_require_lowercase",
        "password_require_digit",
        "password_require_special",
        "login_max_attempts",
        "login_lockout_minutes",
        "password_expiry_days",
        "session_timeout_minutes",
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
    let expiry_days: i64 = map
        .get("password_expiry_days")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(0);
    Ok(Json(serde_json::json!({
        "code": 0,
        "data": {
            "minLength": policy.min_length,
            "requireUppercase": policy.require_uppercase,
            "requireLowercase": policy.require_lowercase,
            "requireDigit": policy.require_digit,
            "requireSpecial": policy.require_special,
            "description": policy.description(),
            "expiryDays": expiry_days,
        }
    })))
}

// ============ 告警接入配置（运行时可更新） ============

/// GET /api/system/alert-ingress — 查询当前告警接入配置（system:read）。
/// 返回值：ingressEnabled + ingressToken（脱敏：仅显示前 4 位 + 后 4 位 + 长度）。
/// 说明：明文密钥不直接返回，前端如需复制明文需通过专门的「重新生成」按钮获取一次性明文。
async fn get_alert_ingress(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "system:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let (enabled, token, source) = {
        let cfg = state
            .alerts_runtime
            .read()
            .map_err(|e| AppError::internal(&format!("alerts_runtime lock poisoned: {}", e)))?;
        let source = if state.config.alerts.ingress_token == cfg.ingress_token
            && state.config.alerts.ingress_enabled == cfg.ingress_enabled
        {
            "config"
        } else {
            "database"
        };
        (cfg.ingress_enabled, cfg.ingress_token.clone(), source)
    };

    // 脱敏：长度 < 8 时全部遮罩
    let masked = if token.len() <= 8 {
        "*".repeat(token.len().max(4))
    } else {
        format!(
            "{}****{}",
            &token[..token.len().min(4)],
            &token[token.len() - 4..]
        )
    };

    // 拉取更新者与时间（从 system_settings 表读，记录最近一次密钥更新）
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT updated_by, updated_at FROM system_settings WHERE setting_key = 'alert_ingress_token'",
    )
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();
    let (updated_by_str, updated_at_str) = match row {
        Some((u, t)) => (Some(u), Some(t)),
        None => (None, None),
    };

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": {
            "ingressEnabled": enabled,
            "ingressTokenMasked": masked,
            "tokenLength": token.len(),
            "isDefault": token.starts_with("change-me") || token.is_empty(),
            "source": source,
            "updatedBy": updated_by_str,
            "updatedAt": updated_at_str,
        }
    })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAlertIngressRequest {
    /// 是否启用 ingress 接收端。None 时保持不变。
    pub ingress_enabled: Option<bool>,
    /// 新密钥（明文）。None 时保持不变；空字符串视为清空（不允许，应至少 8 位）。
    pub ingress_token: Option<String>,
    /// 若为 true，则忽略 ingress_token，服务端生成一个 32 字节随机密钥并以明文返回一次。
    pub regenerate: Option<bool>,
}

/// PUT /api/system/alert-ingress — 更新告警接入配置（system:update）。
/// 同步更新 system_settings 表 + alerts_runtime 内存，无需重启即时生效。
/// 若 regenerate=true，服务端生成新密钥并以明文返回一次（类似 API 令牌的「仅显示一次」语义）。
async fn update_alert_ingress(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    auth: auth::AuthUser,
    Json(req): Json<UpdateAlertIngressRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "system:update")?;
    crate::license_routes::require_active_license(&state.db).await?;
    let ip = audit::extract_ip(&headers, Some(addr));
    let updated_by = auth.0.sub.clone();

    // 1) 计算新值
    let (new_enabled, new_token, generated_plaintext) = {
        let cfg = state
            .alerts_runtime
            .read()
            .map_err(|e| AppError::internal(&format!("alerts_runtime lock poisoned: {}", e)))?;
        let new_enabled = req.ingress_enabled.unwrap_or(cfg.ingress_enabled);
        let mut new_token = req.ingress_token.clone().unwrap_or(cfg.ingress_token.clone());
        let mut generated: Option<String> = None;
        if req.regenerate.unwrap_or(false) {
            // 生成 32 字节随机 hex（64 字符）
            let mut buf = [0u8; 32];
            use rand::RngCore;
            rand::thread_rng().fill_bytes(&mut buf);
            new_token = hex::encode(buf);
            generated = Some(new_token.clone());
        } else if let Some(ref t) = req.ingress_token {
            // 校验自定义密钥：非空且至少 8 位
            if t.is_empty() {
                return Err(AppError::bad("密钥不能为空"));
            }
            if t.len() < 8 {
                return Err(AppError::bad("密钥长度至少 8 位"));
            }
            if t.starts_with("change-me") {
                return Err(AppError::bad("密钥不能以 change-me 开头"));
            }
        }
        (new_enabled, new_token, generated)
    };

    // 2) 写库
    let entries = vec![
        ("alert_ingress_token".to_string(), new_token.clone(), updated_by.clone()),
        (
            "alert_ingress_enabled".to_string(),
            (if new_enabled { "true" } else { "false" }).to_string(),
            updated_by.clone(),
        ),
    ];
    db::upsert_settings(&state.db, &entries).await?;

    // 3) 同步内存
    {
        let mut cfg = state
            .alerts_runtime
            .write()
            .map_err(|e| AppError::internal(&format!("alerts_runtime lock poisoned: {}", e)))?;
        cfg.ingress_enabled = new_enabled;
        cfg.ingress_token = new_token.clone();
    }

    // 4) 审计
    let detail = serde_json::json!({
        "ingressEnabled": new_enabled,
        "tokenRegenerated": req.regenerate.unwrap_or(false),
        "tokenUpdated": req.ingress_token.is_some(),
        "tokenLength": new_token.len(),
    });
    let _ = audit::log_async(
        &state.db,
        &auth,
        "update",
        "alert_ingress",
        "global",
        Some(&detail),
        &ip,
        "success",
    )
    .await;
    tracing::info!(by = %auth.0.sub, enabled = new_enabled, "alert ingress config updated");

    // 5) 返回：仅在 regenerate=true 时返回明文（仅此一次）
    let data = if let Some(plain) = generated_plaintext {
        serde_json::json!({
            "ingressEnabled": new_enabled,
            "ingressToken": plain,
            "regenerated": true,
            "warning": "此密钥仅返回一次，请立即保存",
        })
    } else {
        serde_json::json!({
            "ingressEnabled": new_enabled,
            "regenerated": false,
        })
    };
    Ok(Json(serde_json::json!({ "code": 0, "data": data })))
}
