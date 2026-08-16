//! 产品授权（License）路由：读取/设置产品使用期限等授权信息。
//!
//! 公开接口：
//!   GET  /api/license/status        —— 任意已登录用户，查询授权状态（前端用于展示页脚和到期预警）
//!   GET  /api/license/admin         —— 仅 system:read，管理员查询完整授权信息（含激活码脱敏）
//!   PUT  /api/license/admin         —— 仅 system:update，管理员更新授权（设置到期时间、客户名、激活码等）
//!
//! 注意：本模块使用 system_settings 表存储键值，键前缀均为 `license_`。

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::Json;
use axum::Router;
use serde::{Deserialize, Serialize};

use crate::audit;
use crate::auth;
use crate::db;
use crate::error::AppError;
use crate::routes::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/license/status", get(get_license_status))
        .route("/api/license/admin", get(get_license_admin).put(update_license_admin))
}

/// 授权信息行（用于前端展示）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseInfo {
    pub edition: String,
    pub customer: String,
    /// 到期时间字符串 (RFC3339 或 Y-m-d)；空 = 永不到期。
    pub expires_at: String,
    /// 激活时间字符串；空 = 未激活。
    pub activated_at: String,
    /// 剩余天数（秒级精度取整）。永不到期 = i64::MAX，已过期 = 负数。
    pub days_remaining: i64,
    /// 是否已过期。永不到期 = false。
    pub is_expired: bool,
    /// 到期预警级别：none / soon(30天内) / urgent(7天内) / expired
    pub warn_level: String,
}

/// 授权键列表（读取时用）。
const LICENSE_KEYS: &[&str] = &[
    "license_edition",
    "license_expires_at",
    "license_customer",
    "license_activated_at",
    "license_key",
];

pub async fn load_license_map(
    pool: &sqlx::MySqlPool,
) -> std::collections::HashMap<String, String> {
    let rows = db::list_all_settings(pool).await.unwrap_or_default();
    rows.into_iter()
        .filter(|s| s.setting_key.starts_with("license_"))
        .map(|s| (s.setting_key, s.setting_value))
        .collect()
}

/// 把到期时间字符串解析为 UTC DateTime。支持 RFC3339 与 %Y-%m-%d %H:%M:%S 格式。
fn parse_expiry(raw: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    if raw.is_empty() {
        return None;
    }
    if let Ok(t) = chrono::DateTime::parse_from_rfc3339(raw) {
        return Some(t.with_timezone(&chrono::Utc));
    }
    // 兼容 MySQL YYYY-MM-DD HH:MM:SS (被 chrono 认为是无时区, 按 UTC 解析)
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S") {
        return Some(chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
            naive,
            chrono::Utc,
        ));
    }
    if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(raw, "%Y-%m-%d") {
        let naive = naive_date.and_hms_opt(23, 59, 59).unwrap();
        return Some(chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
            naive,
            chrono::Utc,
        ));
    }
    None
}

/// 根据全部键值对计算出 LicenseInfo。
pub fn build_license_info(map: &std::collections::HashMap<String, String>) -> LicenseInfo {
    let edition = map
        .get("license_edition")
        .cloned()
        .unwrap_or_else(|| "Community".to_string());
    let customer = map
        .get("license_customer")
        .cloned()
        .unwrap_or_default();
    let expires_at_raw = map
        .get("license_expires_at")
        .cloned()
        .unwrap_or_default();
    let activated_at = map
        .get("license_activated_at")
        .cloned()
        .unwrap_or_default();

    let now = chrono::Utc::now();
    let (expiry_dt, days_remaining, is_expired, warn_level) = match parse_expiry(&expires_at_raw) {
        None => {
            // 空值视为永不到期
            (None, i64::MAX, false, "none".to_string())
        }
        Some(exp) => {
            let dur = exp - now;
            let days = dur.num_seconds().div_euclid(86400); // 向下取整，避免 0 天未过期
            let is_expired_now = dur.num_seconds() <= 0;
            let warn = if is_expired_now {
                "expired".to_string()
            } else if days <= 7 {
                "urgent".to_string()
            } else if days <= 30 {
                "soon".to_string()
            } else {
                "none".to_string()
            };
            (Some(exp), days, is_expired_now, warn)
        }
    };
    let expires_at = if let Some(dt) = expiry_dt {
        dt.to_rfc3339()
    } else {
        String::new()
    };
    LicenseInfo {
        edition,
        customer,
        expires_at,
        activated_at,
        days_remaining,
        is_expired,
        warn_level,
    }
}

