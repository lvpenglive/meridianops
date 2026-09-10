//! 鉴权相关 HTTP 路由：login / me / logout / list_users。
//!
//! 字段命名统一 camelCase（后端 `#[serde(rename_all = "camelCase")]`），
//! 与前端现有 `api/types.ts` 风格一致。
//!
//! 成功响应：`{ "code": 0, "data": ... }`
//! 错误响应：由 `AppError` 输出 `{ "code": 4xx/5xx, "message": "..." }`

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{ConnectInfo, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{delete, get, patch, post, put};
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
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/me", get(me))
        .route("/api/auth/change-password", post(change_password))
        // 用户管理：列表+创建（GET/POST 同路径），单条编辑（PUT），启停（PATCH），重置密码（POST）
        .route("/api/users", get(list_users).post(create_user))
        .route("/api/users/:id", put(update_user).delete(delete_user))
        .route("/api/users/:id/enable", patch(toggle_enable))
        .route("/api/users/:id/password-reset", post(reset_password))
}

/// 从 system_settings 加载密码策略。失败时用默认策略。
async fn load_password_policy(state: &Arc<AppState>) -> auth::PasswordPolicy {
    match db::list_all_settings(&state.db).await {
        Ok(settings) => {
            let map: std::collections::HashMap<String, String> = settings
                .into_iter()
                .map(|s| (s.setting_key, s.setting_value))
                .collect();
            auth::PasswordPolicy::from_settings(&map)
        }
        Err(_) => auth::PasswordPolicy::default(),
    }
}

