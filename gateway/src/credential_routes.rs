//! V1.5 SSH 凭据管理 API
//!
//! ## 端点
//!   GET    /api/credentials             凭据列表(分页, 不含密码/私钥明文)   (credential:read)
//!   GET    /api/credentials/:id         单条凭据详情(脱敏)                  (credential:read)
//!   POST   /api/credentials             新建凭据(明文→AES加密→存库)          (credential:create)
//!   PUT    /api/credentials/:id         更新凭据(空字段=不修改)              (credential:create)
//!   DELETE /api/credentials/:id         删除凭据(有作业引用时禁止)           (credential:delete)
//!   GET    /api/credentials/list-all    不分页简易列表(供作业定义下拉选择)    (credential:read)

use std::net::SocketAddr;

use axum::extract::{ConnectInfo, Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use sqlx::Row;

use crate::audit;
use crate::auth;
use crate::crypto;
use crate::error::AppError;
use crate::routes::AppState;

pub fn routes() -> Router<std::sync::Arc<AppState>> {
    Router::new()
        .route(
            "/api/credentials",
            get(list_credentials).post(create_credential),
        )
        .route(
            "/api/credentials/:id",
            get(get_credential)
                .put(update_credential)
                .delete(delete_credential),
        )
        .route("/api/credentials/list-all", get(list_all_simple))
}

// ===== 分页查询参数 =====

#[derive(Deserialize, Default, Clone)]
#[serde(default, rename_all = "camelCase")]
struct PagerQuery {
    page: i64,
    page_size: i64,
    keyword: String,
}
impl PagerQuery {
    fn normalize(&mut self) -> (i64, i64) {
        if self.page < 1 {
            self.page = 1;
        }
        if self.page_size < 1 || self.page_size > 200 {
            self.page_size = 20;
        }
        (self.page, self.page_size)
    }
}

// ===== 请求结构 =====

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateCredentialRequest {
    name: String,
    auth_type: String, // "password" | "key"
    username: String,
    #[serde(default)]
    password: Option<String>,
    #[serde(default)]
    private_key: Option<String>,
    #[serde(default)]
    passphrase: Option<String>,
    #[serde(default)]
    host_key_fingerprint: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateCredentialRequest {
    name: String,
    auth_type: String,
    username: String,
    #[serde(default)]
    password: Option<String>,      // 空=不修改
    #[serde(default)]
    private_key: Option<String>,   // 空=不修改
    #[serde(default)]
    passphrase: Option<String>,    // 空=不修改
    #[serde(default)]
    host_key_fingerprint: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

// ===== 列表 =====

async fn list_credentials(
    State(state): State<std::sync::Arc<AppState>>,
    auth: auth::AuthUser,
    Query(mut query): Query<PagerQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "credential:read")?;
    crate::license_routes::require_active_license(&state.db).await?;
    let (page, page_size) = query.normalize();
    let offset = (page - 1) * page_size;

    let mut sql_where = "WHERE 1=1".to_string();
    if !query.keyword.is_empty() {
        let kw = mysql_like_escape(&query.keyword);
        sql_where.push_str(&format!(
            " AND (name LIKE '%{}%' OR username LIKE '%{}%' OR description LIKE '%{}%')",
            kw, kw, kw
        ));
    }

    let total: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM ssh_credentials {}",
        sql_where
    ))
    .fetch_one(&state.db)
    .await?;

    let rows = sqlx::query(&format!(
        "SELECT id, name, auth_type, username, host_key_fingerprint, description, \
                created_by, created_at, updated_at \
         FROM ssh_credentials {} \
         ORDER BY id DESC LIMIT {} OFFSET {}",
        sql_where, page_size, offset
    ))
    .fetch_all(&state.db)
    .await?;

    let list: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.try_get::<i64, _>("id").unwrap_or(0),
                "name": r.try_get::<String, _>("name").unwrap_or_default(),
                "authType": r.try_get::<String, _>("auth_type").unwrap_or_default(),
                "username": r.try_get::<String, _>("username").unwrap_or_default(),
                "hostKeyFingerprint": r.try_get::<String, _>("host_key_fingerprint").unwrap_or_default(),
                "description": r.try_get::<String, _>("description").unwrap_or_default(),
                "createdBy": r.try_get::<String, _>("created_by").unwrap_or_default(),
                "createdAt": format_dt(&r, "created_at"),
                "updatedAt": format_dt(&r, "updated_at"),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": { "list": list, "total": total, "page": page, "pageSize": page_size }
    })))
}

/// 简易全量列表（供作业定义下拉选择，不分页）
async fn list_all_simple(
    State(state): State<std::sync::Arc<AppState>>,
    auth: auth::AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "credential:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let rows = sqlx::query(
        "SELECT id, name, auth_type, username FROM ssh_credentials ORDER BY id DESC",
    )
    .fetch_all(&state.db)
    .await?;

    let list: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.try_get::<i64, _>("id").unwrap_or(0),
                "name": r.try_get::<String, _>("name").unwrap_or_default(),
                "authType": r.try_get::<String, _>("auth_type").unwrap_or_default(),
                "username": r.try_get::<String, _>("username").unwrap_or_default(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "code": 0, "data": list })))
}