/// 便捷：读取当前授权状态（已过期返回 Err(license_expired)）。
pub async fn require_active_license(
    pool: &sqlx::MySqlPool,
) -> Result<LicenseInfo, AppError> {
    let map = load_license_map(pool).await;
    let info = build_license_info(&map);
    if info.is_expired {
        return Err(AppError::license_expired("产品授权已过期，请联系管理员续期"));
    }
    Ok(info)
}

// ========== HTTP handlers ==========

/// 查询授权状态（任意已登录用户）。前端登录后拉取一次即可判断是否显示到期预警。
async fn get_license_status(
    State(state): State<Arc<AppState>>,
    _auth: auth::AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let map = load_license_map(&state.db).await;
    let info = build_license_info(&map);
    Ok(Json(serde_json::json!({ "code": 0, "data": info })))
}

/// 查询完整授权信息（管理员，system:read）。
async fn get_license_admin(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "system:read")?;
    let map = load_license_map(&state.db).await;
    let mut info = build_license_info(&map);
    let key = map.get("license_key").cloned().unwrap_or_default();
    let fingerprint = crate::license_crypto::get_machine_fingerprint(&state.db).await;
    Ok(Json(serde_json::json!({
        "code": 0,
        "data": {
            "edition": info.edition,
            "customer": info.customer,
            "expiresAt": info.expires_at,
            "activatedAt": info.activated_at,
            "daysRemaining": info.days_remaining,
            "isExpired": info.is_expired,
            "warnLevel": info.warn_level,
            "licenseKey": mask_key(&key),
            "fingerprint": fingerprint,
        }
    })))
}