/// 从 system_settings 读取登录锁定参数。失败用默认值 (5, 15)。
async fn load_lockout_config(state: &Arc<AppState>) -> (i32, i64) {
    let max_attempts = db::get_setting(&state.db, "login_max_attempts")
        .await
        .ok()
        .flatten()
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(5);
    let lockout_minutes = db::get_setting(&state.db, "login_lockout_minutes")
        .await
        .ok()
        .flatten()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(15);
    (max_attempts, lockout_minutes)
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub email: String,
    pub role: auth::Role,
    pub role_id: Option<String>,
    pub department_id: Option<String>,
    pub mobile: Option<String>,
    pub employee_no: Option<String>,
    pub position: Option<String>,
    pub manager_id: Option<String>,
    pub im_account: Option<String>,
    pub employment_status: String,
    pub leave_date: Option<String>,
    pub remark: Option<String>,
    pub enabled: bool,
    pub last_login_at: Option<String>,
    pub password_changed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<db::User> for UserInfo {
    fn from(u: db::User) -> Self {
        let enabled = u.is_enabled();
        let role = auth::Role::parse(&u.role).unwrap_or(auth::Role::Viewer);
        Self {
            id: u.id,
            username: u.username,
            display_name: u.display_name,
            email: u.email,
            role,
            role_id: u.role_id,
            department_id: u.department_id,
            mobile: u.mobile,
            employee_no: u.employee_no,
            position: u.position,
            manager_id: u.manager_id,
            im_account: u.im_account,
            employment_status: u.employment_status,
            leave_date: u.leave_date,
            remark: u.remark,
            enabled,
            last_login_at: u.last_login_at,
            password_changed_at: u.password_changed_at,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub token: String,
    pub expires_at: String,
    pub user: UserInfo,
    /// 密码是否已过期。true 时 token 带 pwd_exp claim，仅能访问改密端点。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password_expired: Option<bool>,
    /// 会话超时分钟数（0=不超时）。前端据此做客户端 idle 计时。
    pub session_timeout_minutes: i64,
    /// 当前产品授权摘要（前端用于页脚标识+到期预警）。
    pub license: LicenseSummary,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseSummary {
    pub edition: String,
    pub customer: String,
    pub expires_at: String,
    pub activated_at: String,
    pub days_remaining: i64,
    pub is_expired: bool,
    pub warn_level: String,
}

impl From<crate::license_routes::LicenseInfo> for LicenseSummary {
    fn from(i: crate::license_routes::LicenseInfo) -> Self {
        Self {
            edition: i.edition,
            customer: i.customer,
            expires_at: i.expires_at,
            activated_at: i.activated_at,
            days_remaining: i.days_remaining,
            is_expired: i.is_expired,
            warn_level: i.warn_level,
        }
    }
}

/// 登录：校验用户名密码，签发 JWT。
/// 用户名不存在与密码错误返回相同提示，避免枚举用户名。
/// 登录失败累计达阈值后锁定账号一段时间。
async fn login(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req_body): Json<LoginRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let ip = audit::extract_ip(&headers, Some(addr));
    if req_body.username.trim().is_empty() || req_body.password.is_empty() {
        return Err(AppError::bad("用户名和密码不能为空"));
    }
    let user = match db::find_user_by_username(&state.db, req_body.username.trim()).await? {
        Some(u) => u,
        None => {
            let _ = audit::log_async(
                &state.db,
                &auth::AuthUser::placeholder(req_body.username.trim().to_string()),
                "login",
                "user",
                "unknown",
                None,
                &ip,
                "failure",
            )
            .await;
            return Err(AppError::unauthorized("用户名或密码错误"));
        }
    };
    if !user.is_enabled() {
        let detail = serde_json::json!({"reason": "disabled"});
        let _ = audit::log_async(
            &state.db,
            &auth::AuthUser::placeholder(user.username.clone()),
            "login",
            "user",
            &user.id,
            Some(&detail),
            &ip,
            "failure",
        )
        .await;
        return Err(AppError::forbidden("账号已禁用，请联系管理员"));
    }
    // 检查账号是否被锁定
    if let Some(locked_until) = db::check_user_locked(&state.db, &user.id).await? {
        let detail = serde_json::json!({"reason": "locked", "lockedUntil": locked_until});
        let _ = audit::log_async(
            &state.db,
            &auth::AuthUser::placeholder(user.username.clone()),
            "login",
            "user",
            &user.id,
            Some(&detail),
            &ip,
            "failure",
        )
        .await;
        // 计算剩余分钟
        let remaining = chrono::DateTime::parse_from_rfc3339(&locked_until)
            .ok()
            .map(|t| (t.with_timezone(&chrono::Utc) - chrono::Utc::now()).num_minutes().max(1))
            .unwrap_or(0);
        return Err(AppError::forbidden(&format!(
            "账号已锁定，请 {} 分钟后重试",
            remaining
        )));
    }
    if !auth::verify_password(&req_body.password, &user.password_hash) {
        // 登录失败：递增计数，可能触发锁定
        let (max_attempts, lockout_minutes) = load_lockout_config(&state).await;
        let locked = db::increment_failed_login(
            &state.db,
            &user.id,
            max_attempts,
            lockout_minutes,
        )
        .await?;
        let detail = if locked.is_some() {
            serde_json::json!({"reason": "bad_password", "locked": true})
        } else {
            serde_json::json!({"reason": "bad_password"})
        };
        let _ = audit::log_async(
            &state.db,
            &auth::AuthUser::placeholder(user.username.clone()),
            "login",
            "user",
            &user.id,
            Some(&detail),
            &ip,
            "failure",
        )
        .await;
        if locked.is_some() {
            return Err(AppError::forbidden(&format!(
                "密码错误次数过多，账号已锁定 {} 分钟",
                lockout_minutes
            )));
        }
        return Err(AppError::unauthorized("用户名或密码错误"));
    }
    let role = auth::Role::parse(&user.role).unwrap_or(auth::Role::Viewer);
    // 查询用户权限码列表（通过 role_id），写入 JWT claims
    let permissions = db::list_permission_codes_by_user(&state.db, user.role_id.as_deref()).await?;

    // 合规检查：密码是否过期
    let expiry_days = db::get_setting(&state.db, "password_expiry_days")
        .await
        .ok()
        .flatten()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(0);
    let password_expired = is_password_expired(user.password_changed_at.as_deref(), expiry_days);

    let (token, expires_at) = auth::issue_token(
        &user.id,
        &user.username,
        role,
        user.role_id.as_deref(),
        permissions,
        &state.jwt_secret,
        state.jwt_ttl_hours,
        password_expired,
    )?;
    // 登录成功：重置失败计数
    let _ = db::reset_failed_login(&state.db, &user.id).await;
    let _ = db::update_last_login(&state.db, &user.id).await;
    let user_info = UserInfo::from(user);
    let detail = serde_json::json!({"role": role.as_str(), "passwordExpired": password_expired});
    let _ = audit::log_async(
        &state.db,
        &auth::AuthUser::placeholder(user_info.username.clone()),
        "login",
        "user",
        &user_info.id,
        Some(&detail),
        &ip,
        "success",
    )
    .await;
    let session_timeout_minutes = db::get_setting(&state.db, "session_timeout_minutes")
        .await
        .ok()
        .flatten()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(0);

    // 授权信息：登录时一并返回（到期了仍允许登录，但返回 is_expired=true，前端拦截功能操作）
    let license_map = crate::license_routes::load_license_map(&state.db).await;
    let license_info = crate::license_routes::build_license_info(&license_map);
    let license_summary = LicenseSummary::from(license_info);

    tracing::info!(username = %user_info.username, pwd_expired = password_expired, "user logged in");
    Ok(Json(serde_json::json!({
        "code": 0,
        "data": LoginResponse {
            token,
            expires_at,
            user: user_info,
            password_expired: if password_expired { Some(true) } else { None },
            session_timeout_minutes,
            license: license_summary,
        }
    })))
}

/// 判断密码是否已过期。
/// expiry_days <= 0 视为不过期；password_changed_at 为 None 视为不过期（兼容旧数据）。
fn is_password_expired(password_changed_at: Option<&str>, expiry_days: i64) -> bool {
    if expiry_days <= 0 {
        return false;
    }
    let changed = match password_changed_at {
        Some(s) => s,
        None => return false,
    };
    let changed_dt = match chrono::DateTime::parse_from_rfc3339(changed) {
        Ok(t) => t.with_timezone(&chrono::Utc),
        Err(_) => return false, // 时间格式异常不阻断登录
    };
    let now = chrono::Utc::now();
    now.signed_duration_since(changed_dt).num_days() >= expiry_days
}

/// 当前登录用户信息。受 AuthUser 保护。
async fn me(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let user = db::find_user_by_id(&state.db, &auth.0.uid)
        .await?
        .ok_or_else(|| AppError::not_found("用户不存在"))?;
    Ok(Json(serde_json::json!({
        "code": 0,
        "data": UserInfo::from(user)
    })))
}

/// 登出。JWT 无状态，仅返回 ok；真正失效由前端清 token 完成。
async fn logout() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "code": 0, "message": "logged out" }))
}

