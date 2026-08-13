//! 鉴权核心：argon2 密码哈希 + JWT 签发/验证 + AuthUser extractor。
//!
//! 模式选择：用 `FromRequestParts` extractor（AxleOps 路线），不挂 axum::middleware。
//! 受保护 handler 签名里写 `auth: AuthUser` 即触发校验，不写则不校验。

use std::fmt;
use std::sync::Arc;

use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use async_trait::async_trait;
use axum::extract::FromRequestParts;
use axum::http::header;
use axum::http::request::Parts;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::db;
use crate::error::AppError;
use crate::routes::AppState;

/// 三级固定角色。DB 层存字符串，应用层用 enum 强约束。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    Operator,
    Viewer,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::Operator => "operator",
            Role::Viewer => "viewer",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "admin" => Some(Role::Admin),
            "operator" => Some(Role::Operator),
            "viewer" => Some(Role::Viewer),
            _ => None,
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// JWT Claims。sub=username, uid=user_id, role=角色, permissions=权限码列表。
/// permissions 含 "*" 表示通配（开发模式匿名用户）。
/// pwd_exp=true 表示密码已过期，仅放行改密相关端点（合规强制）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub uid: String,
    pub role: Role,
    #[serde(default)]
    pub role_id: Option<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
    pub iat: usize,
    pub exp: usize,
    #[serde(default)]
    pub pwd_exp: bool,
}

/// argon2 密码哈希。
pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("hash password failed: {}", e))?;
    Ok(hash.to_string())
}

/// 校验密码。hash 格式非法或校验失败均返回 false。
pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

/// 签发 JWT。返回 `(token, expires_at_rfc3339)`。
/// pwd_exp=true 时签发的 token 仅可用于改密相关端点（合规强制）。
pub fn issue_token(
    uid: &str,
    username: &str,
    role: Role,
    role_id: Option<&str>,
    permissions: Vec<String>,
    secret: &str,
    ttl_hours: u64,
    pwd_exp: bool,
) -> anyhow::Result<(String, String)> {
    let now = Utc::now();
    let exp = now + Duration::hours(ttl_hours as i64);
    let claims = Claims {
        sub: username.to_string(),
        uid: uid.to_string(),
        role,
        role_id: role_id.map(|s| s.to_string()),
        permissions,
        iat: now.timestamp() as usize,
        exp: exp.timestamp() as usize,
        pwd_exp,
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;
    Ok((token, exp.to_rfc3339()))
}

/// 已认证用户 extractor。从 `Authorization: Bearer <token>` 解析 Claims。
///
/// 若 `config.auth.enabled == false`（开发关闭鉴权），返回匿名 admin 用户，
/// 便于本地不带 token 调试。生产环境必须 enabled=true。
pub struct AuthUser(pub Claims);

impl AuthUser {
    /// 登录失败等场景下的占位用户，仅用于审计日志。
    pub fn placeholder(username: String) -> Self {
        AuthUser(Claims {
            sub: username,
            uid: String::new(),
            role: Role::Viewer,
            role_id: None,
            permissions: Vec::new(),
            iat: 0,
            exp: 0,
            pwd_exp: false,
        })
    }

    /// 用户名（即 sub 字段）。
    pub fn username(&self) -> &str {
        &self.0.sub
    }

    /// 检查是否拥有某权限码。permissions 含 "*" 时通配放行。
    pub fn has_permission(&self, code: &str) -> bool {
        self.0.permissions.iter().any(|p| p == code || p == "*")
    }
}

/// 密码过期 token 允许访问的路径白名单（合规强制：过期后只能改密/查自己/登出）。
const PWD_EXPIRED_ALLOWLIST: &[&str] = &[
    "/api/auth/change-password",
    "/api/auth/me",
    "/api/auth/logout",
    "/api/system/password-policy",
];

#[async_trait]
impl FromRequestParts<Arc<AppState>> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        if !state.config.auth.enabled {
            return Ok(AuthUser(Claims {
                sub: "anonymous".to_string(),
                uid: String::new(),
                role: Role::Admin,
                role_id: None,
                permissions: vec!["*".to_string()],
                iat: 0,
                exp: usize::MAX,
                pwd_exp: false,
            }));
        }

        let token = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer ").map(str::trim))
            .filter(|s| !s.is_empty())
            .ok_or_else(|| AppError::unauthorized("missing token"))?;

        // 分支 1：API Token（前缀 mk-）→ 走 DB 校验 + 权限上限裁剪
        if let Some(stripped) = token.strip_prefix("mk-") {
            let plain = format!("mk-{}", stripped);
            let api_tok = db::find_valid_api_token(&state.db, &plain)
                .await
                .map_err(|_| AppError::unauthorized("invalid or expired token"))?
                .ok_or_else(|| AppError::unauthorized("invalid or expired token"))?;

            // 角色：从 Token 行里取，兜底 viewer
            let role = Role::parse(&api_tok.role).unwrap_or(Role::Viewer);
            let uid = api_tok.owner_user_id.clone();
            let username = format!("api-token:{}", api_tok.name);
            // exp：若 token 行 expires_at 为 Some，则用那个；否则永不过期（ usize::MAX ）
            let exp_usize = match api_tok.expires_at.as_deref() {
                None => usize::MAX,
                Some(ts) => chrono::DateTime::parse_from_rfc3339(ts)
                    .map(|d| d.timestamp() as usize)
                    .unwrap_or(0),
            };
            let iat = chrono::Utc::now().timestamp() as usize;
            let permissions = api_tok.parse_scopes();

            return Ok(AuthUser(Claims {
                sub: username,
                uid,
                role,
                role_id: None,
                permissions,
                iat,
                exp: exp_usize,
                pwd_exp: false,
            }));
        }

        // 分支 2：普通 JWT → 原校验逻辑
        let mut validation = Validation::default();
        validation.validate_exp = true;
        let data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
            &validation,
        )
        .map_err(|_| AppError::unauthorized("invalid or expired token"))?;

        // 合规强制：密码过期 token 仅放行白名单路径
        if data.claims.pwd_exp {
            let path = parts.uri.path();
            if !PWD_EXPIRED_ALLOWLIST.contains(&path) {
                return Err(AppError::forbidden("密码已过期，请先修改密码"));
            }
        }

        Ok(AuthUser(data.claims))
    }
}

