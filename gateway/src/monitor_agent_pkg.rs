//! Agent 压缩包库与 Proxy Group → conf 模板映射。

use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{DefaultBodyLimit, Multipart, Path, State};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::Row;
use uuid::Uuid;

use crate::auth;
use crate::error::AppError;
use crate::routes::AppState;

const MAX_PACKAGE_BYTES: usize = 512 * 1024 * 1024; // 512MB
const DEFAULT_INSTALL_PATH: &str = "/opt/ITM/IIM/zabbix_agentd";

pub fn routes() -> Router<Arc<AppState>> {
    // Axum 默认 body 仅 2MB；Agent 压缩包远大于此，需单独放宽，否则 multer 报 multipart 解析失败。
    Router::new()
        .route("/api/monitor/agent-packages", get(list_packages).post(upload_package))
        .route(
            "/api/monitor/agent-packages/:id",
            get(get_package).put(update_package).delete(delete_package),
        )
        .route(
            "/api/monitor/proxy-conf-maps",
            get(list_conf_maps).post(upsert_conf_map),
        )
        .route(
            "/api/monitor/proxy-conf-maps/:id",
            delete(delete_conf_map),
        )
        .route(
            "/api/monitor/ssh-jumps",
            get(list_ssh_jumps).post(upsert_ssh_jump),
        )
        .route("/api/monitor/ssh-jumps/:id", delete(delete_ssh_jump))
        .layer(DefaultBodyLimit::max(MAX_PACKAGE_BYTES + 16 * 1024 * 1024))
}

async fn list_packages(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "monitor:read")?;
    let rows = sqlx::query(
        "SELECT id, name, version, os_arch, agent_kind, filename, stored_path, file_size, sha256, \
                description, enabled, created_by, created_at, updated_at \
         FROM monitor_agent_packages ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await?;
    let items: Vec<Value> = rows.iter().map(package_json).collect();
    Ok(Json(json!({ "code": 0, "data": items })))
}

async fn get_package(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "monitor:read")?;
    let row = sqlx::query(
        "SELECT id, name, version, os_arch, agent_kind, filename, stored_path, file_size, sha256, \
                description, enabled, created_by, created_at, updated_at \
         FROM monitor_agent_packages WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::not_found("Agent 包不存在"))?;
    Ok(Json(json!({ "code": 0, "data": package_json(&row) })))
}

async fn upload_package(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    mut multipart: Multipart,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "monitor:agent")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let mut name = String::new();
    let mut version = String::new();
    let mut os_arch = String::new();
    let mut agent_kind = String::from("agentd");
    let mut description = String::new();
    let mut filename = String::new();
    let mut data: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::bad(&format!("读取上传失败: {e}")))?
    {
        let field_name = field.name().unwrap_or("").to_string();
        match field_name.as_str() {
            "file" => {
                filename = field
                    .file_name()
                    .unwrap_or("agent-package.bin")
                    .to_string();
                data = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| {
                            AppError::bad(&format!(
                                "读取文件失败（单包上限 {}MB，请确认未手动设置 Content-Type）: {e}",
                                MAX_PACKAGE_BYTES / 1024 / 1024
                            ))
                        })?
                        .to_vec(),
                );
            }
            "name" => name = field.text().await.unwrap_or_default(),
            "version" => version = field.text().await.unwrap_or_default(),
            "osArch" => os_arch = field.text().await.unwrap_or_default(),
            "agentKind" => agent_kind = field.text().await.unwrap_or_default(),
            "description" => description = field.text().await.unwrap_or_default(),
            _ => {}
        }
    }

    let data = data.ok_or_else(|| AppError::bad("缺少 file 字段"))?;
    if data.is_empty() {
        return Err(AppError::bad("文件为空"));
    }
    if data.len() > MAX_PACKAGE_BYTES {
        return Err(AppError::bad("文件过大，上限 512MB"));
    }
    if version.trim().is_empty() {
        return Err(AppError::bad("请填写版本号"));
    }
    if name.trim().is_empty() {
        name = filename.clone();
    }
    let lower = filename.to_ascii_lowercase();
    if !(lower.ends_with(".tar.gz")
        || lower.ends_with(".tgz")
        || lower.ends_with(".tar")
        || lower.ends_with(".zip"))
    {
        return Err(AppError::bad("仅支持 tar.gz / tgz / tar / zip"));
    }

    let id = Uuid::new_v4().to_string();
    let safe_filename = filename
        .replace('/', "_")
        .replace('\\', "_")
        .replace("..", "_");
    let stored_name = format!("{}_{}", id, safe_filename);
    let dir = PathBuf::from("./uploads/agent-packages");
    std::fs::create_dir_all(&dir).map_err(|e| AppError::bad(&format!("创建目录失败: {e}")))?;
    let file_path = dir.join(&stored_name);
    std::fs::write(&file_path, &data).map_err(|e| AppError::bad(&format!("写入失败: {e}")))?;

    let mut hasher = Sha256::new();
    hasher.update(&data);
    let sha256 = hex::encode(hasher.finalize());
    let created_by = auth.username().to_string();

    sqlx::query(
        "INSERT INTO monitor_agent_packages \
         (id, name, version, os_arch, agent_kind, filename, stored_path, file_size, sha256, \
          description, enabled, created_by, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, ?, UTC_TIMESTAMP())",
    )
    .bind(&id)
    .bind(name.trim())
    .bind(version.trim())
    .bind(empty_to_none(&os_arch))
    .bind(empty_to_none(&agent_kind).unwrap_or("agentd"))
    .bind(&safe_filename)
    .bind(file_path.to_string_lossy().as_ref())
    .bind(data.len() as i64)
    .bind(&sha256)
    .bind(empty_to_none(&description))
    .bind(&created_by)
    .execute(&state.db)
    .await?;

    let row = sqlx::query(
        "SELECT id, name, version, os_arch, agent_kind, filename, stored_path, file_size, sha256, \
                description, enabled, created_by, created_at, updated_at \
         FROM monitor_agent_packages WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "code": 0, "data": package_json(&row) })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdatePackageBody {
    name: Option<String>,
    version: Option<String>,
    os_arch: Option<String>,
    agent_kind: Option<String>,
    description: Option<String>,
    enabled: Option<bool>,
}