// ---- 个人修改密码 ----

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

/// 用户自助修改密码。需校验旧密码 + 新密码强度策略。
/// 任何已登录用户均可调用（无权限码要求），但只能改自己的密码。
async fn change_password(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Json(req_body): Json<ChangePasswordRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let ip = audit::extract_ip(&headers, Some(addr));

    if req_body.new_password.is_empty() || req_body.old_password.is_empty() {
        return Err(AppError::bad("原密码与新密码不能为空"));
    }
    if req_body.new_password == req_body.old_password {
        return Err(AppError::bad("新密码不能与原密码相同"));
    }

    let user = db::find_user_by_id(&state.db, &auth.0.uid)
        .await?
        .ok_or_else(|| AppError::not_found("用户不存在"))?;
    if !user.is_enabled() {
        return Err(AppError::forbidden("账号已禁用"));
    }
    if !auth::verify_password(&req_body.old_password, &user.password_hash) {
        let _ = audit::log_async(
            &state.db,
            &auth,
            "change_password",
            "user",
            &user.id,
            Some(&serde_json::json!({"reason": "bad_old_password"})),
            &ip,
            "failure",
        )
        .await;
        return Err(AppError::bad("原密码不正确"));
    }

    // 校验新密码强度
    let policy = load_password_policy(&state).await;
    if let Err(msg) = auth::validate_password_strength(&req_body.new_password, &policy) {
        return Err(AppError::bad(&msg));
    }

    let new_hash = auth::hash_password(&req_body.new_password)?;
    db::update_password(&state.db, &user.id, &new_hash).await?;

    let _ = audit::log_async(
        &state.db,
        &auth,
        "change_password",
        "user",
        &user.id,
        None,
        &ip,
        "success",
    )
    .await;

    tracing::info!(user_id = %user.id, by = %auth.0.sub, "user changed own password");
    Ok(Json(serde_json::json!({ "code": 0, "message": "密码修改成功" })))
}

