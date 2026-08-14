//! 数据库访问层：MySQL 连接池、users 表数据访问、首次启动种子 admin。

use serde::Serialize;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::{Connection, MySqlPool};

use crate::auth::hash_password;
use crate::config::{AuthConfig, DatabaseConfig};

pub type DbPool = MySqlPool;

/// 用户行。role 为冗余字符串（向后兼容），role_id 为外键指向 roles 表。
/// enabled 是 MySQL TINYINT，sqlx 映射为 i8（0/1）。
/// last_login_at 可空（首次登录前为 None）。
/// role_id / department_id 可空（迁移期间或未分配时为 None）。
/// failed_login_attempts / locked_until 用于登录失败锁定策略。
/// password_changed_at 用于密码过期判断（None 视为不过期，兼容旧数据）。
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub email: String,
    pub password_hash: String,
    pub password_changed_at: Option<String>,
    pub role: String,
    pub role_id: Option<String>,
    pub department_id: Option<String>,
    pub enabled: i8,
    pub failed_login_attempts: i32,
    pub locked_until: Option<String>,
    pub last_login_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl User {
    pub fn is_enabled(&self) -> bool {
        self.enabled != 0
    }
}

/// 建立 MySQL 连接池。若库不存在则自动创建（降低首次启动门槛）。
pub async fn connect(cfg: &DatabaseConfig) -> anyhow::Result<DbPool> {
    ensure_database_exists(&cfg.url).await?;

    let pool = MySqlPoolOptions::new()
        .max_connections(cfg.max_connections)
        .min_connections(cfg.min_connections)
        .connect(&cfg.url)
        .await?;
    tracing::info!(url = %cfg.url, "mysql connected");
    Ok(pool)
}

/// 从完整 url 中切出 server-only url 和 db 名。
/// `mysql://user:pass@host:port/dbname?params` → (`mysql://user:pass@host:port`, `dbname`)
fn split_db_url(url: &str) -> (String, String) {
    let no_params = url.split('?').next().unwrap_or(url);
    if let Some(idx) = no_params.rfind('/') {
        let server_url = &no_params[..idx];
        let db_name = &no_params[idx + 1..];
        (server_url.to_string(), db_name.to_string())
    } else {
        (url.to_string(), String::new())
    }
}

/// 库不存在则创建（幂等）。连 server（不带 db 名）执行 CREATE DATABASE IF NOT EXISTS。
async fn ensure_database_exists(url: &str) -> anyhow::Result<()> {
    let (server_url, db_name) = split_db_url(url);
    if db_name.is_empty() {
        return Ok(());
    }
    let mut conn = sqlx::MySqlConnection::connect(&server_url)
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "connect mysql server failed ({}). 检查 DATABASE_URL 与 mysql 是否可达",
                e
            )
        })?;
    let sql = format!(
        "CREATE DATABASE IF NOT EXISTS `{}` CHARACTER SET utf8mb4",
        db_name
    );
    sqlx::query(&sql).execute(&mut conn).await?;
    tracing::info!(db = %db_name, "database ensured");
    Ok(())
}

/// users 表为空时，用 AuthConfig 的 seed 账号创建首个 admin。
pub async fn seed_admin_if_empty(pool: &DbPool, cfg: &AuthConfig) -> anyhow::Result<()> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    if count > 0 {
        tracing::info!(count, "users table not empty, skip seed");
        return Ok(());
    }

    let id = uuid::Uuid::new_v4().to_string();
    let hash = hash_password(&cfg.seed_password)?;
    let now = chrono::Utc::now().to_rfc3339();
    // 内置 admin 角色的固定 id（见 migration 000006）
    let admin_role_id = "00000000-0000-0000-0000-000000000001";
    sqlx::query(
        "INSERT INTO users (id, username, display_name, email, password_hash, password_changed_at, role, role_id, department_id, enabled, created_at, updated_at)
         VALUES (?, ?, '管理员', '', ?, ?, 'admin', ?, NULL, 1, ?, ?)",
    )
    .bind(&id)
    .bind(&cfg.seed_username)
    .bind(&hash)
    .bind(&now)
    .bind(admin_role_id)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    tracing::info!(username = %cfg.seed_username, "seed admin user created");
    Ok(())
}

pub async fn find_user_by_username(pool: &DbPool, username: &str) -> anyhow::Result<Option<User>> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?")
        .bind(username)
        .fetch_optional(pool)
        .await?;
    Ok(user)
}

pub async fn find_user_by_id(pool: &DbPool, id: &str) -> anyhow::Result<Option<User>> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(user)
}

pub async fn list_all_users(pool: &DbPool) -> anyhow::Result<Vec<User>> {
    let users = sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY username")
        .fetch_all(pool)
        .await?;
    Ok(users)
}

/// 创建用户。返回新建行的 id。
/// 调用方负责：username 唯一性（先 count_by_username 校验）、password 已 hash、role 合法。
/// role 为角色 name（冗余字段，便于快速判断），role_id 为外键。
pub async fn create_user(
    pool: &DbPool,
    username: &str,
    display_name: &str,
    email: &str,
    password_hash: &str,
    role: &str,
    role_id: Option<&str>,
    department_id: Option<&str>,
    enabled: bool,
) -> anyhow::Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO users (id, username, display_name, email, password_hash, password_changed_at, role, role_id, department_id, enabled, last_login_at, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, ?, ?)",
    )
    .bind(&id)
    .bind(username)
    .bind(display_name)
    .bind(email)
    .bind(password_hash)
    .bind(&now)
    .bind(role)
    .bind(role_id)
    .bind(department_id)
    .bind(if enabled { 1_i8 } else { 0_i8 })
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(id)
}

/// 更新用户可变字段（不含密码、不含用户名）。用户名不可改（作为外键引用键）。
pub async fn update_user(
    pool: &DbPool,
    id: &str,
    display_name: &str,
    email: &str,
    role: &str,
    role_id: Option<&str>,
    department_id: Option<&str>,
    enabled: bool,
) -> anyhow::Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let affected = sqlx::query(
        "UPDATE users SET display_name = ?, email = ?, role = ?, role_id = ?, department_id = ?, enabled = ?, updated_at = ? WHERE id = ?",
    )
    .bind(display_name)
    .bind(email)
    .bind(role)
    .bind(role_id)
    .bind(department_id)
    .bind(if enabled { 1_i8 } else { 0_i8 })
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?
    .rows_affected();
    if affected == 0 {
        anyhow::bail!("user not found: {}", id);
    }
    Ok(())
}

/// 仅更新启用状态（独立端点，避免与 PUT 的全量更新耦合）。
pub async fn update_enabled(pool: &DbPool, id: &str, enabled: bool) -> anyhow::Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let affected = sqlx::query("UPDATE users SET enabled = ?, updated_at = ? WHERE id = ?")
        .bind(if enabled { 1_i8 } else { 0_i8 })
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected();
    if affected == 0 {
        anyhow::bail!("user not found: {}", id);
    }
    Ok(())
}

/// 管理员重置密码：直接写入新 hash，并刷新 password_changed_at。
pub async fn update_password(pool: &DbPool, id: &str, password_hash: &str) -> anyhow::Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let affected = sqlx::query("UPDATE users SET password_hash = ?, password_changed_at = ?, updated_at = ? WHERE id = ?")
        .bind(password_hash)
        .bind(&now)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected();
    if affected == 0 {
        anyhow::bail!("user not found: {}", id);
    }
    Ok(())
}

/// 登录成功时更新 last_login_at。
pub async fn update_last_login(pool: &DbPool, id: &str) -> anyhow::Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("UPDATE users SET last_login_at = ? WHERE id = ?")
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// 用户名计数，用于创建前唯一性校验。返回 1 表示已存在。
pub async fn count_by_username(pool: &DbPool, username: &str) -> anyhow::Result<i64> {
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE username = ?")
        .bind(username)
        .fetch_one(pool)
        .await?;
    Ok(n)
}

// ============ 审计日志 ============

/// 审计日志行。
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLog {
    pub id: i64,
    pub actor_username: String,
    pub action: String,
    pub target_type: String,
    pub target_id: String,
    pub detail: Option<String>,
    pub ip: String,
    pub status: String,
    pub created_at: String,
}

/// 插入一条审计日志。detail 为可选 JSON，失败不阻塞主流程（调用方用 let _ = 忽略错误）。
pub async fn insert_audit_log(
        pool: &DbPool,
        actor_username: &str,
        action: &str,
        target_type: &str,
        target_id: &str,
        detail: Option<&serde_json::Value>,
        ip: &str,
        status: &str,
    ) -> anyhow::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let detail_str: Option<String> = detail.map(|v| v.to_string());
        sqlx::query(
            "INSERT INTO audit_logs (actor_username, action, target_type, target_id, detail, ip, status, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(actor_username)
        .bind(action)
        .bind(target_type)
        .bind(target_id)
        .bind(detail_str)
        .bind(ip)
        .bind(status)
        .bind(&now)
        .execute(pool)
        .await?;
        Ok(())
    }