async fn update_package(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(id): Path<String>,
    Json(body): Json<UpdatePackageBody>,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "monitor:agent")?;
    let exists: Option<(String,)> =
        sqlx::query_as("SELECT id FROM monitor_agent_packages WHERE id = ?")
            .bind(&id)
            .fetch_optional(&state.db)
            .await?;
    if exists.is_none() {
        return Err(AppError::not_found("Agent 包不存在"));
    }
    sqlx::query(
        "UPDATE monitor_agent_packages SET \
            name = COALESCE(?, name), \
            version = COALESCE(?, version), \
            os_arch = COALESCE(?, os_arch), \
            agent_kind = COALESCE(?, agent_kind), \
            description = COALESCE(?, description), \
            enabled = COALESCE(?, enabled), \
            updated_at = UTC_TIMESTAMP() \
         WHERE id = ?",
    )
    .bind(body.name.as_deref().map(str::trim).filter(|s| !s.is_empty()))
    .bind(body.version.as_deref().map(str::trim).filter(|s| !s.is_empty()))
    .bind(body.os_arch.as_deref())
    .bind(body.agent_kind.as_deref())
    .bind(body.description.as_deref())
    .bind(body.enabled.map(|v| if v { 1i8 } else { 0 }))
    .bind(&id)
    .execute(&state.db)
    .await?;
    get_package(State(state), auth, Path(id)).await
}