/// 列出所有用户（仅 admin）。
async fn list_users(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "user:read")?;
    crate::license_routes::require_active_license(&state.db).await?;
    let users = db::list_all_users(&state.db).await?;
    let users_info: Vec<UserInfo> = users.into_iter().map(UserInfo::from).collect();
    Ok(Json(serde_json::json!({ "code": 0, "data": users_info })))
}

// ---- 用户管理 CRUD 请求体 ----

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub role: Option<String>,
    pub role_id: Option<String>,
    pub department_id: Option<String>,
    pub mobile: Option<String>,
    pub employee_no: Option<String>,
    pub position: Option<String>,
    pub manager_id: Option<String>,
    pub im_account: Option<String>,
    pub employment_status: Option<String>,
    pub leave_date: Option<String>,
    pub remark: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserRequest {
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub role: Option<String>,
    pub role_id: Option<String>,
    pub department_id: Option<String>,
    pub mobile: Option<String>,
    pub employee_no: Option<String>,
    pub position: Option<String>,
    pub manager_id: Option<String>,
    pub im_account: Option<String>,
    pub employment_status: Option<String>,
    pub leave_date: Option<String>,
    pub remark: Option<String>,
    pub enabled: Option<bool>,
}

/// 手机号格式校验：中国大陆 11 位手机号（1 开头）。
fn validate_mobile(m: &str) -> Result<(), AppError> {
    let s = m.trim();
    if s.is_empty() {
        return Err(AppError::bad("手机号不能为空"));
    }
    if s.len() != 11 || !s.starts_with('1') || !s.chars().all(|c| c.is_ascii_digit()) {
        return Err(AppError::bad("手机号格式不正确，应为 1 开头的 11 位数字"));
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct ToggleEnableRequest {
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub password: String,
}

/// 创建用户（仅 admin）。
async fn create_user(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Json(req_body): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    auth::require_permission(&auth, "user:create")?;
    crate::license_routes::require_active_license(&state.db).await?;
    let ip = audit::extract_ip(&headers, Some(addr));

    let username = req_body.username.trim().to_string();
    if username.is_empty() {
        return Err(AppError::bad("用户名不能为空"));
    }
    if req_body.password.len() < 6 {
        return Err(AppError::bad("密码至少 6 位"));
    }
    // 角色解析：优先 role_id（查角色表），其次 role 字符串，默认 viewer
    let (role_str, role_id): (String, Option<String>) = if let Some(rid) = req_body.role_id.as_deref().filter(|s| !s.is_empty()) {
        let r = db::find_role_by_id(&state.db, rid)
            .await?
            .ok_or_else(|| AppError::bad("指定的角色不存在"))?;
        if !r.is_enabled() {
            return Err(AppError::bad("角色已禁用，无法分配"));
        }
        (r.name, Some(r.id))
    } else if let Some(r) = req_body.role.as_deref().filter(|s| !s.is_empty()) {
        (r.to_string(), None)
    } else {
        ("viewer".to_string(), None)
    };
    let role = auth::Role::parse(&role_str).unwrap_or(auth::Role::Viewer);
    let display_name = req_body.display_name.as_deref().unwrap_or("").to_string();
    let email = req_body.email.as_deref().unwrap_or("").to_string();
    let department_id = req_body.department_id.as_deref().filter(|s| !s.is_empty());
    let enabled = req_body.enabled.unwrap_or(true);

    // 手机号：新建必填且必须合规（短信告警的落点）
    let mobile = req_body.mobile.as_deref().unwrap_or("").trim().to_string();
    validate_mobile(&mobile)?;

    // 手机号唯一：不允许与其他用户重复（用户名与手机号均为身份标识）
    if let Some(owner) = db::find_user_by_mobile(&state.db, &mobile, None).await? {
        return Err(AppError {
            status: StatusCode::CONFLICT,
            code: 409,
            message: format!("手机号 {} 已被用户 '{}' 使用", mobile, owner),
        });
    }

    // 工号唯一（可选字段，填了才校验）
    let employee_no = req_body
        .employee_no
        .as_deref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    if let Some(no) = employee_no.as_deref() {
        if let Some(owner) = db::find_user_by_employee_no(&state.db, no, None).await? {
            return Err(AppError {
                status: StatusCode::CONFLICT,
                code: 409,
                message: format!("工号 {} 已被用户 '{}' 使用", no, owner),
            });
        }
    }

    // 直属上级：必须指向已存在的用户
    let manager_id = req_body.manager_id.as_deref().filter(|s| !s.is_empty());
    if let Some(mid) = manager_id {
        if db::find_user_by_id(&state.db, mid).await?.is_none() {
            return Err(AppError::bad("指定的直属上级不存在"));
        }
    }

    // 在职状态：仅允许 active / left
    let employment_status = req_body
        .employment_status
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or("active");
    if employment_status != "active" && employment_status != "left" {
        return Err(AppError::bad("在职状态仅支持 active 或 left"));
    }

    let profile = db::UserProfileFields {
        mobile: Some(mobile),
        employee_no,
        position: req_body
            .position
            .as_deref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        manager_id: manager_id.map(|s| s.to_string()),
        im_account: req_body
            .im_account
            .as_deref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        employment_status: Some(employment_status.to_string()),
        leave_date: req_body
            .leave_date
            .as_deref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        remark: req_body
            .remark
            .as_deref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
    };

    if db::count_by_username(&state.db, &username).await? > 0 {
        return Err(AppError {
            status: StatusCode::CONFLICT,
            code: 409,
            message: format!("用户名 '{}' 已存在", username),
        });
    }

    let hash = auth::hash_password(&req_body.password)?;
    let id = db::create_user(
        &state.db,
        &username,
        &display_name,
        &email,
        &hash,
        role.as_str(),
        role_id.as_deref(),
        department_id,
        enabled,
        &profile,
    )
    .await?;
    let user = db::find_user_by_id(&state.db, &id)
        .await?
        .ok_or_else(|| AppError::internal("创建后回查失败"))?;

    let detail = serde_json::json!({"username": username, "role": role.as_str()});
    let _ = audit::log_async(
        &state.db,
        &auth,
        "create",
        "user",
        &id,
        Some(&detail),
        &ip,
        "success",
    )
    .await;

    tracing::info!(username = %username, by = %auth.0.sub, "user created");
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "code": 0, "data": UserInfo::from(user) })),
    ))
}