/// 分页查询审计日志。支持按 actor/action/target_type/target_id/status 筛选。
/// 返回 (总条数, 当前页数据)。
pub async fn query_audit_logs(
    pool: &DbPool,
    actor: Option<&str>,
    action: Option<&str>,
    target_type: Option<&str>,
    target_id: Option<&str>,
    status: Option<&str>,
    start_from: Option<&str>,
    page: u64,
    page_size: u64,
) -> anyhow::Result<(i64, Vec<AuditLog>)> {
    let mut conditions = Vec::new();
    let mut args: Vec<String> = Vec::new();

    if let Some(v) = actor {
        conditions.push("actor_username = ?");
        args.push(v.to_string());
    }
    if let Some(v) = action {
        conditions.push("action = ?");
        args.push(v.to_string());
    }
    if let Some(v) = target_type {
        conditions.push("target_type = ?");
        args.push(v.to_string());
    }
    if let Some(v) = target_id {
        conditions.push("target_id = ?");
        args.push(v.to_string());
    }
    if let Some(v) = status {
        conditions.push("status = ?");
        args.push(v.to_string());
    }
    if let Some(v) = start_from {
        conditions.push("created_at >= ?");
        args.push(v.to_string());
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // count
    let count_sql = format!("SELECT COUNT(*) FROM audit_logs {}", where_clause);
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    for arg in &args {
        count_query = count_query.bind(arg);
    }
    let total = count_query.fetch_one(pool).await?;

    // data
    let offset = (page.saturating_sub(1)) * page_size;
    let data_sql = format!(
        "SELECT id, actor_username, action, target_type, target_id, \
                CAST(detail AS CHAR) AS detail, ip, status, created_at \
         FROM audit_logs {} ORDER BY id DESC LIMIT ? OFFSET ?",
        where_clause
    );
    let mut data_query = sqlx::query_as::<_, AuditLog>(&data_sql);
    for arg in &args {
        data_query = data_query.bind(arg);
    }
    data_query = data_query.bind(page_size as i64).bind(offset as i64);
    let rows = data_query.fetch_all(pool).await?;

    Ok((total, rows))
}

// ============ RBAC：角色 / 权限 / 角色权限关联 ============

/// 将 i8 (0/1) 序列化为 JSON boolean，与前端 TypeScript boolean 类型对齐。
/// 数据库 TINYINT 经 sqlx 映射为 i8，但 API 响应需输出 true/false。
fn serialize_bool<S: serde::Serializer>(v: &i8, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_bool(*v != 0)
}

/// 角色行。built_in=1 表示内置三级角色（admin/operator/viewer），不可删除。
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Role {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub description: String,
    #[serde(serialize_with = "serialize_bool")]
    pub enabled: i8,
    #[serde(serialize_with = "serialize_bool")]
    pub built_in: i8,
    pub created_at: String,
    pub updated_at: String,
}

impl Role {
    pub fn is_enabled(&self) -> bool {
        self.enabled != 0
    }
    pub fn is_built_in(&self) -> bool {
        self.built_in != 0
    }
}

/// 权限点行。
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Permission {
    pub id: String,
    pub code: String,
    pub name: String,
    pub module: String,
    pub description: String,
    pub created_at: String,
}

pub async fn list_roles(pool: &DbPool) -> anyhow::Result<Vec<Role>> {
    let roles = sqlx::query_as::<_, Role>("SELECT * FROM roles ORDER BY built_in DESC, name")
        .fetch_all(pool)
        .await?;
    Ok(roles)
}

pub async fn find_role_by_id(pool: &DbPool, id: &str) -> anyhow::Result<Option<Role>> {
    let role = sqlx::query_as::<_, Role>("SELECT * FROM roles WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(role)
}

pub async fn find_role_by_name(pool: &DbPool, name: &str) -> anyhow::Result<Option<Role>> {
    let role = sqlx::query_as::<_, Role>("SELECT * FROM roles WHERE name = ?")
        .bind(name)
        .fetch_optional(pool)
        .await?;
    Ok(role)
}

pub async fn count_role_by_name(pool: &DbPool, name: &str, exclude_id: Option<&str>) -> anyhow::Result<i64> {
    let n: i64 = match exclude_id {
        Some(eid) => sqlx::query_scalar("SELECT COUNT(*) FROM roles WHERE name = ? AND id <> ?")
            .bind(name)
            .bind(eid)
            .fetch_one(pool)
            .await?,
        None => sqlx::query_scalar("SELECT COUNT(*) FROM roles WHERE name = ?")
            .bind(name)
            .fetch_one(pool)
            .await?,
    };
    Ok(n)
}

pub async fn create_role(
    pool: &DbPool,
    name: &str,
    display_name: &str,
    description: &str,
    enabled: bool,
) -> anyhow::Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO roles (id, name, display_name, description, enabled, built_in, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, 0, ?, ?)",
    )
    .bind(&id)
    .bind(name)
    .bind(display_name)
    .bind(description)
    .bind(if enabled { 1_i8 } else { 0_i8 })
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(id)
}

pub async fn update_role(
    pool: &DbPool,
    id: &str,
    display_name: &str,
    description: &str,
    enabled: bool,
) -> anyhow::Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let affected = sqlx::query(
        "UPDATE roles SET display_name = ?, description = ?, enabled = ?, updated_at = ? WHERE id = ?",
    )
    .bind(display_name)
    .bind(description)
    .bind(if enabled { 1_i8 } else { 0_i8 })
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?
    .rows_affected();
    if affected == 0 {
        anyhow::bail!("role not found: {}", id);
    }
    Ok(())
}