/// 简单角色守卫：要求 admin。非 admin 返回 403。
pub fn require_admin(user: &AuthUser) -> Result<(), AppError> {
    if user.0.role != Role::Admin {
        return Err(AppError::forbidden("admin only"));
    }
    Ok(())
}

/// 权限守卫：要求拥有指定权限码。无权限返回 403。
/// permissions 含 "*" 时通配放行（开发模式匿名用户）。
pub fn require_permission(user: &AuthUser, code: &str) -> Result<(), AppError> {
    if !user.has_permission(code) {
        return Err(AppError::forbidden(&format!("需要权限: {}", code)));
    }
    Ok(())
}

// ============ 密码强度策略 ============

/// 密码策略配置。从 system_settings 表读取后构造。
#[derive(Debug, Clone)]
pub struct PasswordPolicy {
    pub min_length: usize,
    pub require_uppercase: bool,
    pub require_lowercase: bool,
    pub require_digit: bool,
    pub require_special: bool,
}

impl Default for PasswordPolicy {
    fn default() -> Self {
        Self {
            min_length: 8,
            require_uppercase: true,
            require_lowercase: true,
            require_digit: true,
            require_special: false,
        }
    }
}

impl PasswordPolicy {
    /// 从 system_settings 的键值 map 构造策略。缺失的项用默认值。
    pub fn from_settings(map: &std::collections::HashMap<String, String>) -> Self {
        let parse_bool = |key: &str| map.get(key).map(|v| v == "true").unwrap_or(true);
        let parse_usize = |key: &str| {
            map.get(key)
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(8)
        };
        Self {
            min_length: parse_usize("password_min_length"),
            require_uppercase: parse_bool("password_require_uppercase"),
            require_lowercase: parse_bool("password_require_lowercase"),
            require_digit: parse_bool("password_require_digit"),
            require_special: parse_bool("password_require_special"),
        }
    }

    /// 返回策略的人类可读描述（用于前端提示）。
    pub fn description(&self) -> String {
        let mut parts = vec![format!("至少 {} 位", self.min_length)];
        if self.require_uppercase {
            parts.push("大写字母".to_string());
        }
        if self.require_lowercase {
            parts.push("小写字母".to_string());
        }
        if self.require_digit {
            parts.push("数字".to_string());
        }
        if self.require_special {
            parts.push("特殊字符".to_string());
        }
        format!("密码需包含：{}", parts.join("、"))
    }
}

/// 校验密码强度。不满足策略时返回 Err(错误描述)。
pub fn validate_password_strength(password: &str, policy: &PasswordPolicy) -> Result<(), String> {
    if password.len() < policy.min_length {
        return Err(format!("密码至少 {} 位", policy.min_length));
    }
    if policy.require_uppercase && !password.chars().any(|c| c.is_uppercase()) {
        return Err("密码需包含大写字母".to_string());
    }
    if policy.require_lowercase && !password.chars().any(|c| c.is_lowercase()) {
        return Err("密码需包含小写字母".to_string());
    }
    if policy.require_digit && !password.chars().any(|c| c.is_ascii_digit()) {
        return Err("密码需包含数字".to_string());
    }
    if policy.require_special
        && !password
            .chars()
            .any(|c| !c.is_alphanumeric())
    {
        return Err("密码需包含特殊字符".to_string());
    }
    Ok(())
}