// ===== 详情 =====

async fn get_credential(
    State(state): State<std::sync::Arc<AppState>>,
    auth: auth::AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "credential:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let row = sqlx::query(
        "SELECT id, name, auth_type, username, host_key_fingerprint, description, \
                created_by, created_at, updated_at \
         FROM ssh_credentials WHERE id=?",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?;
    let row = match row {
        Some(r) => r,
        None => return Err(AppError::not_found("凭据不存在")),
    };

    // 脱敏：不返回 password/private_key 明文，只返回是否已设置
    Ok(Json(serde_json::json!({
        "code": 0,
        "data": {
            "id": row.try_get::<i64, _>("id").unwrap_or(0),
            "name": row.try_get::<String, _>("name").unwrap_or_default(),
            "authType": row.try_get::<String, _>("auth_type").unwrap_or_default(),
            "username": row.try_get::<String, _>("username").unwrap_or_default(),
            "hasPassword": !row.try_get::<String, _>("host_key_fingerprint").unwrap_or_default().is_empty(), // 占位
            "hostKeyFingerprint": row.try_get::<String, _>("host_key_fingerprint").unwrap_or_default(),
            "description": row.try_get::<String, _>("description").unwrap_or_default(),
            "createdBy": row.try_get::<String, _>("created_by").unwrap_or_default(),
            "createdAt": format_dt(&row, "created_at"),
            "updatedAt": format_dt(&row, "updated_at"),
        }
    })))
}

// ===== 创建 =====

async fn create_credential(
    State(state): State<std::sync::Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    auth: auth::AuthUser,
    Json(req): Json<CreateCredentialRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "credential:create")?;
    crate::license_routes::require_active_license(&state.db).await?;

    // 校验
    if req.name.trim().is_empty() {
        return Err(AppError::bad("凭据名称不能为空"));
    }
    if req.username.trim().is_empty() {
        return Err(AppError::bad("SSH 用户名不能为空"));
    }
    let auth_type = req.auth_type.as_str();
    if auth_type != "password" && auth_type != "key" {
        return Err(AppError::bad("认证方式必须是 password 或 key"));
    }

    // 加密敏感字段
    let (password_enc, private_key_enc, passphrase_enc) =
        encrypt_credential_fields(&req, auth_type)?;

    let result = sqlx::query(
        "INSERT INTO ssh_credentials \
            (name, auth_type, username, password_enc, private_key_enc, passphrase_enc, \
             host_key_fingerprint, description, created_by) \
         VALUES (?,?,?,?,?,?,?,?,?)",
    )
    .bind(req.name.trim())
    .bind(auth_type)
    .bind(req.username.trim())
    .bind(&password_enc)
    .bind(&private_key_enc)
    .bind(&passphrase_enc)
    .bind(req.host_key_fingerprint.clone().unwrap_or_default())
    .bind(req.description.clone().unwrap_or_default())
    .bind(auth.username())
    .execute(&state.db)
    .await?;
    let new_id = result.last_insert_id() as i64;

    audit::log_async(
        &state.db,
        &auth,
        "create_credential",
        "ssh_credential",
        &new_id.to_string(),
        Some(&serde_json::json!({
            "id": new_id, "name": req.name, "authType": auth_type, "username": req.username,
        })),
        &addr.ip().to_string(),
        "success",
    )
    .await;

    Ok(Json(serde_json::json!({
        "code": 0,
        "message": "凭据创建成功",
        "data": { "id": new_id }
    })))
}

// ===== 更新 =====