/// 删除角色。内置角色不可删（调用方先校验）。有用户引用时也不可删。
pub async fn delete_role(pool: &DbPool, id: &str) -> anyhow::Result<()> {
    let affected = sqlx::query("DELETE FROM roles WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected();
    if affected == 0 {
        anyhow::bail!("role not found: {}", id);
    }
    Ok(())
}

/// 统计引用某角色的用户数（删除前校验）。
pub async fn count_users_by_role(pool: &DbPool, role_id: &str) -> anyhow::Result<i64> {
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role_id = ?")
        .bind(role_id)
        .fetch_one(pool)
        .await?;
    Ok(n)
}

pub async fn list_permissions(pool: &DbPool) -> anyhow::Result<Vec<Permission>> {
    let perms = sqlx::query_as::<_, Permission>("SELECT * FROM permissions ORDER BY module, code")
        .fetch_all(pool)
        .await?;
    Ok(perms)
}

/// 查询某角色已分配的权限点列表。
pub async fn list_permissions_by_role(pool: &DbPool, role_id: &str) -> anyhow::Result<Vec<Permission>> {
    let perms = sqlx::query_as::<_, Permission>(
        "SELECT p.* FROM permissions p
         INNER JOIN role_permissions rp ON rp.permission_id = p.id
         WHERE rp.role_id = ?
         ORDER BY p.module, p.code",
    )
    .bind(role_id)
    .fetch_all(pool)
    .await?;
    Ok(perms)
}

/// 查询某角色的权限 code 列表（用于 JWT claims）。
pub async fn list_permission_codes_by_role(pool: &DbPool, role_id: &str) -> anyhow::Result<Vec<String>> {
    let codes: Vec<String> = sqlx::query_scalar(
        "SELECT p.code FROM permissions p
         INNER JOIN role_permissions rp ON rp.permission_id = p.id
         WHERE rp.role_id = ?",
    )
    .bind(role_id)
    .fetch_all(pool)
    .await?;
    Ok(codes)
}

/// 查询某用户的权限 code 列表（通过其 role_id）。
/// 若 role_id 为空或角色被禁用，返回空列表。
pub async fn list_permission_codes_by_user(pool: &DbPool, role_id: Option<&str>) -> anyhow::Result<Vec<String>> {
    match role_id {
        None => Ok(Vec::new()),
        Some(rid) => {
            let codes: Vec<String> = sqlx::query_scalar(
                "SELECT p.code FROM permissions p
                 INNER JOIN role_permissions rp ON rp.permission_id = p.id
                 INNER JOIN roles r ON r.id = rp.role_id
                 WHERE rp.role_id = ? AND r.enabled = 1",
            )
            .bind(rid)
            .fetch_all(pool)
            .await?;
            Ok(codes)
        }
    }
}

/// 批量设置角色权限（先删后插，事务保证一致性）。
pub async fn set_role_permissions(pool: &DbPool, role_id: &str, permission_ids: &[String]) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM role_permissions WHERE role_id = ?")
        .bind(role_id)
        .execute(&mut *tx)
        .await?;
    for pid in permission_ids {
        sqlx::query("INSERT INTO role_permissions (role_id, permission_id) VALUES (?, ?)")
            .bind(role_id)
            .bind(pid)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

// ============ 部门管理（树形）============

/// 部门行。parent_id 为空表示根部门。
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Department {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub sort_order: i32,
    #[serde(serialize_with = "serialize_bool")]
    pub enabled: i8,
    pub created_at: String,
    pub updated_at: String,
}

impl Department {
    pub fn is_enabled(&self) -> bool {
        self.enabled != 0
    }
}

pub async fn list_departments(pool: &DbPool) -> anyhow::Result<Vec<Department>> {
    let depts = sqlx::query_as::<_, Department>(
        "SELECT * FROM departments ORDER BY sort_order, name",
    )
    .fetch_all(pool)
    .await?;
    Ok(depts)
}

pub async fn find_department_by_id(pool: &DbPool, id: &str) -> anyhow::Result<Option<Department>> {
    let dept = sqlx::query_as::<_, Department>("SELECT * FROM departments WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(dept)
}

pub async fn count_departments_by_parent(pool: &DbPool, parent_id: Option<&str>) -> anyhow::Result<i64> {
    let n: i64 = match parent_id {
        Some(pid) => sqlx::query_scalar("SELECT COUNT(*) FROM departments WHERE parent_id = ?")
            .bind(pid)
            .fetch_one(pool)
            .await?,
        None => sqlx::query_scalar("SELECT COUNT(*) FROM departments WHERE parent_id IS NULL")
            .fetch_one(pool)
            .await?,
    };
    Ok(n)
}

pub async fn count_users_by_department(pool: &DbPool, dept_id: &str) -> anyhow::Result<i64> {
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE department_id = ?")
        .bind(dept_id)
        .fetch_one(pool)
        .await?;
    Ok(n)
}

pub async fn create_department(
    pool: &DbPool,
    name: &str,
    parent_id: Option<&str>,
    sort_order: i32,
    enabled: bool,
) -> anyhow::Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO departments (id, name, parent_id, sort_order, enabled, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(name)
    .bind(parent_id)
    .bind(sort_order)
    .bind(if enabled { 1_i8 } else { 0_i8 })
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(id)
}

pub async fn update_department(
    pool: &DbPool,
    id: &str,
    name: &str,
    parent_id: Option<&str>,
    sort_order: i32,
    enabled: bool,
) -> anyhow::Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let affected = sqlx::query(
        "UPDATE departments SET name = ?, parent_id = ?, sort_order = ?, enabled = ?, updated_at = ? WHERE id = ?",
    )
    .bind(name)
    .bind(parent_id)
    .bind(sort_order)
    .bind(if enabled { 1_i8 } else { 0_i8 })
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?
    .rows_affected();
    if affected == 0 {
        anyhow::bail!("department not found: {}", id);
    }
    Ok(())
}

/// 删除部门。有子部门或用户引用时不可删（调用方校验）。
pub async fn delete_department(pool: &DbPool, id: &str) -> anyhow::Result<()> {
    let affected = sqlx::query("DELETE FROM departments WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected();
    if affected == 0 {
        anyhow::bail!("department not found: {}", id);
    }
    Ok(())
}

// ============ 登录失败锁定 ============

/// 登录失败时递增失败计数。达到 max_attempts 时设置 locked_until = now + lockout_minutes。
/// 返回 Some(locked_until_rfc3339) 表示本次触发锁定，None 表示未触发。
pub async fn increment_failed_login(
    pool: &DbPool,
    user_id: &str,
    max_attempts: i32,
    lockout_minutes: i64,
) -> anyhow::Result<Option<String>> {
    let now = chrono::Utc::now();
    // 1. 递增失败计数
    sqlx::query(
        "UPDATE users SET failed_login_attempts = failed_login_attempts + 1, updated_at = ? WHERE id = ?",
    )
    .bind(now.to_rfc3339())
    .bind(user_id)
    .execute(pool)
    .await?;

    // 2. 查最新计数
    let current: i32 = sqlx::query_scalar("SELECT failed_login_attempts FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_one(pool)
        .await?;

    // 3. 达到阈值则锁定
    if current >= max_attempts {
        let locked_until = now + chrono::Duration::minutes(lockout_minutes);
        let locked_str = locked_until.to_rfc3339();
        sqlx::query("UPDATE users SET locked_until = ?, updated_at = ? WHERE id = ?")
            .bind(&locked_str)
            .bind(now.to_rfc3339())
            .bind(user_id)
            .execute(pool)
            .await?;
        Ok(Some(locked_str))
    } else {
        Ok(None)
    }
}

/// 登录成功后重置失败计数与锁定状态。
pub async fn reset_failed_login(pool: &DbPool, user_id: &str) -> anyhow::Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE users SET failed_login_attempts = 0, locked_until = NULL, updated_at = ? WHERE id = ?",
    )
    .bind(&now)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// 检查用户是否被锁定。返回 Some(locked_until_rfc3339) 表示仍在锁定期内。
pub async fn check_user_locked(pool: &DbPool, user_id: &str) -> anyhow::Result<Option<String>> {
    let row: Option<(Option<String>,)> =
        sqlx::query_as("SELECT locked_until FROM users WHERE id = ?")
            .bind(user_id)
            .fetch_optional(pool)
            .await?;
    match row {
        Some((Some(locked_until),)) => {
            // 解析锁定时间，与当前比较
            if let Ok(lock_time) = chrono::DateTime::parse_from_rfc3339(&locked_until) {
                if chrono::Utc::now() < lock_time.with_timezone(&chrono::Utc) {
                    return Ok(Some(locked_until));
                }
                // 锁定已过期，清理
                let now = chrono::Utc::now().to_rfc3339();
                sqlx::query(
                    "UPDATE users SET failed_login_attempts = 0, locked_until = NULL, updated_at = ? WHERE id = ?",
                )
                .bind(&now)
                .bind(user_id)
                .execute(pool)
                .await?;
                return Ok(None);
            }
            Ok(None)
        }
        _ => Ok(None),
    }
}

// ============ 系统配置 ============

/// 系统配置行（键值对）。
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemSetting {
    pub setting_key: String,
    pub setting_value: String,
    pub description: String,
    pub updated_at: String,
    pub updated_by: String,
}

/// 读取全部系统配置。
pub async fn list_all_settings(pool: &DbPool) -> anyhow::Result<Vec<SystemSetting>> {
    let rows = sqlx::query_as::<_, SystemSetting>(
        "SELECT setting_key, setting_value, description, updated_at, updated_by FROM system_settings ORDER BY setting_key",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// 读取单个配置项的值。不存在返回 None。
pub async fn get_setting(pool: &DbPool, key: &str) -> anyhow::Result<Option<String>> {
    let v: Option<String> = sqlx::query_scalar(
        "SELECT setting_value FROM system_settings WHERE setting_key = ?",
    )
    .bind(key)
    .fetch_optional(pool)
    .await?;
    Ok(v)
}

/// 批量 upsert 配置项（INSERT ... ON DUPLICATE KEY UPDATE）。
pub async fn upsert_settings(
    pool: &DbPool,
    entries: &[(String, String, String)], // (key, value, updated_by)
) -> anyhow::Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let mut tx = pool.begin().await?;
    for (key, value, updated_by) in entries {
        sqlx::query(
            "INSERT INTO system_settings (setting_key, setting_value, description, updated_at, updated_by)
             VALUES (?, ?, '', ?, ?)
             ON DUPLICATE KEY UPDATE setting_value = VALUES(setting_value), updated_at = VALUES(updated_at), updated_by = VALUES(updated_by)",
        )
        .bind(key)
        .bind(value)
        .bind(&now)
        .bind(updated_by)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

// ============ Dashboard 聚合查询 ============

/// 用户总数与启用用户数。返回 (total, enabled)。
pub async fn count_users_summary(pool: &DbPool) -> anyhow::Result<(i64, i64)> {
    // MySQL SUM() 返回 DECIMAL，需 CAST 为 SIGNED 才能与 i64 匹配
    let row: (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*), CAST(COALESCE(SUM(CASE WHEN enabled = 1 THEN 1 ELSE 0 END), 0) AS SIGNED) FROM users",
    )
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// 角色总数。
pub async fn count_roles_total(pool: &DbPool) -> anyhow::Result<i64> {
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM roles")
        .fetch_one(pool)
        .await?;
    Ok(n)
}

/// 部门总数。
pub async fn count_departments_total(pool: &DbPool) -> anyhow::Result<i64> {
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM departments")
        .fetch_one(pool)
        .await?;
    Ok(n)
}

/// 自指定时间以来（含）的审计日志条数。
/// since 为 RFC3339 字符串；为 None 时返回全量条数。
pub async fn count_audit_logs_since(pool: &DbPool, since: Option<&str>) -> anyhow::Result<i64> {
    let n: i64 = match since {
        Some(s) => {
            sqlx::query_scalar("SELECT COUNT(*) FROM audit_logs WHERE created_at >= ?")
                .bind(s)
                .fetch_one(pool)
                .await?
        }
        None => sqlx::query_scalar("SELECT COUNT(*) FROM audit_logs")
            .fetch_one(pool)
            .await?,
    };
    Ok(n)
}

/// 自指定时间以来（含）的登录成功条数。
pub async fn count_login_success_since(pool: &DbPool, since: Option<&str>) -> anyhow::Result<i64> {
    let n: i64 = match since {
        Some(s) => {
            sqlx::query_scalar(
                "SELECT COUNT(*) FROM audit_logs WHERE action = 'login' AND status = 'success' AND created_at >= ?",
            )
            .bind(s)
            .fetch_one(pool)
            .await?
        }
        None => {
            sqlx::query_scalar(
                "SELECT COUNT(*) FROM audit_logs WHERE action = 'login' AND status = 'success'",
            )
            .fetch_one(pool)
            .await?
        }
    };
    Ok(n)
}

/// 最近 N 条审计日志（全局）。按 id 倒序。
pub async fn list_recent_audit_logs(pool: &DbPool, limit: i64) -> anyhow::Result<Vec<AuditLog>> {
    let rows = sqlx::query_as::<_, AuditLog>(
        "SELECT id, actor_username, action, target_type, target_id, \
                CAST(detail AS CHAR) AS detail, ip, status, created_at \
         FROM audit_logs ORDER BY id DESC LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// 某操作人最近 N 条审计日志。按 id 倒序。
pub async fn list_recent_audit_logs_by_actor(
    pool: &DbPool,
    actor: &str,
    limit: i64,
) -> anyhow::Result<Vec<AuditLog>> {
    let rows = sqlx::query_as::<_, AuditLog>(
        "SELECT id, actor_username, action, target_type, target_id, \
                CAST(detail AS CHAR) AS detail, ip, status, created_at \
         FROM audit_logs WHERE actor_username = ? ORDER BY id DESC LIMIT ?",
    )
    .bind(actor)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

// ============ 报表中心查询 ============

/// 报表：登录趋势。返回 [(date_str, success_count, failed_count)]，按日期升序。
/// date_str 格式 YYYY-MM-DD（UTC 日期）。
pub async fn report_login_trend(pool: &DbPool, days: i64) -> anyhow::Result<Vec<(String, i64, i64)>> {
    // DATE_FORMAT 把 DATE 类型转成字符串，避免 sqlx 解码 DATE 到 String 类型不匹配
    let rows: Vec<(String, i64, i64)> = sqlx::query_as(
        "SELECT DATE_FORMAT(created_at, '%Y-%m-%d') AS d, \
                CAST(COALESCE(SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END), 0) AS SIGNED) AS s, \
                CAST(COALESCE(SUM(CASE WHEN status != 'success' THEN 1 ELSE 0 END), 0) AS SIGNED) AS f \
         FROM audit_logs \
         WHERE action = 'login' AND created_at >= ? \
         GROUP BY d ORDER BY d ASC",
    )
    .bind(days_ago_rfc3339(days))
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// 报表：失败登录 TOP 用户。返回 [(username, failed_count, last_failed_at)]，按失败次数倒序。
pub async fn report_login_failed_top(
    pool: &DbPool,
    days: i64,
    limit: i64,
) -> anyhow::Result<Vec<(String, i64, String)>> {
    let rows: Vec<(String, i64, String)> = sqlx::query_as(
        "SELECT actor_username, \
                CAST(COUNT(*) AS SIGNED) AS c, \
                MAX(created_at) AS last_at \
         FROM audit_logs \
         WHERE action = 'login' AND status != 'success' AND created_at >= ? \
         GROUP BY actor_username \
         ORDER BY c DESC, last_at DESC \
         LIMIT ?",
    )
    .bind(days_ago_rfc3339(days))
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// 报表：当前锁定账号列表。返回 UserInfo 列表（locked_until > now）。
pub async fn report_locked_users(pool: &DbPool) -> anyhow::Result<Vec<User>> {
    let now = chrono::Utc::now().to_rfc3339();
    let rows = sqlx::query_as::<_, User>(
        "SELECT id, username, display_name, email, password_hash, password_changed_at, \
                role, role_id, department_id, enabled, failed_login_attempts, locked_until, \
                last_login_at, created_at, updated_at \
         FROM users WHERE locked_until IS NOT NULL AND locked_until > ? \
         ORDER BY locked_until DESC",
    )
    .bind(&now)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// 报表：敏感操作趋势。返回 [(date_str, count)]，按日期升序。
/// 敏感操作 = 删除类 + 权限变更类 + 系统配置类 + 密码相关
pub async fn report_sensitive_ops_trend(
    pool: &DbPool,
    days: i64,
) -> anyhow::Result<Vec<(String, i64)>> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT DATE_FORMAT(created_at, '%Y-%m-%d') AS d, CAST(COUNT(*) AS SIGNED) AS c \
         FROM audit_logs \
         WHERE action IN ('delete_user','disable_user','reset_password','create_role','update_role','delete_role','assign_permission','update_settings','change_password') \
           AND created_at >= ? \
         GROUP BY d ORDER BY d ASC",
    )
    .bind(days_ago_rfc3339(days))
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// 报表：敏感操作 TOP 操作人。返回 [(username, count, last_action_at)]。
pub async fn report_sensitive_ops_top(
    pool: &DbPool,
    days: i64,
    limit: i64,
) -> anyhow::Result<Vec<(String, i64, String)>> {
    let rows: Vec<(String, i64, String)> = sqlx::query_as(
        "SELECT actor_username, \
                CAST(COUNT(*) AS SIGNED) AS c, \
                MAX(created_at) AS last_at \
         FROM audit_logs \
         WHERE action IN ('delete_user','disable_user','reset_password','create_role','update_role','delete_role','assign_permission','update_settings','change_password') \
           AND created_at >= ? \
         GROUP BY actor_username \
         ORDER BY c DESC, last_at DESC \
         LIMIT ?",
    )
    .bind(days_ago_rfc3339(days))
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// 报表：敏感操作明细。返回分页审计日志。
pub async fn report_sensitive_ops_list(
    pool: &DbPool,
    days: i64,
    page: u64,
    page_size: u64,
) -> anyhow::Result<(i64, Vec<AuditLog>)> {
    let since = days_ago_rfc3339(days);
    let actions_in = "('delete_user','disable_user','reset_password','create_role','update_role','delete_role','assign_permission','update_settings','change_password')";

    let total: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM audit_logs WHERE action IN {} AND created_at >= ?",
        actions_in
    ))
    .bind(&since)
    .fetch_one(pool)
    .await?;

    let offset = ((page - 1) * page_size) as i64;
    let rows = sqlx::query_as::<_, AuditLog>(
        &format!(
            "SELECT id, actor_username, action, target_type, target_id, \
                    CAST(detail AS CHAR) AS detail, ip, status, created_at \
             FROM audit_logs \
             WHERE action IN {} AND created_at >= ? \
             ORDER BY id DESC LIMIT ? OFFSET ?",
            actions_in
        ),
    )
    .bind(&since)
    .bind(page_size as i64)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok((total, rows))
}

/// 报表：合规健康度摘要。
/// 返回 (total_users, weak_password_count, expired_password_count, inactive_90d_count)。
/// weak_password_count：密码不满足当前密码策略的用户数（近似：密码长度 < min_length 的用户数）。
///   注意：密码哈希不可逆，这里只能近似估算（用 password_hash 长度或历史标记）。
///   简化实现：直接返回 0，并在前端标注"需人工审计"。
/// expired_password_count：password_changed_at + expiry_days < now 的用户数。
/// inactive_90d_count：last_login_at 为 NULL 或 < 90 天前的用户数。
pub async fn report_compliance_summary(
    pool: &DbPool,
    expiry_days: i64,
) -> anyhow::Result<(i64, i64, i64, i64)> {
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;

    // 密码已过期用户数（expiry_days <= 0 时为 0）
    let expired_count: i64 = if expiry_days > 0 {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM users \
             WHERE password_changed_at IS NOT NULL \
               AND DATE_ADD(password_changed_at, INTERVAL ? DAY) < UTC_TIMESTAMP()",
        )
        .bind(expiry_days)
        .fetch_one(pool)
        .await?
    } else {
        0
    };

    // 90 天未登录用户数（last_login_at 为 NULL 视为从未登录，计入；非空但早于 90 天也计入）
    let inactive_90d: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users \
         WHERE last_login_at IS NULL \
            OR (last_login_at IS NOT NULL AND last_login_at < ?)",
    )
    .bind(days_ago_rfc3339(90))
    .fetch_one(pool)
    .await?;

    // weak_password_count 简化为 0（哈希不可逆，需人工审计）
    Ok((total, 0, expired_count, inactive_90d))
}

/// 报表：长期未登录用户。返回 User 列表（last_login_at 为 NULL 或 > days 天前）。
pub async fn report_inactive_users(pool: &DbPool, days: i64) -> anyhow::Result<Vec<User>> {
    let since = days_ago_rfc3339(days);
    let rows = sqlx::query_as::<_, User>(
        "SELECT id, username, display_name, email, password_hash, password_changed_at, \
                role, role_id, department_id, enabled, failed_login_attempts, locked_until, \
                last_login_at, created_at, updated_at \
         FROM users \
         WHERE last_login_at IS NULL OR last_login_at < ? \
         ORDER BY last_login_at ASC, created_at ASC",
    )
    .bind(&since)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// 报表：角色权限分配统计。返回 [(role_name, user_count)]，按用户数倒序。
pub async fn report_role_assignment(pool: &DbPool) -> anyhow::Result<Vec<(String, i64)>> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT r.name, CAST(COUNT(u.id) AS SIGNED) AS c \
         FROM roles r \
         LEFT JOIN users u ON u.role_id = r.id \
         GROUP BY r.id, r.name \
         ORDER BY c DESC, r.name ASC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// 计算 N 天前的 RFC3339 时间戳（用于 SQL 查询条件）。
fn days_ago_rfc3339(days: i64) -> String {
    (chrono::Utc::now() - chrono::Duration::days(days)).to_rfc3339()
}

// ============ CMDB 配置管理 ============

/// 将 JSON 字符串列序列化为 JSON Value（前端直接拿到对象而非字符串）。
fn serialize_json_str<S: serde::Serializer>(v: &Option<String>, s: S) -> Result<S::Ok, S::Error> {
    match v {
        Some(json_str) => {
            let val: serde_json::Value =
                serde_json::from_str(json_str).unwrap_or(serde_json::Value::Null);
            val.serialize(s)
        }
        None => s.serialize_none(),
    }
}

// ---- CI 模型 ----

/// CI 模型行。
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CiModel {
    pub id: String,
    pub code: String,
    pub name: String,
    pub icon: String,
    pub description: String,
    #[serde(serialize_with = "serialize_bool")]
    pub enabled: i8,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

pub async fn list_ci_models(pool: &DbPool) -> anyhow::Result<Vec<CiModel>> {
    let rows = sqlx::query_as::<_, CiModel>(
        "SELECT id, code, name, icon, description, enabled, sort_order, created_at, updated_at \
         FROM ci_models ORDER BY sort_order, name",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn find_ci_model_by_id(pool: &DbPool, id: &str) -> anyhow::Result<Option<CiModel>> {
    let row = sqlx::query_as::<_, CiModel>(
        "SELECT id, code, name, icon, description, enabled, sort_order, created_at, updated_at \
         FROM ci_models WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// 按 code 查询 CI 模型（用于唯一性校验）。
pub async fn find_ci_model_by_code(pool: &DbPool, code: &str) -> anyhow::Result<Option<CiModel>> {
    let row = sqlx::query_as::<_, CiModel>(
        "SELECT id, code, name, icon, description, enabled, sort_order, created_at, updated_at \
         FROM ci_models WHERE code = ?",
    )
    .bind(code)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// 创建 CI 模型。
pub async fn create_ci_model(
    pool: &DbPool,
    id: &str,
    code: &str,
    name: &str,
    icon: &str,
    description: &str,
    enabled: bool,
    sort_order: i32,
) -> anyhow::Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO ci_models (id, code, name, icon, description, enabled, sort_order, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(code)
    .bind(name)
    .bind(icon)
    .bind(description)
    .bind(if enabled { 1 } else { 0 })
    .bind(sort_order)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(())
}

/// 更新 CI 模型（全量更新可变字段，code 不可改）。
pub async fn update_ci_model(
    pool: &DbPool,
    id: &str,
    name: &str,
    icon: &str,
    description: &str,
    enabled: bool,
    sort_order: i32,
) -> anyhow::Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let affected = sqlx::query(
        "UPDATE ci_models SET name = ?, icon = ?, description = ?, enabled = ?, sort_order = ?, updated_at = ? WHERE id = ?",
    )
    .bind(name)
    .bind(icon)
    .bind(description)
    .bind(if enabled { 1 } else { 0 })
    .bind(sort_order)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?
    .rows_affected();
    if affected == 0 {
        anyhow::bail!("ci_model not found: {}", id);
    }
    Ok(())
}

/// 删除 CI 模型（连同属性定义一起删，需先确认无实例）。
pub async fn delete_ci_model(pool: &DbPool, id: &str) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM ci_model_attrs WHERE model_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    let affected = sqlx::query("DELETE FROM ci_models WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    if affected == 0 {
        anyhow::bail!("ci_model not found: {}", id);
    }
    tx.commit().await?;
    Ok(())
}

/// 统计某模型下的实例数（删除前校验用）。
pub async fn count_ci_instances_by_model_id(pool: &DbPool, model_id: &str) -> anyhow::Result<i64> {
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ci_instances WHERE model_id = ?")
        .bind(model_id)
        .fetch_one(pool)
        .await?;
    Ok(n)
}

// ---- 模型属性 ----

/// 模型属性行。options 为 JSON（枚举选项）。
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CiModelAttr {
    pub id: String,
    pub model_id: String,
    pub code: String,
    pub name: String,
    pub value_type: String,
    pub default_value: String,
    #[serde(serialize_with = "serialize_json_str")]
    pub options: Option<String>,
    #[serde(serialize_with = "serialize_bool")]
    pub is_required: i8,
    #[serde(serialize_with = "serialize_bool")]
    pub is_unique: i8,
    #[serde(serialize_with = "serialize_bool")]
    pub is_searchable: i8,
    pub sort_order: i32,
    pub created_at: String,
}

pub async fn list_ci_model_attrs(pool: &DbPool, model_id: &str) -> anyhow::Result<Vec<CiModelAttr>> {
    let rows = sqlx::query_as::<_, CiModelAttr>(
        "SELECT id, model_id, code, name, value_type, default_value, \
                CAST(options AS CHAR) AS options, is_required, is_unique, is_searchable, sort_order, created_at \
         FROM ci_model_attrs WHERE model_id = ? ORDER BY sort_order",
    )
    .bind(model_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// 一次性查询所有模型的属性数（GROUP BY model_id），避免前端逐个请求。
/// 返回 model_id → 属性数 的映射。
pub async fn count_ci_model_attrs_by_model(pool: &DbPool) -> anyhow::Result<std::collections::HashMap<String, i64>> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT model_id, COUNT(*) FROM ci_model_attrs GROUP BY model_id",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().collect())
}

/// 批量查询多模型的属性（用于实例列表展示列定义）。
pub async fn list_ci_model_attrs_batch(
    pool: &DbPool,
    model_ids: &[String],
) -> anyhow::Result<Vec<CiModelAttr>> {
    if model_ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = model_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT id, model_id, code, name, value_type, default_value, \
                CAST(options AS CHAR) AS options, is_required, is_unique, is_searchable, sort_order, created_at \
         FROM ci_model_attrs WHERE model_id IN ({}) ORDER BY sort_order",
        placeholders
    );
    let mut q = sqlx::query_as::<_, CiModelAttr>(&sql);
    for mid in model_ids {
        q = q.bind(mid);
    }
    let rows = q.fetch_all(pool).await?;
    Ok(rows)
}

/// 按 id 查询模型属性。
pub async fn find_ci_model_attr(pool: &DbPool, id: &str) -> anyhow::Result<Option<CiModelAttr>> {
    let row = sqlx::query_as::<_, CiModelAttr>(
        "SELECT id, model_id, code, name, value_type, default_value, \
                CAST(options AS CHAR) AS options, is_required, is_unique, is_searchable, sort_order, created_at \
         FROM ci_model_attrs WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// 按 (model_id, code) 查询属性（唯一性校验用）。
pub async fn find_ci_model_attr_by_code(
    pool: &DbPool,
    model_id: &str,
    code: &str,
) -> anyhow::Result<Option<CiModelAttr>> {
    let row = sqlx::query_as::<_, CiModelAttr>(
        "SELECT id, model_id, code, name, value_type, default_value, \
                CAST(options AS CHAR) AS options, is_required, is_unique, is_searchable, sort_order, created_at \
         FROM ci_model_attrs WHERE model_id = ? AND code = ?",
    )
    .bind(model_id)
    .bind(code)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// 创建模型属性。options 为 JSON 字符串（枚举选项数组），可为空。
pub async fn create_ci_model_attr(
    pool: &DbPool,
    id: &str,
    model_id: &str,
    code: &str,
    name: &str,
    value_type: &str,
    default_value: &str,
    options_json: Option<&str>,
    is_required: bool,
    is_unique: bool,
    is_searchable: bool,
    sort_order: i32,
) -> anyhow::Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let options: Option<&str> = if options_json.map(|s| s.is_empty()).unwrap_or(true) {
        None
    } else {
        options_json
    };
    sqlx::query(
        "INSERT INTO ci_model_attrs (id, model_id, code, name, value_type, default_value, options, is_required, is_unique, is_searchable, sort_order, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(model_id)
    .bind(code)
    .bind(name)
    .bind(value_type)
    .bind(default_value)
    .bind(options)
    .bind(if is_required { 1 } else { 0 })
    .bind(if is_unique { 1 } else { 0 })
    .bind(if is_searchable { 1 } else { 0 })
    .bind(sort_order)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(())
}

/// 更新模型属性（code 不可改）。
pub async fn update_ci_model_attr(
    pool: &DbPool,
    id: &str,
    name: &str,
    value_type: &str,
    default_value: &str,
    options_json: Option<&str>,
    is_required: bool,
    is_unique: bool,
    is_searchable: bool,
    sort_order: i32,
) -> anyhow::Result<()> {
    let options: Option<&str> = if options_json.map(|s| s.is_empty()).unwrap_or(true) {
        None
    } else {
        options_json
    };
    let affected = sqlx::query(
        "UPDATE ci_model_attrs SET name = ?, value_type = ?, default_value = ?, options = ?, is_required = ?, is_unique = ?, is_searchable = ?, sort_order = ? WHERE id = ?",
    )
    .bind(name)
    .bind(value_type)
    .bind(default_value)
    .bind(options)
    .bind(if is_required { 1 } else { 0 })
    .bind(if is_unique { 1 } else { 0 })
    .bind(if is_searchable { 1 } else { 0 })
    .bind(sort_order)
    .bind(id)
    .execute(pool)
    .await?
    .rows_affected();
    if affected == 0 {
        anyhow::bail!("ci_model_attr not found: {}", id);
    }
    Ok(())
}

/// 删除模型属性。
pub async fn delete_ci_model_attr(pool: &DbPool, id: &str) -> anyhow::Result<()> {
    let affected = sqlx::query("DELETE FROM ci_model_attrs WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected();
    if affected == 0 {
        anyhow::bail!("ci_model_attr not found: {}", id);
    }
    Ok(())
}

// ---- CI 实例 ----

/// CI 实例行。attributes 为 JSON 字符串（序列化时转为 JSON Value）。
/// source/external_id/last_synced_at 用于外部 CMDB 同步标识。
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CiInstance {
    pub id: String,
    pub model_id: String,
    pub name: String,
    pub status: String,
    pub department_id: Option<String>,
    pub owner_id: Option<String>,
    #[serde(serialize_with = "serialize_json_str")]
    pub attributes: Option<String>,
    pub tags: String,
    pub source: Option<String>,
    pub external_id: Option<String>,
    pub last_synced_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 分页查询 CI 实例。支持按 model_id / status / keyword（name 模糊）筛选。
/// 返回 (总数, 当前页数据)。
pub async fn query_ci_instances(
    pool: &DbPool,
    model_id: Option<&str>,
    status: Option<&str>,
    keyword: Option<&str>,
    department_id: Option<&str>,
    page: u64,
    page_size: u64,
) -> anyhow::Result<(i64, Vec<CiInstance>)> {
    let mut conditions = Vec::new();
    let mut args: Vec<String> = Vec::new();

    if let Some(v) = model_id {
        conditions.push("model_id = ?");
        args.push(v.to_string());
    }
    if let Some(v) = status {
        conditions.push("status = ?");
        args.push(v.to_string());
    }
    if let Some(v) = keyword {
        conditions.push("name LIKE ?");
        args.push(format!("%{}%", v));
    }
    if let Some(v) = department_id {
        conditions.push("department_id = ?");
        args.push(v.to_string());
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // count
    let count_sql = format!("SELECT COUNT(*) FROM ci_instances {}", where_clause);
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    for arg in &args {
        count_query = count_query.bind(arg);
    }
    let total = count_query.fetch_one(pool).await?;

    // data
    let offset = (page.saturating_sub(1)) * page_size;
    let data_sql = format!(
        "SELECT id, model_id, name, status, department_id, owner_id, \
                CAST(attributes AS CHAR) AS attributes, tags, \
                source, external_id, last_synced_at, created_at, updated_at \
         FROM ci_instances {} ORDER BY updated_at DESC LIMIT ? OFFSET ?",
        where_clause
    );
    let mut data_query = sqlx::query_as::<_, CiInstance>(&data_sql);
    for arg in &args {
        data_query = data_query.bind(arg);
    }
    data_query = data_query.bind(page_size as i64).bind(offset as i64);
    let rows = data_query.fetch_all(pool).await?;

    Ok((total, rows))
}

pub async fn find_ci_instance_by_id(pool: &DbPool, id: &str) -> anyhow::Result<Option<CiInstance>> {
    let row = sqlx::query_as::<_, CiInstance>(
        "SELECT id, model_id, name, status, department_id, owner_id, \
                CAST(attributes AS CHAR) AS attributes, tags, \
                source, external_id, last_synced_at, created_at, updated_at \
         FROM ci_instances WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// 创建 CI 实例。attributes 为 JSON 字符串。
pub async fn create_ci_instance(
    pool: &DbPool,
    model_id: &str,
    name: &str,
    status: &str,
    department_id: Option<&str>,
    owner_id: Option<&str>,
    attributes_json: &str,
    tags: &str,
) -> anyhow::Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO ci_instances (id, model_id, name, status, department_id, owner_id, attributes, tags, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(model_id)
    .bind(name)
    .bind(status)
    .bind(department_id)
    .bind(owner_id)
    .bind(attributes_json)
    .bind(tags)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(id)
}

/// 更新 CI 实例（全量更新可变字段）。
pub async fn update_ci_instance(
    pool: &DbPool,
    id: &str,
    name: &str,
    status: &str,
    department_id: Option<&str>,
    owner_id: Option<&str>,
    attributes_json: &str,
    tags: &str,
) -> anyhow::Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let affected = sqlx::query(
        "UPDATE ci_instances SET name = ?, status = ?, department_id = ?, owner_id = ?, attributes = ?, tags = ?, updated_at = ? WHERE id = ?",
    )
    .bind(name)
    .bind(status)
    .bind(department_id)
    .bind(owner_id)
    .bind(attributes_json)
    .bind(tags)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?
    .rows_affected();
    if affected == 0 {
        anyhow::bail!("ci_instance not found: {}", id);
    }
    Ok(())
}

/// 删除 CI 实例。同时清理关联的关系（调用方事务保证）。
pub async fn delete_ci_instance(pool: &DbPool, id: &str) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;
    // 先删关系
    sqlx::query("DELETE FROM ci_relations WHERE source_id = ? OR target_id = ?")
        .bind(id)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    // 再删实例
    let affected = sqlx::query("DELETE FROM ci_instances WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    if affected == 0 {
        anyhow::bail!("ci_instance not found: {}", id);
    }
    tx.commit().await?;
    Ok(())
}

/// 模型统计行（含实例数），用于 dashboard 一次性获取全部模型 + 计数，避免 N+1 查询。
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CiModelStat {
    pub id: String,
    pub code: String,
    pub name: String,
    pub icon: String,
    pub count: i64,
}

/// 一次性查询所有模型及其实例数（LEFT JOIN + GROUP BY），单条 SQL 替代 N+1 循环。
/// 返回 (模型统计列表, 总实例数)。
pub async fn list_ci_model_stats(pool: &DbPool) -> anyhow::Result<(Vec<CiModelStat>, i64)> {
    let rows = sqlx::query_as::<_, CiModelStat>(
        "SELECT m.id AS id, m.code AS code, m.name AS name, m.icon AS icon, \
                COUNT(i.id) AS count \
         FROM ci_models m \
         LEFT JOIN ci_instances i ON i.model_id = m.id \
         GROUP BY m.id, m.code, m.name, m.icon \
         ORDER BY m.sort_order, m.name",
    )
    .fetch_all(pool)
    .await?;
    let total = rows.iter().map(|r| r.count).sum();
    Ok((rows, total))
}

// ---- CI 关系类型 ----

/// CI 关系类型字典行。
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CiRelationType {
    pub id: String,
    pub code: String,
    pub name: String,
    pub description: String,
    #[serde(serialize_with = "serialize_bool")]
    pub directional: i8,
    #[serde(serialize_with = "serialize_bool")]
    pub enabled: i8,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// 查询所有关系类型（默认按 sort_order 排序）。
pub async fn list_ci_relation_types(pool: &DbPool) -> anyhow::Result<Vec<CiRelationType>> {
    let rows = sqlx::query_as::<_, CiRelationType>(
        "SELECT id, code, name, description, directional, enabled, sort_order, created_at, updated_at \
         FROM ci_relation_types ORDER BY sort_order ASC, code ASC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// 查询已启用的关系类型（供资产详情页下拉使用）。
pub async fn list_enabled_ci_relation_types(pool: &DbPool) -> anyhow::Result<Vec<CiRelationType>> {
    let rows = sqlx::query_as::<_, CiRelationType>(
        "SELECT id, code, name, description, directional, enabled, sort_order, created_at, updated_at \
         FROM ci_relation_types WHERE enabled = 1 ORDER BY sort_order ASC, code ASC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// 创建关系类型。code 重复时返回错误。
pub async fn create_ci_relation_type(
    pool: &DbPool,
    id: &str,
    code: &str,
    name: &str,
    description: &str,
    directional: bool,
    enabled: bool,
    sort_order: i32,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO ci_relation_types (id, code, name, description, directional, enabled, sort_order) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(code)
    .bind(name)
    .bind(description)
    .bind(directional as i8)
    .bind(enabled as i8)
    .bind(sort_order)
    .execute(pool)
    .await?;
    Ok(())
}

/// 更新关系类型。
pub async fn update_ci_relation_type(
    pool: &DbPool,
    id: &str,
    name: &str,
    description: &str,
    directional: bool,
    enabled: bool,
    sort_order: i32,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE ci_relation_types SET name=?, description=?, directional=?, enabled=?, sort_order=? WHERE id=?",
    )
    .bind(name)
    .bind(description)
    .bind(directional as i8)
    .bind(enabled as i8)
    .bind(sort_order)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

/// 删除关系类型。有关联关系存在时拒绝。
pub async fn delete_ci_relation_type(pool: &DbPool, id: &str) -> anyhow::Result<bool> {
    // 先查出 code，再检查 ci_relations 是否有引用
    let row: Option<(String,)> = sqlx::query_as("SELECT code FROM ci_relation_types WHERE id=?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    let code = match row {
        Some((c,)) => c,
        None => return Ok(false),
    };
    let used: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ci_relations WHERE relation_type=?")
        .bind(&code)
        .fetch_one(pool)
        .await?;
    if used.0 > 0 {
        return Err(anyhow::anyhow!("该关系类型已被 {} 条关系引用，无法删除", used.0));
    }
    sqlx::query("DELETE FROM ci_relation_types WHERE id=?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(true)
}

// ---- CI 关系 ----

/// CI 关系行。
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CiRelation {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub relation_type: String,
    pub created_at: String,
}

/// 关系行（含对端实例名称 + 关系类型中文名称），避免前端逐个请求导致 N+1。
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CiRelationWithName {
    pub id: String,
    pub source_id: String,
    pub source_name: String,
    pub target_id: String,
    pub target_name: String,
    pub relation_type: String,
    /// 关系类型中文名称（JOIN ci_relation_types.name），未匹配时回空串
    pub relation_type_name: String,
    pub created_at: String,
}

/// 查询某 CI 实例的所有关系（作为源或目标），同时 JOIN 返回对端实例名称和关系类型中文名称。
pub async fn list_ci_relations_with_names(pool: &DbPool, ci_id: &str) -> anyhow::Result<Vec<CiRelationWithName>> {
    let rows = sqlx::query_as::<_, CiRelationWithName>(
        "SELECT r.id, r.source_id, s.name AS source_name, \
                r.target_id, t.name AS target_name, \
                r.relation_type, COALESCE(rt.name, '') AS relation_type_name, \
                r.created_at \
         FROM ci_relations r \
         LEFT JOIN ci_instances s ON s.id = r.source_id \
         LEFT JOIN ci_instances t ON t.id = r.target_id \
         LEFT JOIN ci_relation_types rt ON rt.code = r.relation_type \
         WHERE r.source_id = ? OR r.target_id = ? \
         ORDER BY r.created_at DESC",
    )
    .bind(ci_id)
    .bind(ci_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// 查询某 CI 实例的所有关系（作为源或目标）。
pub async fn list_ci_relations(pool: &DbPool, ci_id: &str) -> anyhow::Result<Vec<CiRelation>> {
    let rows = sqlx::query_as::<_, CiRelation>(
        "SELECT id, source_id, target_id, relation_type, created_at \
         FROM ci_relations WHERE source_id = ? OR target_id = ? ORDER BY created_at DESC",
    )
    .bind(ci_id)
    .bind(ci_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// 创建 CI 关系（幂等：相同 source+target+type 不重复）。
pub async fn create_ci_relation(
    pool: &DbPool,
    source_id: &str,
    target_id: &str,
    relation_type: &str,
) -> anyhow::Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT IGNORE INTO ci_relations (id, source_id, target_id, relation_type, created_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(source_id)
    .bind(target_id)
    .bind(relation_type)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(id)
}

/// 删除 CI 关系。
pub async fn delete_ci_relation(pool: &DbPool, id: &str) -> anyhow::Result<()> {
    let affected = sqlx::query("DELETE FROM ci_relations WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected();
    if affected == 0 {
        anyhow::bail!("ci_relation not found: {}", id);
    }
    Ok(())
}

// ---- 拓扑视图 ----

/// 拓扑节点：实例 + 模型信息。
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopoNode {
    pub id: String,
    pub name: String,
    pub status: String,
    pub model_id: String,
    pub model_code: Option<String>,
    pub model_name: Option<String>,
    pub icon: Option<String>,
    pub source: Option<String>,
}

/// 拓扑边：CI 关系。
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopoLink {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub relation_type: String,
}

/// 查询拓扑：返回节点 + 边。
/// 支持按 model_id / status 筛选节点；边只保留两端都在节点集合内的关系。
pub async fn query_topology(
    pool: &DbPool,
    model_id: Option<&str>,
    status: Option<&str>,
) -> anyhow::Result<(Vec<TopoNode>, Vec<TopoLink>)> {
    let mut conditions = Vec::new();
    let mut args: Vec<String> = Vec::new();
    if let Some(v) = model_id {
        conditions.push("i.model_id = ?");
        args.push(v.to_string());
    }
    if let Some(v) = status {
        conditions.push("i.status = ?");
        args.push(v.to_string());
    }
    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // 节点：实例 JOIN 模型（INNER JOIN，仅保留有有效模型的实例，避免 model_code NULL）
    let node_sql = format!(
        "SELECT i.id, i.name, i.status, i.model_id, m.code AS model_code, m.name AS model_name, m.icon, i.source \
         FROM ci_instances i \
         INNER JOIN ci_models m ON m.id = i.model_id \
         {} ORDER BY m.sort_order, i.name",
        where_clause
    );
    let mut node_query = sqlx::query_as::<_, TopoNode>(&node_sql);
    for arg in &args {
        node_query = node_query.bind(arg);
    }
    let nodes = node_query.fetch_all(pool).await?;

    if nodes.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }

    // 边：只保留两端都在节点集合内的关系
    let node_ids: Vec<&str> = nodes.iter().map(|n| n.id.as_str()).collect();
    let placeholders = node_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let link_sql = format!(
        "SELECT id, source_id, target_id, relation_type \
         FROM ci_relations \
         WHERE source_id IN ({}) AND target_id IN ({})",
        placeholders, placeholders
    );
    let mut link_query = sqlx::query_as::<_, TopoLink>(&link_sql);
    for nid in &node_ids {
        link_query = link_query.bind(*nid);
    }
    for nid in &node_ids {
        link_query = link_query.bind(*nid);
    }
    let links = link_query.fetch_all(pool).await?;

    Ok((nodes, links))
}

// ============ CMDB 同步：外部系统（蓝鲸等）数据接入 ============

/// 同步数据源行。
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncSource {
    pub id: String,
    pub code: String,
    pub name: String,
    pub source_type: String,
    pub api_url: String,
    pub api_token: String,
    pub webhook_secret: String,
    #[serde(serialize_with = "serialize_bool")]
    pub enabled: i8,
    pub last_sync_at: Option<String>,
    pub last_sync_count: i32,
    pub last_sync_status: String,
    /// 拉取配置 JSON（API 路径/分页/过滤等）
    #[serde(serialize_with = "serialize_json_str")]
    pub pull_config: Option<String>,
    /// 定时拉取 cron 表达式（空=不定时）
    pub pull_cron: String,
    #[serde(serialize_with = "serialize_bool")]
    pub pull_enabled: i8,
    pub created_at: String,
    pub updated_at: String,
}

impl SyncSource {
    pub fn is_enabled(&self) -> bool {
        self.enabled != 0
    }
}

/// 同步日志行。
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncLog {
    pub id: i64,
    pub source_code: String,
    pub batch_id: String,
    pub action: String,
    pub model_code: String,
    pub external_id: String,
    pub instance_id: Option<String>,
    pub instance_name: String,
    pub status: String,
    pub message: String,
    #[serde(serialize_with = "serialize_json_str")]
    pub payload: Option<String>,
    pub created_at: String,
}

/// 列出所有同步数据源。
pub async fn list_sync_sources(pool: &DbPool) -> anyhow::Result<Vec<SyncSource>> {
    let rows = sqlx::query_as::<_, SyncSource>(
        "SELECT id, code, name, source_type, api_url, api_token, webhook_secret, \
                enabled, last_sync_at, last_sync_count, last_sync_status, \
                CAST(pull_config AS CHAR) AS pull_config, pull_cron, pull_enabled, \
                created_at, updated_at \
         FROM sync_sources ORDER BY created_at",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// 按 code 查询同步数据源。
pub async fn find_sync_source_by_code(pool: &DbPool, code: &str) -> anyhow::Result<Option<SyncSource>> {
    let row = sqlx::query_as::<_, SyncSource>(
        "SELECT id, code, name, source_type, api_url, api_token, webhook_secret, \
                enabled, last_sync_at, last_sync_count, last_sync_status, \
                CAST(pull_config AS CHAR) AS pull_config, pull_cron, pull_enabled, \
                created_at, updated_at \
         FROM sync_sources WHERE code = ?",
    )
    .bind(code)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// 按 (source, external_id) 查找已存在的 CI 实例。
/// 用于 upsert：找到则更新，找不到则创建。
pub async fn find_ci_instance_by_external(
    pool: &DbPool,
    source: &str,
    external_id: &str,
) -> anyhow::Result<Option<CiInstance>> {
    let row = sqlx::query_as::<_, CiInstance>(
        "SELECT id, model_id, name, status, department_id, owner_id, \
                CAST(attributes AS CHAR) AS attributes, tags, \
                source, external_id, last_synced_at, created_at, updated_at \
         FROM ci_instances WHERE source = ? AND external_id = ?",
    )
    .bind(source)
    .bind(external_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// 同步 upsert：按 (source, external_id) 幂等写入。
/// 已存在则更新（保留 department_id / owner_id / tags 等本地字段），不存在则创建。
/// 返回 (instance_id, is_new)。
pub async fn upsert_ci_instance(
    pool: &DbPool,
    model_id: &str,
    name: &str,
    status: &str,
    attributes_json: &str,
    source: &str,
    external_id: &str,
) -> anyhow::Result<(String, bool)> {
    let now = chrono::Utc::now().to_rfc3339();

    // 先尝试更新
    let updated = sqlx::query(
        "UPDATE ci_instances SET model_id = ?, name = ?, status = ?, attributes = ?, \
         source = ?, external_id = ?, last_synced_at = ?, updated_at = ? \
         WHERE source = ? AND external_id = ?",
    )
    .bind(model_id)
    .bind(name)
    .bind(status)
    .bind(attributes_json)
    .bind(source)
    .bind(external_id)
    .bind(&now)
    .bind(&now)
    .bind(source)
    .bind(external_id)
    .execute(pool)
    .await?
    .rows_affected();

    if updated > 0 {
        // 回查 id
        let row: Option<(String,)> = sqlx::query_as("SELECT id FROM ci_instances WHERE source = ? AND external_id = ?")
            .bind(source)
            .bind(external_id)
            .fetch_optional(pool)
            .await?;
        return Ok((row.unwrap().0, false));
    }

    // 不存在则创建
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO ci_instances (id, model_id, name, status, department_id, owner_id, attributes, tags, \
         source, external_id, last_synced_at, created_at, updated_at) \
         VALUES (?, ?, ?, ?, NULL, NULL, ?, '', ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(model_id)
    .bind(name)
    .bind(status)
    .bind(attributes_json)
    .bind(source)
    .bind(external_id)
    .bind(&now)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok((id, true))
}

/// 插入同步日志明细。
pub async fn insert_sync_log(
    pool: &DbPool,
    source_code: &str,
    batch_id: &str,
    action: &str,
    model_code: &str,
    external_id: &str,
    instance_id: Option<&str>,
    instance_name: &str,
    status: &str,
    message: &str,
    payload: Option<&serde_json::Value>,
) -> anyhow::Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let payload_str: Option<String> = payload.map(|v| v.to_string());
    sqlx::query(
        "INSERT INTO sync_logs (source_code, batch_id, action, model_code, external_id, instance_id, \
         instance_name, status, message, payload, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(source_code)
    .bind(batch_id)
    .bind(action)
    .bind(model_code)
    .bind(external_id)
    .bind(instance_id)
    .bind(instance_name)
    .bind(status)
    .bind(message)
    .bind(payload_str)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(())
}

/// 更新数据源的最近同步状态。
pub async fn update_sync_source_status(
    pool: &DbPool,
    source_code: &str,
    sync_count: i32,
    sync_status: &str,
) -> anyhow::Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE sync_sources SET last_sync_at = ?, last_sync_count = ?, last_sync_status = ?, updated_at = ? \
         WHERE code = ?",
    )
    .bind(&now)
    .bind(sync_count)
    .bind(sync_status)
    .bind(&now)
    .bind(source_code)
    .execute(pool)
    .await?;
    Ok(())
}

/// 更新数据源的拉取配置。
pub async fn update_sync_source_pull_config(
    pool: &DbPool,
    code: &str,
    api_url: &str,
    api_token: &str,
    pull_config: &str,
    pull_cron: &str,
    pull_enabled: bool,
) -> anyhow::Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    // source_type：启用定时拉取→pull，否则保持原值（webhook 推送仍可用）
    if pull_enabled {
        sqlx::query(
            "UPDATE sync_sources SET api_url = ?, api_token = ?, pull_config = ?, \
                    pull_cron = ?, pull_enabled = ?, source_type = 'pull', updated_at = ? \
             WHERE code = ?",
        )
        .bind(api_url)
        .bind(api_token)
        .bind(pull_config)
        .bind(pull_cron)
        .bind(1i8)
        .bind(&now)
        .bind(code)
        .execute(pool)
        .await?;
    } else {
        sqlx::query(
            "UPDATE sync_sources SET api_url = ?, api_token = ?, pull_config = ?, \
                    pull_cron = ?, pull_enabled = ?, updated_at = ? \
             WHERE code = ?",
        )
        .bind(api_url)
        .bind(api_token)
        .bind(pull_config)
        .bind(pull_cron)
        .bind(0i8)
        .bind(&now)
        .bind(code)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// 列出所有启用了定时拉取的数据源（pull_enabled=1 且 pull_cron 非空）。
pub async fn list_pull_enabled_sources(pool: &DbPool) -> anyhow::Result<Vec<SyncSource>> {
    let rows = sqlx::query_as::<_, SyncSource>(
        "SELECT id, code, name, source_type, api_url, api_token, webhook_secret, \
                enabled, last_sync_at, last_sync_count, last_sync_status, \
                CAST(pull_config AS CHAR) AS pull_config, pull_cron, pull_enabled, \
                created_at, updated_at \
         FROM sync_sources WHERE pull_enabled = 1 AND pull_cron != '' AND enabled = 1",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// 创建新的同步数据源。
/// code 必须唯一（表上有 UNIQUE KEY），重复会返回 Err。
pub async fn create_sync_source(
    pool: &DbPool,
    id: &str,
    code: &str,
    name: &str,
    source_type: &str,
    api_url: &str,
    api_token: &str,
    webhook_secret: &str,
    pull_config: &str,
    pull_cron: &str,
    pull_enabled: bool,
) -> anyhow::Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO sync_sources (id, code, name, source_type, api_url, api_token, webhook_secret, \
                enabled, pull_config, pull_cron, pull_enabled, last_sync_count, last_sync_status, \
                created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, 1, ?, ?, ?, 0, '', ?, ?)",
    )
    .bind(id)
    .bind(code)
    .bind(name)
    .bind(source_type)
    .bind(api_url)
    .bind(api_token)
    .bind(webhook_secret)
    .bind(pull_config)
    .bind(pull_cron)
    .bind(if pull_enabled { 1i8 } else { 0i8 })
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(())
}

/// 删除同步数据源。code 不存在不影响（幂等）。
pub async fn delete_sync_source(pool: &DbPool, code: &str) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM sync_sources WHERE code = ?")
        .bind(code)
        .execute(pool)
        .await?;
    Ok(())
}

/// 统计某数据源关联的 CI 实例数（用于删除前校验）。
pub async fn count_ci_instances_by_source(pool: &DbPool, source: &str) -> anyhow::Result<i64> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM ci_instances WHERE source = ?")
        .bind(source)
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

/// 统计某数据源关联的同步日志数（用于删除前校验）。
pub async fn count_sync_logs_by_source(pool: &DbPool, source_code: &str) -> anyhow::Result<i64> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sync_logs WHERE source_code = ?")
        .bind(source_code)
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

/// 分页查询同步日志。支持按 source_code / batch_id / status / instance_id 筛选。
pub async fn query_sync_logs(
    pool: &DbPool,
    source_code: Option<&str>,
    batch_id: Option<&str>,
    status: Option<&str>,
    instance_id: Option<&str>,
    page: u64,
    page_size: u64,
) -> anyhow::Result<(i64, Vec<SyncLog>)> {
    let mut conditions = Vec::new();
    let mut args: Vec<String> = Vec::new();

    if let Some(v) = source_code {
        conditions.push("source_code = ?");
        args.push(v.to_string());
    }
    if let Some(v) = batch_id {
        conditions.push("batch_id = ?");
        args.push(v.to_string());
    }
    if let Some(v) = status {
        conditions.push("status = ?");
        args.push(v.to_string());
    }
    if let Some(v) = instance_id {
        conditions.push("instance_id = ?");
        args.push(v.to_string());
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let count_sql = format!("SELECT COUNT(*) FROM sync_logs {}", where_clause);
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    for arg in &args {
        count_query = count_query.bind(arg);
    }
    let total = count_query.fetch_one(pool).await?;

    let offset = (page.saturating_sub(1)) * page_size;
    let data_sql = format!(
        "SELECT id, source_code, batch_id, action, model_code, external_id, instance_id, \
                instance_name, status, message, CAST(payload AS CHAR) AS payload, created_at \
         FROM sync_logs {} ORDER BY id DESC LIMIT ? OFFSET ?",
        where_clause
    );
    let mut data_query = sqlx::query_as::<_, SyncLog>(&data_sql);
    for arg in &args {
        data_query = data_query.bind(arg);
    }
    data_query = data_query.bind(page_size as i64).bind(offset as i64);
    let rows = data_query.fetch_all(pool).await?;

    Ok((total, rows))
}

// ============ 蓝鲸 CMDB 字段映射 ============

/// 蓝鲸主机字段 → MeridianOps 主机模型属性映射。
/// 蓝鲸字段命名：bk_host_id / bk_host_name / bk_host_innerip / bk_os_name / bk_cpu / bk_mem / bk_disk
/// MeridianOps 字段：hostname / ip / os / cpu / memory / disk
pub fn map_blueking_host(payload: &serde_json::Value) -> Option<MappedInstance> {
    let bk_host_id = payload.get("bk_host_id")?.as_i64()?.to_string();
    let bk_host_name = payload
        .get("bk_host_name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let bk_inner_ip = payload
        .get("bk_host_innerip")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let bk_os_name = payload
        .get("bk_os_name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let bk_cpu = payload.get("bk_cpu").and_then(|v| v.as_i64()).unwrap_or(0);
    let bk_mem = payload.get("bk_mem").and_then(|v| v.as_i64()).unwrap_or(0);
    let bk_disk = payload.get("bk_disk").and_then(|v| v.as_i64()).unwrap_or(0);

    // 蓝鲸 os_name 归一化到 MeridianOps 枚举
    let os = normalize_blueking_os(&bk_os_name);

    // 主机状态：蓝鲸没有直接对应字段，默认 running
    let status = "running".to_string();

    // 显示名：优先 bk_host_name，其次 IP
    let display_name = if !bk_host_name.is_empty() {
        bk_host_name.clone()
    } else if !bk_inner_ip.is_empty() {
        format!("host-{}", bk_inner_ip)
    } else {
        format!("bk-{}", bk_host_id)
    };

    let attributes = serde_json::json!({
        "hostname": bk_host_name,
        "ip": bk_inner_ip,
        "os": os,
        "cpu": bk_cpu,
        "memory": bk_mem,
        "disk": bk_disk
    });

    Some(MappedInstance {
        external_id: bk_host_id,
        model_code: "host".to_string(),
        name: display_name,
        status,
        attributes,
    })
}

/// 蓝鲸 os_name 归一化到 MeridianOps 主机模型 os 枚举。
fn normalize_blueking_os(bk_os: &str) -> String {
    let s = bk_os.to_lowercase();
    if s.contains("centos 7") || s.contains("centos7") {
        "CentOS 7".to_string()
    } else if s.contains("centos 8") || s.contains("centos8") {
        "CentOS 8".to_string()
    } else if s.contains("rhel 7") || s.contains("redhat 7") || s.contains("red hat 7") {
        "RHEL 7".to_string()
    } else if s.contains("rhel 8") || s.contains("redhat 8") || s.contains("red hat 8") {
        "RHEL 8".to_string()
    } else if s.contains("ubuntu 20") {
        "Ubuntu 20.04".to_string()
    } else if s.contains("ubuntu 22") {
        "Ubuntu 22.04".to_string()
    } else if s.contains("windows") {
        "Windows Server 2019".to_string()
    } else if s.contains("aix") {
        "AIX".to_string()
    } else if s.is_empty() {
        "Other".to_string()
    } else {
        bk_os.to_string()
    }
}

/// 映射后的标准化实例（用于同步入库）。
pub struct MappedInstance {
    pub external_id: String,
    pub model_code: String,
    pub name: String,
    pub status: String,
    pub attributes: serde_json::Value,
}

// ============ API 令牌管理（外部系统对接） ============

/// API 令牌行。token 为带前缀的明文（mk-xxxxxxxx），入库时同步写入 token_hash(SHA256)。
/// 注：前端新建时 token 明文**只返回一次**给用户，后续只显示脱敏（mk-****）。
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiToken {
    pub id: String,
    pub name: String,
    pub token: String,
    pub owner_user_id: String,
    #[serde(serialize_with = "serialize_json_str")]
    pub scopes: Option<String>,
    pub role: String,
    pub expires_at: Option<String>,
    #[serde(serialize_with = "serialize_bool")]
    pub revoked: i8,
    pub revoked_at: Option<String>,
    pub last_used_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl ApiToken {
    pub fn is_revoked(&self) -> bool {
        self.revoked != 0
    }

    /// 是否已过期（expires_at < now）。None 表示永不过期。
    pub fn is_expired(&self) -> bool {
        match self.expires_at.as_deref() {
            None => false,
            Some(ts) => match chrono::DateTime::parse_from_rfc3339(ts) {
                Ok(dt) => dt < chrono::Utc::now().with_timezone(&chrono::Local),
                Err(_) => true,
            },
        }
    }

    /// 解析 scopes JSON 字符串为权限码向量。
    pub fn parse_scopes(&self) -> Vec<String> {
        self.scopes
            .as_deref()
            .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
            .unwrap_or_default()
    }
}

/// 创建 API Token。返回 (id, 明文 token)。
/// 明文 token 格式：`mk-` + 48 位安全随机字符（共 51 字符）。
pub async fn create_api_token(
    pool: &DbPool,
    name: &str,
    owner_user_id: &str,
    scopes: &[String],
    role: &str,
    expires_at: Option<&str>,
) -> anyhow::Result<(String, String)> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let token = generate_api_token_plain();
    let token_hash = sha256_hex(&token);
    let scopes_json = serde_json::json!(scopes).to_string();

    sqlx::query(
        "INSERT INTO api_tokens (id, name, token, token_hash, owner_user_id, scopes, role, expires_at, revoked, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?)",
    )
    .bind(&id)
    .bind(name)
    .bind(&token)
    .bind(&token_hash)
    .bind(owner_user_id)
    .bind(&scopes_json)
    .bind(role)
    .bind(expires_at)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok((id, token))
}

/// 生成 API Token 明文：mk- + 48 位 URL 安全随机字符。
fn generate_api_token_plain() -> String {
    use rand::RngCore;
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789";
    let mut bytes = vec![0u8; 48];
    rand::thread_rng().fill_bytes(&mut bytes);
    let s: String = bytes
        .iter()
        .map(|b| CHARSET[(b % CHARSET.len() as u8) as usize] as char)
        .collect();
    format!("mk-{}", s)
}

/// 计算 SHA256 十六进制摘要（小写 64 字符）。
fn sha256_hex(s: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    let result = hasher.finalize();
    result.iter().map(|b| format!("{:02x}", b)).collect()
}

/// 列表：按 owner_user_id 列出全部令牌（返回脱敏 token）。
pub async fn list_api_tokens(pool: &DbPool, owner_user_id: Option<&str>) -> anyhow::Result<Vec<ApiToken>> {
    let (sql, bind_owner): (&str, Option<&str>) = match owner_user_id {
        Some(uid) => (
            "SELECT id, name, token, owner_user_id, CAST(scopes AS CHAR) AS scopes, role, expires_at, \
                    revoked, revoked_at, last_used_at, created_at, updated_at \
             FROM api_tokens WHERE owner_user_id = ? ORDER BY created_at DESC",
            Some(uid),
        ),
        None => (
            "SELECT id, name, token, owner_user_id, CAST(scopes AS CHAR) AS scopes, role, expires_at, \
                    revoked, revoked_at, last_used_at, created_at, updated_at \
             FROM api_tokens ORDER BY created_at DESC",
            None,
        ),
    };
    let mut query = sqlx::query_as::<_, ApiToken>(sql);
    if let Some(uid) = bind_owner {
        query = query.bind(uid);
    }
    let mut rows = query.fetch_all(pool).await?;
    // 脱敏：只保留前缀 mk- + 后 4 位
    for r in &mut rows {
        r.token = mask_token(&r.token);
    }
    Ok(rows)
}

/// 明文 token → 脱敏显示。
fn mask_token(token: &str) -> String {
    if token.len() <= 8 {
        token.to_string()
    } else {
        let suffix = &token[token.len().saturating_sub(4)..];
        format!("mk-****{}", suffix)
    }
}

/// 按 token 明文查找并校验（有效且未过期且未吊销）。
/// 查找成功后自动更新 last_used_at。
pub async fn find_valid_api_token(pool: &DbPool, token_plain: &str) -> anyhow::Result<Option<ApiToken>> {
    let token_hash = sha256_hex(token_plain);
    let row: Option<ApiToken> = sqlx::query_as::<_, ApiToken>(
        "SELECT id, name, token, owner_user_id, CAST(scopes AS CHAR) AS scopes, role, expires_at, \
                revoked, revoked_at, last_used_at, created_at, updated_at \
         FROM api_tokens WHERE token_hash = ? LIMIT 1",
    )
    .bind(&token_hash)
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else { return Ok(None) };
    if row.is_revoked() {
        return Ok(None);
    }
    if row.is_expired() {
        return Ok(None);
    }

    // 更新 last_used_at
    let now = chrono::Utc::now().to_rfc3339();
    let _ = sqlx::query("UPDATE api_tokens SET last_used_at = ? WHERE id = ?")
        .bind(&now)
        .bind(&row.id)
        .execute(pool)
        .await;

    Ok(Some(row))
}

/// 吊销令牌。
pub async fn revoke_api_token(pool: &DbPool, id: &str, owner_user_id: Option<&str>) -> anyhow::Result<bool> {
    let now = chrono::Utc::now().to_rfc3339();
    let (sql, bind_owner): (&str, Option<&str>) = match owner_user_id {
        Some(uid) => (
            "UPDATE api_tokens SET revoked = 1, revoked_at = ?, updated_at = ? \
             WHERE id = ? AND owner_user_id = ? AND revoked = 0",
            Some(uid),
        ),
        None => (
            "UPDATE api_tokens SET revoked = 1, revoked_at = ?, updated_at = ? WHERE id = ? AND revoked = 0",
            None,
        ),
    };
    let mut q = sqlx::query(sql).bind(&now).bind(&now).bind(id);
    if let Some(uid) = bind_owner {
        q = q.bind(uid);
    }
    let affected = q.execute(pool).await?.rows_affected();
    Ok(affected > 0)
}

/// 删除令牌（仅管理员可彻底删除，普通用户只能吊销）。
pub async fn delete_api_token(pool: &DbPool, id: &str) -> anyhow::Result<bool> {
    let affected = sqlx::query("DELETE FROM api_tokens WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected();
    Ok(affected > 0)
}

/// 更新令牌有效期（延长或取消过期）。
pub async fn update_api_token_expiry(
    pool: &DbPool,
    id: &str,
    expires_at: Option<&str>,
    owner_user_id: Option<&str>,
) -> anyhow::Result<bool> {
    let now = chrono::Utc::now().to_rfc3339();
    let (sql, bind_owner): (&str, Option<&str>) = match owner_user_id {
        Some(uid) => (
            "UPDATE api_tokens SET expires_at = ?, updated_at = ? WHERE id = ? AND owner_user_id = ?",
            Some(uid),
        ),
        None => (
            "UPDATE api_tokens SET expires_at = ?, updated_at = ? WHERE id = ?",
            None,
        ),
    };
    let mut q = sqlx::query(sql).bind(expires_at).bind(&now).bind(id);
    if let Some(uid) = bind_owner {
        q = q.bind(uid);
    }
    let affected = q.execute(pool).await?.rows_affected();
    Ok(affected > 0)
}

/// 按 id 查令牌（用于详情，返回脱敏）。
pub async fn find_api_token(pool: &DbPool, id: &str) -> anyhow::Result<Option<ApiToken>> {
    let row: Option<ApiToken> = sqlx::query_as::<_, ApiToken>(
        "SELECT id, name, token, owner_user_id, CAST(scopes AS CHAR) AS scopes, role, expires_at, \
                revoked, revoked_at, last_used_at, created_at, updated_at \
         FROM api_tokens WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|mut r| { r.token = mask_token(&r.token); r }))
}

/// 查找用户角色与权限列表，用于 API Token 创建时做权限上限校验。
pub async fn get_user_role_and_permissions(
    pool: &DbPool,
    user_id: &str,
) -> anyhow::Result<Option<(String, Vec<String>)>> {
    let row: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT role, role_id FROM users WHERE id = ?",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    let Some((role, role_id)) = row else { return Ok(None) };
    let perms = match role_id {
        Some(rid) => list_permission_codes_by_role(pool, &rid).await?,
        None => Vec::new(),
    };
    Ok(Some((role, perms)))
}