/// 对激活码做脱敏显示（首4+尾4，其余用 * 替代）。
fn mask_key(key: &str) -> String {
    if key.is_empty() {
        return String::new();
    }
    if key.len() <= 8 {
        return "*".repeat(key.len());
    }
    let head: String = key.chars().take(4).collect();
    let tail: String = key.chars().rev().take(4).collect::<String>().chars().rev().collect();
    format!("{}{}{}", head, "*".repeat(key.len() - 8), tail)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLicenseRequest {
    pub edition: Option<String>,
    pub customer: Option<String>,
    /// 新的到期时间。传空字符串代表「永不到期」。
    pub expires_at: Option<String>,
    pub license_key: Option<String>,
}

/// 更新授权信息（仅 system:update）。
async fn update_license_admin(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Json(req): Json<UpdateLicenseRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "system:update")?;
    let ip = audit::extract_ip(&headers, Some(addr));
    let updated_by = auth.0.sub.clone();

    // 1. 先把当前的值读到内存做合并
    let cur_map = load_license_map(&state.db).await;
    let mut edition = cur_map
        .get("license_edition")
        .cloned()
        .unwrap_or_else(|| "Community".to_string());
    let mut customer = cur_map.get("license_customer").cloned().unwrap_or_default();
    let mut expires_at = cur_map
        .get("license_expires_at")
        .cloned()
        .unwrap_or_default();
    let mut license_key = cur_map.get("license_key").cloned().unwrap_or_default();

    // ---- 激活码验签 ----
    // 如果提供了激活码，先验签。验签通过后用 payload 中的信息覆盖手动填写值。
    let mut key_verified = false;
    if let Some(ref v) = req.license_key {
        let key_str = v.trim();
        if !key_str.is_empty() {
            // 获取当前机器指纹
            let fingerprint = crate::license_crypto::get_machine_fingerprint(&state.db).await;
            // 验签
            match crate::license_crypto::verify_license(key_str, &fingerprint) {
                Ok(verified) => {
                    if !verified.fingerprint_match {
                        return Err(AppError::bad(&format!(
                            "激活码机器指纹不匹配（期望: {}, 当前: {}），该激活码无法在本机使用",
                            verified.payload.fingerprint, fingerprint
                        )));
                    }
                    // 验签通过：用 payload 中的信息覆盖
                    edition = verified.payload.edition.clone();
                    customer = verified.payload.customer.clone();
                    // payload 中的 expires_at 是 RFC3339 格式，直接使用
                    expires_at = verified.payload.expires_at.clone();
                    license_key = key_str.to_string();
                    key_verified = true;
                    tracing::info!(
                        edition = %edition,
                        customer = %customer,
                        "license key verified successfully"
                    );
                }
                Err(e) => {
                    return Err(AppError::bad(&format!("激活码验证失败：{}", e)));
                }
            }
        } else {
            // 空字符串 = 清除激活码
            license_key = String::new();
        }
    }

    // 如果没有通过激活码设置，则使用手动填写的值
    if !key_verified {
        if let Some(v) = req.edition.filter(|s| !s.trim().is_empty()) {
            edition = v.trim().to_string();
        }
        if let Some(v) = req.customer {
            customer = v;
        }
        if let Some(v) = req.expires_at {
            // 传空 => 永不到期（但需要是合法的空或合法日期）
            if !v.trim().is_empty() {
                // 校验格式
                if parse_expiry(v.trim()).is_none() {
                    return Err(AppError::bad(
                        "到期时间格式不正确，支持 RFC3339 / Y-m-d H:M:S / Y-m-d",
                    ));
                }
            }
            expires_at = v.trim().to_string();
        }
    }

    // 版本白名单（防止任意值）
    if !matches!(
        edition.as_str(),
        "Community" | "Enterprise" | "Ultimate"
    ) {
        return Err(AppError::bad("版本只能是 Community / Enterprise / Ultimate"));
    }

    // 激活时间：如果原来没有激活，且现在设置了激活码/到期时间，则写入当前时间
    let activated_raw = cur_map
        .get("license_activated_at")
        .cloned()
        .unwrap_or_default();
    let activated_at = if activated_raw.trim().is_empty()
        && (!expires_at.is_empty() || !license_key.is_empty())
    {
        chrono::Utc::now().to_rfc3339()
    } else {
        activated_raw
    };

    // 2. 写回库
    let entries: Vec<(String, String, String)> = vec![
        ("license_edition".to_string(), edition, updated_by.clone()),
        ("license_customer".to_string(), customer, updated_by.clone()),
        ("license_expires_at".to_string(), expires_at, updated_by.clone()),
        ("license_activated_at".to_string(), activated_at, updated_by.clone()),
        ("license_key".to_string(), license_key, updated_by.clone()),
    ];
    db::upsert_settings(&state.db, &entries).await?;

    // 3. 审计 + 返回
    let detail = serde_json::json!({
        "edition": entries[0].1,
        "customer": entries[1].1,
        "expiresAt": entries[2].1,
        "activatedAt": entries[3].1,
        "licenseKeyUpdated": !entries[4].1.is_empty(),
    });
    let _ = audit::log_async(
        &state.db,
        &auth,
        "update_license",
        "system_settings",
        "license",
        Some(&detail),
        &ip,
        "success",
    )
    .await;

    let final_map = load_license_map(&state.db).await;
    let info = build_license_info(&final_map);
    let key = final_map.get("license_key").cloned().unwrap_or_default();
    tracing::info!(by = %updated_by, "license updated");
    Ok(Json(serde_json::json!({
        "code": 0,
        "message": "授权已更新",
        "data": {
            "edition": info.edition,
            "customer": info.customer,
            "expiresAt": info.expires_at,
            "activatedAt": info.activated_at,
            "daysRemaining": info.days_remaining,
            "isExpired": info.is_expired,
            "warnLevel": info.warn_level,
            "licenseKey": mask_key(&key),
        }
    })))
}