async fn delete_package(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "monitor:agent")?;
    let row = sqlx::query("SELECT stored_path FROM monitor_agent_packages WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::not_found("Agent 包不存在"))?;
    let path: String = row.try_get("stored_path").unwrap_or_default();
    sqlx::query("DELETE FROM monitor_agent_packages WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;
    if !path.is_empty() {
        let _ = std::fs::remove_file(&path);
    }
    Ok(Json(json!({ "code": 0, "data": { "id": id } })))
}

async fn list_conf_maps(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "monitor:read")?;
    let rows = sqlx::query(
        "SELECT id, instance_code, proxy_group_id, conf_suffix, jump_proxy_id, remark, created_at, updated_at \
         FROM monitor_proxy_conf_maps ORDER BY instance_code, conf_suffix",
    )
    .fetch_all(&state.db)
    .await?;
    let items: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "id": r.try_get::<i64, _>("id").unwrap_or(0),
                "instanceCode": r.try_get::<String, _>("instance_code").unwrap_or_default(),
                "proxyGroupId": r.try_get::<String, _>("proxy_group_id").unwrap_or_default(),
                "confSuffix": r.try_get::<String, _>("conf_suffix").unwrap_or_default(),
                "jumpProxyId": r.try_get::<Option<String>, _>("jump_proxy_id").unwrap_or(None),
                "remark": r.try_get::<Option<String>, _>("remark").unwrap_or(None),
                "createdAt": r.try_get::<Option<chrono::NaiveDateTime>, _>("created_at")
                    .ok()
                    .flatten()
                    .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
            })
        })
        .collect();
    Ok(Json(json!({ "code": 0, "data": items })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpsertConfMapBody {
    instance_code: String,
    proxy_group_id: String,
    conf_suffix: String,
    jump_proxy_id: Option<String>,
    remark: Option<String>,
}

async fn upsert_conf_map(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Json(body): Json<UpsertConfMapBody>,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "monitor:agent")?;
    let instance = body.instance_code.trim();
    let group = body.proxy_group_id.trim();
    let suffix = body.conf_suffix.trim();
    let jump_proxy = body
        .jump_proxy_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if instance.is_empty() || group.is_empty() || suffix.is_empty() {
        return Err(AppError::bad("实例、Proxy Group、conf 后缀均不能为空"));
    }
    if !group.chars().all(|c| c.is_ascii_digit()) {
        return Err(AppError::bad("Proxy Group ID 无效"));
    }
    if let Some(jp) = jump_proxy {
        if !jp.chars().all(|c| c.is_ascii_digit()) {
            return Err(AppError::bad("跳板 Proxy ID 无效"));
        }
    }
    if suffix.contains('/') || suffix.contains('\\') || suffix.contains("..") {
        return Err(AppError::bad("conf 后缀无效"));
    }
    sqlx::query(
        "INSERT INTO monitor_proxy_conf_maps \
         (instance_code, proxy_group_id, conf_suffix, jump_proxy_id, remark, created_at) \
         VALUES (?, ?, ?, ?, ?, UTC_TIMESTAMP()) \
         ON DUPLICATE KEY UPDATE conf_suffix = VALUES(conf_suffix), \
         jump_proxy_id = VALUES(jump_proxy_id), remark = VALUES(remark), \
         updated_at = UTC_TIMESTAMP()",
    )
    .bind(instance)
    .bind(group)
    .bind(suffix)
    .bind(jump_proxy)
    .bind(body.remark.as_deref())
    .execute(&state.db)
    .await?;
    list_conf_maps(State(state), auth).await
}

async fn delete_conf_map(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "monitor:agent")?;
    let res = sqlx::query("DELETE FROM monitor_proxy_conf_maps WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::not_found("映射不存在"));
    }
    Ok(Json(json!({ "code": 0, "data": { "id": id } })))
}

async fn list_ssh_jumps(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "monitor:read")?;
    let rows = sqlx::query(
        "SELECT j.id, j.instance_code, j.proxy_id, j.proxy_name, j.jump_host, j.jump_port, \
                j.credential_id, j.enabled, j.remark, j.created_at, c.name AS credential_name \
         FROM monitor_ssh_jumps j \
         LEFT JOIN ssh_credentials c ON c.id = j.credential_id \
         ORDER BY j.instance_code, j.proxy_id",
    )
    .fetch_all(&state.db)
    .await?;
    let items: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "id": r.try_get::<i64, _>("id").unwrap_or(0),
                "instanceCode": r.try_get::<String, _>("instance_code").unwrap_or_default(),
                "proxyId": r.try_get::<String, _>("proxy_id").unwrap_or_default(),
                "proxyName": r.try_get::<Option<String>, _>("proxy_name").unwrap_or(None),
                "jumpHost": r.try_get::<String, _>("jump_host").unwrap_or_default(),
                "jumpPort": r.try_get::<i32, _>("jump_port").unwrap_or(22),
                "credentialId": r.try_get::<i64, _>("credential_id").unwrap_or(0),
                "credentialName": r.try_get::<Option<String>, _>("credential_name").unwrap_or(None),
                "enabled": r.try_get::<i8, _>("enabled").unwrap_or(1) == 1,
                "remark": r.try_get::<Option<String>, _>("remark").unwrap_or(None),
                "createdAt": r.try_get::<Option<chrono::NaiveDateTime>, _>("created_at")
                    .ok()
                    .flatten()
                    .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
            })
        })
        .collect();
    Ok(Json(json!({ "code": 0, "data": items })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpsertSshJumpBody {
    instance_code: String,
    proxy_id: String,
    proxy_name: Option<String>,
    jump_host: String,
    jump_port: Option<i32>,
    credential_id: i64,
    enabled: Option<bool>,
    remark: Option<String>,
}

async fn upsert_ssh_jump(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Json(body): Json<UpsertSshJumpBody>,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "monitor:agent")?;
    let instance = body.instance_code.trim();
    let proxy_id = body.proxy_id.trim();
    let host = body.jump_host.trim();
    if instance.is_empty() || proxy_id.is_empty() || host.is_empty() {
        return Err(AppError::bad("实例、Proxy ID、跳板地址均不能为空"));
    }
    if !proxy_id.chars().all(|c| c.is_ascii_digit()) {
        return Err(AppError::bad("Proxy ID 无效"));
    }
    if body.credential_id <= 0 {
        return Err(AppError::bad("请选择跳板 SSH 凭据"));
    }
    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ssh_credentials WHERE id = ?")
        .bind(body.credential_id)
        .fetch_one(&state.db)
        .await?;
    if exists == 0 {
        return Err(AppError::bad("SSH 凭据不存在"));
    }
    let port = body.jump_port.unwrap_or(22).max(1);
    let enabled = if body.enabled.unwrap_or(true) { 1 } else { 0 };
    sqlx::query(
        "INSERT INTO monitor_ssh_jumps \
         (instance_code, proxy_id, proxy_name, jump_host, jump_port, credential_id, enabled, remark, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, UTC_TIMESTAMP()) \
         ON DUPLICATE KEY UPDATE proxy_name = VALUES(proxy_name), jump_host = VALUES(jump_host), \
         jump_port = VALUES(jump_port), credential_id = VALUES(credential_id), enabled = VALUES(enabled), \
         remark = VALUES(remark), updated_at = UTC_TIMESTAMP()",
    )
    .bind(instance)
    .bind(proxy_id)
    .bind(body.proxy_name.as_deref())
    .bind(host)
    .bind(port)
    .bind(body.credential_id)
    .bind(enabled)
    .bind(body.remark.as_deref())
    .execute(&state.db)
    .await?;
    list_ssh_jumps(State(state), auth).await
}

async fn delete_ssh_jump(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "monitor:agent")?;
    let res = sqlx::query("DELETE FROM monitor_ssh_jumps WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::not_found("跳板配置不存在"));
    }
    Ok(Json(json!({ "code": 0, "data": { "id": id } })))
}

