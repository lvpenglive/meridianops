//! 数据库访问层：MySQL 连接池、users 表数据访问、首次启动种子 admin。

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
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub email: String,
    pub password_hash: String,
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
        "INSERT INTO users (id, username, display_name, email, password_hash, role, role_id, department_id, enabled, created_at, updated_at)
         VALUES (?, ?, '管理员', '', ?, 'admin', ?, NULL, 1, ?, ?)",
    )
    .bind(&id)
    .bind(&cfg.seed_username)
    .bind(&hash)
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
        "INSERT INTO users (id, username, display_name, email, password_hash, role, role_id, department_id, enabled, last_login_at, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, ?, ?)",
    )
    .bind(&id)
    .bind(username)
    .bind(display_name)
    .bind(email)
    .bind(password_hash)
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

/// 管理员重置密码：直接写入新 hash。
pub async fn update_password(pool: &DbPool, id: &str, password_hash: &str) -> anyhow::Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let affected = sqlx::query("UPDATE users SET password_hash = ?, updated_at = ? WHERE id = ?")
        .bind(password_hash)
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

/// 分页查询审计日志。支持按 actor/action/target_type/status 筛选。
/// 返回 (总条数, 当前页数据)。
pub async fn query_audit_logs(
    pool: &DbPool,
    actor: Option<&str>,
    action: Option<&str>,
    target_type: Option<&str>,
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