async fn update_credential(
    State(state): State<std::sync::Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    auth: auth::AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateCredentialRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "credential:create")?;
    crate::license_routes::require_active_license(&state.db).await?;

    if req.name.trim().is_empty() {
        return Err(AppError::bad("凭据名称不能为空"));
    }
    let auth_type = req.auth_type.as_str();
    if auth_type != "password" && auth_type != "key" {
        return Err(AppError::bad("认证方式必须是 password 或 key"));
    }

    // 检查凭据是否存在
    let exists: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM ssh_credentials WHERE id=?")
            .bind(id)
            .fetch_one(&state.db)
            .await?;
    if exists == 0 {
        return Err(AppError::not_found("凭据不存在"));
    }

    // 加密敏感字段（空=保持原值）
    let (password_enc, private_key_enc, passphrase_enc) = {
        let mut pw = String::new();
        let mut pk: Option<String> = None;
        let mut pp = String::new();

        if let Some(ref p) = req.password {
            if !p.is_empty() {
                pw = crypto::encrypt(p)
                    .map_err(|e| AppError::internal(&format!("密码加密失败: {}", e)))?;
            }
        }
        if let Some(ref k) = req.private_key {
            if !k.is_empty() {
                let enc = crypto::encrypt(k)
                    .map_err(|e| AppError::internal(&format!("私钥加密失败: {}", e)))?;
                pk = Some(enc);
            }
        }
        if let Some(ref p) = req.passphrase {
            if !p.is_empty() {
                pp = crypto::encrypt(p)
                    .map_err(|e| AppError::internal(&format!("口令加密失败: {}", e)))?;
            }
        }
        (pw, pk, pp)
    };

    // 动态构造 UPDATE：空字段保持原值
    // 使用 CASE WHEN 处理空字符串（空=不更新）
    sqlx::query(
        "UPDATE ssh_credentials SET \
            name = ?, \
            auth_type = ?, \
            username = ?, \
            password_enc = CASE WHEN ? = '' THEN password_enc ELSE ? END, \
            private_key_enc = CASE WHEN ? IS NULL THEN private_key_enc ELSE ? END, \
            passphrase_enc = CASE WHEN ? = '' THEN passphrase_enc ELSE ? END, \
            host_key_fingerprint = ?, \
            description = ?, \
            updated_at = NOW() \
         WHERE id = ?",
    )
    .bind(req.name.trim())
    .bind(auth_type)
    .bind(req.username.trim())
    .bind(&password_enc)
    .bind(&password_enc)
    .bind(&private_key_enc)
    .bind(&private_key_enc)
    .bind(&passphrase_enc)
    .bind(&passphrase_enc)
    .bind(req.host_key_fingerprint.clone().unwrap_or_default())
    .bind(req.description.clone().unwrap_or_default())
    .bind(id)
    .execute(&state.db)
    .await?;

    audit::log_async(
        &state.db,
        &auth,
        "update_credential",
        "ssh_credential",
        &id.to_string(),
        Some(&serde_json::json!({
            "name": req.name, "authType": auth_type, "username": req.username,
        })),
        &addr.ip().to_string(),
        "success",
    )
    .await;

    Ok(Json(serde_json::json!({
        "code": 0,
        "message": "凭据更新成功"
    })))
}

// ===== 删除 =====

async fn delete_credential(
    State(state): State<std::sync::Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    auth: auth::AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "credential:delete")?;
    crate::license_routes::require_active_license(&state.db).await?;

    // 检查是否有作业定义引用
    let ref_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM job_definitions WHERE credential_id = ?",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;
    if ref_count > 0 {
        return Err(AppError::bad(&format!(
            "该凭据被 {} 个作业定义引用，请先解除关联后再删除",
            ref_count
        )));
    }

    sqlx::query("DELETE FROM ssh_credentials WHERE id=?")
        .bind(id)
        .execute(&state.db)
        .await?;

    audit::log_async(
        &state.db,
        &auth,
        "delete_credential",
        "ssh_credential",
        &id.to_string(),
        None,
        &addr.ip().to_string(),
        "success",
    )
    .await;

    Ok(Json(serde_json::json!({
        "code": 0,
        "message": "凭据已删除"
    })))
}

// ===== 工具函数 =====

fn encrypt_credential_fields(
    req: &CreateCredentialRequest,
    auth_type: &str,
) -> Result<(String, Option<String>, String), AppError> {
    let password_enc = if auth_type == "password" {
        let pw = req
            .password
            .as_ref()
            .ok_or_else(|| AppError::bad("密码认证方式必须提供密码"))?;
        if pw.is_empty() {
            return Err(AppError::bad("密码不能为空"));
        }
        crypto::encrypt(pw)
            .map_err(|e| AppError::internal(&format!("密码加密失败: {}", e)))?
    } else {
        String::new()
    };

    let private_key_enc = if auth_type == "key" {
        let pk = req
            .private_key
            .as_ref()
            .ok_or_else(|| AppError::bad("私钥认证方式必须提供私钥"))?;
        if pk.is_empty() {
            return Err(AppError::bad("私钥不能为空"));
        }
        Some(
            crypto::encrypt(pk)
                .map_err(|e| AppError::internal(&format!("私钥加密失败: {}", e)))?,
        )
    } else {
        None
    };

    let passphrase_enc = if let Some(ref p) = req.passphrase {
        if p.is_empty() {
            String::new()
        } else {
            crypto::encrypt(p)
                .map_err(|e| AppError::internal(&format!("口令加密失败: {}", e)))?
        }
    } else {
        String::new()
    };

    Ok((password_enc, private_key_enc, passphrase_enc))
}

fn mysql_like_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn format_dt(row: &sqlx::mysql::MySqlRow, col: &str) -> String {
    row.try_get::<String, _>(col).unwrap_or_else(|_| {
        row.try_get::<chrono::NaiveDateTime, _>(col)
            .ok()
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_default()
    })
}