pub async fn resolve_package_path(
    pool: &sqlx::MySqlPool,
    package_id: &str,
) -> Result<(String, String, Option<String>), AppError> {
    let row = sqlx::query(
        "SELECT filename, stored_path, sha256 FROM monitor_agent_packages \
         WHERE id = ? AND enabled = 1",
    )
    .bind(package_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::bad("Agent 包不存在或已禁用"))?;
    let filename: String = row.try_get("filename").unwrap_or_default();
    let path: String = row.try_get("stored_path").unwrap_or_default();
    let sha256: Option<String> = row.try_get("sha256").ok().flatten();
    if path.is_empty() || !std::path::Path::new(&path).is_file() {
        return Err(AppError::bad("Agent 包文件缺失"));
    }
    Ok((filename, path, sha256))
}

/// 返回包版本号，供升级任务写入 targetVersion。
pub async fn resolve_package_version(
    pool: &sqlx::MySqlPool,
    package_id: &str,
) -> Result<String, AppError> {
    let version: Option<String> = sqlx::query_scalar(
        "SELECT version FROM monitor_agent_packages WHERE id = ? AND enabled = 1",
    )
    .bind(package_id)
    .fetch_optional(pool)
    .await?;
    version
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::bad("Agent 包不存在或已禁用"))
}

pub async fn lookup_conf_suffix(
    pool: &sqlx::MySqlPool,
    instance_code: &str,
    proxy_group_id: &str,
) -> Result<Option<String>, AppError> {
    let row = sqlx::query(
        "SELECT conf_suffix FROM monitor_proxy_conf_maps \
         WHERE instance_code = ? AND proxy_group_id = ?",
    )
    .bind(instance_code)
    .bind(proxy_group_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.and_then(|r| r.try_get::<String, _>("conf_suffix").ok()))
}

pub fn default_install_path() -> &'static str {
    DEFAULT_INSTALL_PATH
}

fn package_json(r: &sqlx::mysql::MySqlRow) -> Value {
    json!({
        "id": r.try_get::<String, _>("id").unwrap_or_default(),
        "name": r.try_get::<String, _>("name").unwrap_or_default(),
        "version": r.try_get::<String, _>("version").unwrap_or_default(),
        "osArch": r.try_get::<Option<String>, _>("os_arch").unwrap_or(None),
        "agentKind": r.try_get::<Option<String>, _>("agent_kind").unwrap_or(None),
        "filename": r.try_get::<String, _>("filename").unwrap_or_default(),
        "fileSize": r.try_get::<i64, _>("file_size").unwrap_or(0),
        "sha256": r.try_get::<Option<String>, _>("sha256").unwrap_or(None),
        "description": r.try_get::<Option<String>, _>("description").unwrap_or(None),
        "enabled": r.try_get::<i8, _>("enabled").unwrap_or(0) == 1,
        "createdBy": r.try_get::<Option<String>, _>("created_by").unwrap_or(None),
        "createdAt": r.try_get::<Option<chrono::NaiveDateTime>, _>("created_at")
            .ok()
            .flatten()
            .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
        "defaultInstallPath": DEFAULT_INSTALL_PATH,
    })
}

fn empty_to_none(s: &str) -> Option<&str> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t)
    }
}