/// 编辑用户可变字段（仅 admin）。用户名不可改。
async fn update_user(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Path(id): Path<String>,
    Json(req_body): Json<UpdateUserRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "user:update")?;
    crate::license_routes::require_active_license(&state.db).await?;
    let ip = audit::extract_ip(&headers, Some(addr));

    let existing = db::find_user_by_id(&state.db, &id)
        .await?
        .ok_or_else(|| AppError::not_found("用户不存在"))?;

    let enabled = req_body.enabled.unwrap_or(existing.is_enabled());
    // 部门：有值就用新的，None 保留原值（须在 display_name/email move 之前读取）
    let department_id: Option<&str> = match &req_body.department_id {
        Some(d) => if d.is_empty() { None } else { Some(d.as_str()) },
        None => existing.department_id.as_deref(),
    };
    // 角色解析：有 role_id 就查角色表，否则保留原值
    let (role_str, role_id): (String, Option<String>) = if let Some(rid) = req_body.role_id.as_deref().filter(|s| !s.is_empty()) {
        let r = db::find_role_by_id(&state.db, rid)
            .await?
            .ok_or_else(|| AppError::bad("角色不存在"))?;
        if !r.is_enabled() {
            return Err(AppError::bad("角色已禁用，无法分配"));
        }
        (r.name, Some(r.id))
    } else {
        (existing.role.clone(), existing.role_id.clone())
    };
    let display_name = req_body.display_name.unwrap_or(existing.display_name);
    let email = req_body.email.unwrap_or(existing.email);

    // 手机号：传了就校验格式；不允许把已有号码清空。
    // 存量用户尚未补录时（原本为空），允许继续为空，避免无法编辑其他字段。
    let mobile: Option<String> = match req_body.mobile.as_deref() {
        Some(s) if !s.trim().is_empty() => {
            validate_mobile(s)?;
            Some(s.trim().to_string())
        }
        Some(_) => {
            if existing.mobile.as_deref().unwrap_or("").is_empty() {
                None
            } else {
                return Err(AppError::bad("手机号不能清空"));
            }
        }
        None => existing.mobile.clone(),
    };

    // 手机号唯一：仅当号码发生变化时才查重（编辑其他字段不受影响）
    if let Some(m) = mobile.as_deref() {
        if existing.mobile.as_deref() != Some(m) {
            if let Some(owner) = db::find_user_by_mobile(&state.db, m, Some(&id)).await? {
                return Err(AppError {
                    status: StatusCode::CONFLICT,
                    code: 409,
                    message: format!("手机号 {} 已被用户 '{}' 使用", m, owner),
                });
            }
        }
    }

    // 工号唯一：仅当发生变化时才查重
    let employee_no: Option<String> = req_body
        .employee_no
        .as_deref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| existing.employee_no.clone());
    if let Some(no) = employee_no.as_deref() {
        if existing.employee_no.as_deref() != Some(no) {
            if let Some(owner) = db::find_user_by_employee_no(&state.db, no, Some(&id)).await? {
                return Err(AppError {
                    status: StatusCode::CONFLICT,
                    code: 409,
                    message: format!("工号 {} 已被用户 '{}' 使用", no, owner),
                });
            }
        }
    }

    // 直属上级：不能指向自己，且必须存在
    let manager_id: Option<String> = match req_body.manager_id.as_deref() {
        Some(s) if !s.trim().is_empty() => {
            if s.trim() == id {
                return Err(AppError::bad("直属上级不能是自己"));
            }
            if db::find_user_by_id(&state.db, s.trim()).await?.is_none() {
                return Err(AppError::bad("指定的直属上级不存在"));
            }
            Some(s.trim().to_string())
        }
        Some(_) => None,
        None => existing.manager_id.clone(),
    };

    let employment_status = req_body
        .employment_status
        .as_deref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| existing.employment_status.clone());
    if employment_status != "active" && employment_status != "left" {
        return Err(AppError::bad("在职状态仅支持 active 或 left"));
    }

    let profile = db::UserProfileFields {
        mobile,
        employee_no,
        position: req_body
            .position
            .as_deref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| existing.position.clone()),
        manager_id,
        im_account: req_body
            .im_account
            .as_deref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| existing.im_account.clone()),
        employment_status: Some(employment_status),
        leave_date: req_body
            .leave_date
            .as_deref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| existing.leave_date.clone()),
        remark: req_body
            .remark
            .as_deref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| existing.remark.clone()),
    };

    db::update_user(
        &state.db,
        &id,
        &display_name,
        &email,
        role_str.as_str(),
        role_id.as_deref(),
        department_id,
        enabled,
        &profile,
    )
    .await?;

    let detail = serde_json::json!({
        "displayName": display_name,
        "email": email,
        "role": role_str.as_str(),
        "enabled": enabled,
    });
    let _ = audit::log_async(
        &state.db,
        &auth,
        "update",
        "user",
        &id,
        Some(&detail),
        &ip,
        "success",
    )
    .await;

    let user = db::find_user_by_id(&state.db, &id)
        .await?
        .ok_or_else(|| AppError::internal("更新后回查失败"))?;
    tracing::info!(user_id = %id, by = %auth.0.sub, "user updated");
    Ok(Json(serde_json::json!({ "code": 0, "data": UserInfo::from(user) })))
}

/// 删除用户（仅 admin）。物理删除，并级联清理该用户的成员关系/通知/令牌等关联数据。
///
/// 安全约束：
/// - 不能删除自己
/// - 不能删除最后一个启用的管理员
async fn delete_user(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "user:delete")?;
    crate::license_routes::require_active_license(&state.db).await?;
    let ip = audit::extract_ip(&headers, Some(addr));

    if id == auth.0.uid {
        return Err(AppError::bad("不能删除当前登录的账号"));
    }

    let target = db::find_user_by_id(&state.db, &id)
        .await?
        .ok_or_else(|| AppError::bad("用户不存在或已被删除"))?;

    // 保护最后一个启用管理员，避免出现无管理员的孤儿系统
    if target.role == "admin" && target.enabled == 1 {
        let admins = db::count_enabled_admins(&state.db).await.unwrap_or(1);
        if admins <= 1 {
            return Err(AppError::bad("至少需保留一个启用的管理员账号"));
        }
    }

    let removed = db::delete_user(&state.db, &id).await?;
    if !removed {
        return Err(AppError::bad("用户不存在或已被删除"));
    }

    let _ = audit::log_async(
        &state.db,
        &auth,
        "delete",
        "user",
        &id,
        Some(&serde_json::json!({ "username": target.username, "displayName": target.display_name })),
        &ip,
        "success",
    )
    .await;

    tracing::info!(user_id = %id, username = %target.username, by = %auth.0.sub, "user deleted");
    Ok(Json(serde_json::json!({ "code": 0, "message": "删除成功" })))
}

/// 启用/禁用用户（仅 admin）。
async fn toggle_enable(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Path(id): Path<String>,
    Json(req_body): Json<ToggleEnableRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "user:toggle_enable")?;
    crate::license_routes::require_active_license(&state.db).await?;
    let ip = audit::extract_ip(&headers, Some(addr));
    db::update_enabled(&state.db, &id, req_body.enabled).await?;

    let _ = audit::log_async(
        &state.db,
        &auth,
        if req_body.enabled { "enable" } else { "disable" },
        "user",
        &id,
        None,
        &ip,
        "success",
    )
    .await;

    tracing::info!(user_id = %id, enabled = req_body.enabled, by = %auth.0.sub, "user enabled toggled");
    Ok(Json(serde_json::json!({ "code": 0, "message": "ok" })))
}

/// 管理员重置用户密码（仅 admin）。
async fn reset_password(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: auth::AuthUser,
    Path(id): Path<String>,
    Json(req_body): Json<ResetPasswordRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "user:reset_password")?;
    crate::license_routes::require_active_license(&state.db).await?;
    let ip = audit::extract_ip(&headers, Some(addr));
    if req_body.password.len() < 6 {
        return Err(AppError::bad("密码至少 6 位"));
    }
    let hash = auth::hash_password(&req_body.password)?;
    db::update_password(&state.db, &id, &hash).await?;

    let _ = audit::log_async(
        &state.db,
        &auth,
        "reset_password",
        "user",
        &id,
        None,
        &ip,
        "success",
    )
    .await;

    tracing::info!(user_id = %id, by = %auth.0.sub, "password reset");
    Ok(Json(serde_json::json!({ "code": 0, "message": "ok" })))
}