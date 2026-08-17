//! 作业中心路由（V1 核心框架）
//!
//! ## V1 功能范围
//!   - 作业定义 CRUD（name/description/script_type/script_content/timeout/target_scope/target_assets）
//!   - 执行作业：选择目标资产 → 发起异步执行 → 立即返回 job_run.id
//!   - 执行引擎：V1 使用 MockExecutor（返回示例 stdout/exit_code），V1.5 接入 russh 真实 SSH
//!   - 执行历史列表 + 单条 job_run 详情（含所有 target 的 stdout/stderr）
//!
//! ## 端点
//!   GET    /api/jobs/definitions                  作业定义列表(分页)          (job:read)
//!   POST   /api/jobs/definitions                  新建作业定义                 (job:create)
//!   GET    /api/jobs/definitions/:id              单条作业定义详情              (job:read)
//!   PUT    /api/jobs/definitions/:id              更新作业定义                 (job:create)
//!   DELETE /api/jobs/definitions/:id              删除作业定义                 (job:admin)
//!   POST   /api/jobs/definitions/:id/execute      立即执行作业(传资产ID数组)    (job:execute)
//!   GET    /api/jobs/runs                         执行历史列表(分页)            (job:read)
//!   GET    /api/jobs/runs/:id                     单条执行历史+targets         (job:read)
//!   GET    /api/jobs/runs/:id/targets/:tid        单个target的stdout/stderr    (job:read)

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use axum::extract::{ConnectInfo, Path, Query, State};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use serde::Deserialize;
use sqlx::{MySqlPool, Row};

use crate::audit;
use crate::auth;
use crate::error::AppError;
use crate::routes::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/jobs/assets", get(list_assets_for_select))
        .route(
            "/api/jobs/definitions",
            get(list_definitions).post(create_definition),
        )
        .route(
            "/api/jobs/definitions/:id",
            get(get_definition)
                .put(update_definition)
                .delete(delete_definition),
        )
        .route(
            "/api/jobs/definitions/:id/execute",
            post(execute_job),
        )
        .route("/api/jobs/runs", get(list_runs))
        .route("/api/jobs/runs/:id", get(get_run))
        .route(
            "/api/jobs/runs/:id/targets/:tid",
            get(get_run_target_output),
        )
}

// ===== 通用分页查询 =====
#[derive(Deserialize, Default, Clone)]
#[serde(default, rename_all = "camelCase")]
struct PagerQuery {
    page: i64,
    page_size: i64,
    keyword: String,
    status: String,
}
impl PagerQuery {
    fn normalize(&mut self) -> (i64, i64) {
        if self.page < 1 {
            self.page = 1;
        }
        if self.page_size < 1 || self.page_size > 1000 {
            self.page_size = 20;
        }
        (self.page, self.page_size)
    }
}

// ===== 请求结构 =====

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateDefinitionRequest {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default = "default_script_type")]
    script_type: String,
    script_content: String,
    #[serde(default = "default_timeout")]
    timeout_secs: i64,
    #[serde(default = "default_scope")]
    target_scope: String,
    #[serde(default)]
    target_asset_ids: Option<Vec<String>>,
    #[serde(default)]
    target_cmdb_query: Option<String>,
    #[serde(default = "default_run_as")]
    run_as: String,
    #[serde(default = "default_port")]
    port: i64,
    #[serde(default = "default_enabled")]
    enabled: bool,
    /// V1.5: 执行器类型 mock / ssh（默认 ssh）
    #[serde(default = "default_executor_type")]
    executor_type: String,
    /// V1.5: SSH 凭据 ID（executor_type=ssh 时必填）
    #[serde(default)]
    credential_id: Option<i64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateDefinitionRequest {
    name: String,
    #[serde(default)]
    description: Option<String>,
    script_type: String,
    script_content: String,
    timeout_secs: i64,
    target_scope: String,
    #[serde(default)]
    target_asset_ids: Option<Vec<String>>,
    #[serde(default)]
    target_cmdb_query: Option<String>,
    run_as: String,
    port: i64,
    enabled: bool,
    /// V1.5: 执行器类型 mock / ssh
    executor_type: String,
    /// V1.5: SSH 凭据 ID
    #[serde(default)]
    credential_id: Option<i64>,
}

fn default_script_type() -> String { "shell".to_string() }
fn default_timeout() -> i64 { 300 }
fn default_scope() -> String { "manual".to_string() }
fn default_run_as() -> String { "root".to_string() }
fn default_port() -> i64 { 22 }
fn default_enabled() -> bool { true }
fn default_executor_type() -> String { "ssh".to_string() }

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExecuteRequest {
    /// 选择的资产 ID 数组（CI 实例 UUID 字符串）；空数组 => 走作业定义的 target_asset_ids
    #[serde(default)]
    asset_ids: Option<Vec<String>>,
}

// ===== 作业定义 CRUD =====

async fn list_definitions(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Query(mut query): Query<PagerQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "job:read")?;
    crate::license_routes::require_active_license(&state.db).await?;
    let (page, page_size) = query.normalize();
    let offset = (page - 1) * page_size;
    let kw = query.keyword.clone();
    let st = query.status.clone();

    let mut sql_where = "WHERE 1=1".to_string();
    if !kw.is_empty() {
        sql_where.push_str(&format!(
            " AND (name LIKE '%{}%' OR description LIKE '%{}%')",
            mysql_like_escape(&kw),
            mysql_like_escape(&kw)
        ));
    }
    if !st.is_empty() {
        match st.as_str() {
            "enabled" => sql_where.push_str(" AND enabled=1"),
            "disabled" => sql_where.push_str(" AND enabled=0"),
            _ => {}
        }
    }

    let total: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM job_definitions {}",
        sql_where
    ))
    .fetch_one(&state.db)
    .await?;

    let rows = sqlx::query(&format!(
        "SELECT id, name, description, script_type, timeout_secs, target_scope, \
                run_as, port, enabled, executor_type, credential_id, created_by, created_at, updated_at \
         FROM job_definitions {0} \
         ORDER BY id DESC LIMIT {1} OFFSET {2}",
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
                "description": r.try_get::<String, _>("description").unwrap_or_default(),
                "scriptType": r.try_get::<String, _>("script_type").unwrap_or_default(),
                "timeoutSecs": r.try_get::<i64, _>("timeout_secs").unwrap_or(300),
                "targetScope": r.try_get::<String, _>("target_scope").unwrap_or_default(),
                "runAs": r.try_get::<String, _>("run_as").unwrap_or_default(),
                "port": r.try_get::<i64, _>("port").unwrap_or(22),
                "enabled": r.try_get::<i8, _>("enabled").unwrap_or(1) == 1,
                "executorType": r.try_get::<String, _>("executor_type").unwrap_or_else(|_| "ssh".into()),
                "credentialId": r.try_get::<Option<i64>, _>("credential_id").ok().flatten(),
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

async fn get_definition(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "job:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let def = load_def(&state.db, id).await?;
    Ok(Json(serde_json::json!({ "code": 0, "data": def })))
}

async fn create_definition(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    auth: auth::AuthUser,
    Json(req): Json<CreateDefinitionRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "job:create")?;
    crate::license_routes::require_active_license(&state.db).await?;

    validate_script_content(&req.script_content)?;
    if req.name.trim().is_empty() {
        return Err(AppError::bad("作业名称不能为空"));
    }

    let target_ids_json = req.target_asset_ids.as_ref().and_then(|v| {
        if v.is_empty() {
            None
        } else {
            Some(serde_json::to_string(v).ok()?)
        }
    });

    let result = sqlx::query(
        "INSERT INTO job_definitions \
            (name, description, script_type, script_content, timeout_secs, target_scope, \
             target_asset_ids, target_cmdb_query, run_as, port, enabled, executor_type, credential_id, created_by) \
         VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
    )
    .bind(req.name.trim())
    .bind(req.description.clone().unwrap_or_default())
    .bind(req.script_type.clone())
    .bind(req.script_content.clone())
    .bind(req.timeout_secs)
    .bind(req.target_scope.clone())
    .bind(target_ids_json)
    .bind(req.target_cmdb_query.clone().unwrap_or_default())
    .bind(req.run_as.clone())
    .bind(req.port)
    .bind(if req.enabled { 1i8 } else { 0i8 })
    .bind(req.executor_type.clone())
    .bind(req.credential_id)
    .bind(auth.username())
    .execute(&state.db)
    .await?;
    let new_id = result.last_insert_id() as i64;

    audit::log_async(
        &state.db,
        &auth,
        "create_job_def",
        "job_definition",
        &new_id.to_string(),
        Some(&serde_json::json!({
            "id": new_id, "name": req.name, "scriptType": req.script_type, "timeoutSecs": req.timeout_secs,
        })),
        &addr.ip().to_string(),
        "success",
    )
    .await;

    Ok(Json(serde_json::json!({
        "code": 0, "message": "创建成功",
        "data": { "id": new_id }
    })))
}

async fn update_definition(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    auth: auth::AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateDefinitionRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "job:create")?;
    crate::license_routes::require_active_license(&state.db).await?;
    validate_script_content(&req.script_content)?;

    let target_ids_json = req.target_asset_ids.as_ref().and_then(|v| {
        if v.is_empty() { None } else { Some(serde_json::to_string(v).ok()?) }
    });

    sqlx::query(
        "UPDATE job_definitions SET name=?, description=?, script_type=?, script_content=?, \
                timeout_secs=?, target_scope=?, target_asset_ids=?, target_cmdb_query=?, \
                run_as=?, port=?, enabled=?, executor_type=?, credential_id=?, updated_at=NOW() \
         WHERE id=?",
    )
    .bind(req.name.trim())
    .bind(req.description.clone().unwrap_or_default())
    .bind(req.script_type.clone())
    .bind(req.script_content.clone())
    .bind(req.timeout_secs)
    .bind(req.target_scope.clone())
    .bind(target_ids_json)
    .bind(req.target_cmdb_query.clone().unwrap_or_default())
    .bind(req.run_as.clone())
    .bind(req.port)
    .bind(if req.enabled { 1i8 } else { 0i8 })
    .bind(req.executor_type.clone())
    .bind(req.credential_id)
    .bind(id)
    .execute(&state.db)
    .await?;

    audit::log_async(
        &state.db,
        &auth,
        "update_job_def",
        "job_definition",
        &id.to_string(),
        Some(&serde_json::json!({
            "name": req.name, "scriptType": req.script_type, "timeoutSecs": req.timeout_secs,
        })),
        &addr.ip().to_string(),
        "success",
    )
    .await;

    Ok(Json(serde_json::json!({ "code": 0, "message": "更新成功" })))
}

async fn delete_definition(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    auth: auth::AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "job:admin")?;
    crate::license_routes::require_active_license(&state.db).await?;

    // 有执行历史的作业定义不允许删除（保留审计链路）
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM job_runs WHERE job_id=?")
            .bind(id)
            .fetch_one(&state.db)
            .await?;
    if count > 0 {
        return Err(AppError::bad(&format!(
            "该作业已有 {} 条执行历史，不允许删除。如需停用可将 enabled 置为 false。",
            count
        )));
    }

    sqlx::query("DELETE FROM job_definitions WHERE id=?")
        .bind(id)
        .execute(&state.db)
        .await?;

    audit::log_async(
        &state.db,
        &auth,
        "delete_job_def",
        "job_definition",
        &id.to_string(),
        None,
        &addr.ip().to_string(),
        "success",
    )
    .await;

    Ok(Json(serde_json::json!({ "code": 0, "message": "删除成功" })))
}

// ===== 执行作业 =====

async fn execute_job(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    auth: auth::AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<ExecuteRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "job:execute")?;
    crate::license_routes::require_active_license(&state.db).await?;

    // 1. 加载作业定义
    let def = load_def(&state.db, id).await?;
    if def["enabled"].as_bool() != Some(true) {
        return Err(AppError::bad("该作业已被禁用，无法执行"));
    }

    // 2. 解析目标资产
    let asset_ids: Vec<String> = if let Some(ids) = req.asset_ids {
        ids.into_iter().filter(|s| !s.trim().is_empty()).collect()
    } else if let Some(ids) = def["targetAssetIds"].as_array() {
        ids.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).filter(|s| !s.trim().is_empty()).collect()
    } else {
        Vec::new()
    };
    if asset_ids.is_empty() {
        return Err(AppError::bad("未选择任何目标资产，请在执行时传入 asset_ids 或在作业定义中配置"));
    }
    // 去重
    let mut asset_ids = asset_ids;
    asset_ids.sort_unstable();
    asset_ids.dedup();

    // 3. 查询资产信息（name + ip），从 ci_instances 表（CMDB 动态模型，IP 位于 attributes JSON）
    let placeholders = asset_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");
    // 从 JSON attributes 中尽量推断 IP：key 名为 ip/mgmt_ip/bk_host_innerip 之一
    let sql_str = format!(
        "SELECT i.id, \
                i.name, \
                COALESCE(\
                    JSON_UNQUOTE(JSON_EXTRACT(i.attributes, '$.ip')),\
                    JSON_UNQUOTE(JSON_EXTRACT(i.attributes, '$.mgmt_ip')),\
                    JSON_UNQUOTE(JSON_EXTRACT(i.attributes, '$.bk_host_innerip')),\
                    ''\
                ) AS primary_ip, \
                m.code AS model_code, \
                m.name AS model_name \
         FROM ci_instances i \
         LEFT JOIN ci_models m ON m.id = i.model_id \
         WHERE i.id IN ({})",
        placeholders
    );
    let mut q = sqlx::query(&sql_str);
    for aid in &asset_ids {
        q = q.bind(aid);
    }
    let asset_rows = q.fetch_all(&state.db).await?;
    let mut asset_map: HashMap<String, (String, String)> = HashMap::new();
    for r in &asset_rows {
        let aid: String = r.try_get("id").unwrap_or_default();
        let name: String = r.try_get("name").unwrap_or_default();
        let ip: String = r.try_get("primary_ip").unwrap_or_default();
        asset_map.insert(aid, (name, ip));
    }
    // 未查到的资产给默认值
    for aid in &asset_ids {
        asset_map.entry(aid.clone()).or_insert_with(|| (format!("Asset#{}", aid), String::new()));
    }

    // 4. 创建 job_run
    let script_type = def["scriptType"].as_str().unwrap_or("shell").to_string();
    let script_content = def["scriptContent"].as_str().unwrap_or("").to_string();
    let job_name = def["name"].as_str().unwrap_or("Unknown Job").to_string();

    let run_res = sqlx::query(
        "INSERT INTO job_runs (job_id, job_name, script_type, script_content, trigger_mode, \
                target_count, overall_status, started_by, started_at) \
         VALUES (?,?,?,?, 'manual', ?, 'running', ?, NOW())",
    )
    .bind(id)
    .bind(&job_name)
    .bind(&script_type)
    .bind(&script_content)
    .bind(asset_ids.len() as i64)
    .bind(auth.username())
    .execute(&state.db)
    .await?;
    let run_id = run_res.last_insert_id() as i64;

    // 5. 为每个资产创建 target 记录
    for aid in &asset_ids {
        let (aname, aip) = asset_map.get(aid).cloned().unwrap_or_default();
        sqlx::query(
            "INSERT INTO job_run_targets (job_run_id, asset_id, asset_name, asset_ip, status, duration_ms) \
             VALUES (?,?,?,?, 'pending', 0)",
        )
        .bind(run_id)
        .bind(aid)
        .bind(&aname)
        .bind(&aip)
        .execute(&state.db)
        .await?;
    }

    // 6. 启动异步执行（根据 executor_type 分发：mock / ssh）
    let db_for_exec = state.db.clone();
    let db_for_error = state.db.clone();
    let script_snap = script_content.clone();
    let type_snap = script_type.clone();
    let asset_ids_clone = asset_ids.clone();
    let executor_type = def["executorType"].as_str().unwrap_or("ssh").to_string();
    let credential_id = def["credentialId"].as_i64();
    let timeout_snap = def["timeoutSecs"].as_i64().unwrap_or(300);
    let ssh_port = def["port"].as_i64().unwrap_or(22);

    tokio::spawn(async move {
        let result = if executor_type == "mock" {
            // V1 Mock 执行器
            run_job_mock_executor(
                db_for_exec,
                run_id,
                asset_ids_clone,
                asset_map,
                &script_snap,
                &type_snap,
            )
            .await
        } else {
            // V1.5 SSH 执行器
            let cred_id = match credential_id {
                Some(id) => id,
                None => {
                    tracing::error!(
                        "job_run {} executor_type=ssh 但 credential_id 为空",
                        run_id
                    );
                    let _ = sqlx::query(
                        "UPDATE job_run_targets SET status='failed', stderr='SSH 凭据未配置', finished_at=NOW() WHERE job_run_id=?",
                    )
                    .bind(run_id)
                    .execute(&db_for_error)
                    .await;
                    let _ = sqlx::query(
                        "UPDATE job_runs SET overall_status='failed', finished_at=NOW() WHERE id=?",
                    )
                    .bind(run_id)
                    .execute(&db_for_error)
                    .await;
                    return;
                }
            };

            crate::ssh_executor::run_job_ssh_executor(
                db_for_exec,
                run_id,
                asset_ids_clone,
                asset_map,
                script_snap,
                type_snap,
                timeout_snap,
                cred_id,
                ssh_port,
            )
            .await
        };

        if let Err(e) = result {
            tracing::error!("job_run {} 执行失败: {}", run_id, e);
            let _ = sqlx::query(
                "UPDATE job_runs SET overall_status='failed', finished_at=NOW() WHERE id=?",
            )
            .bind(run_id)
            .execute(&db_for_error)
            .await;
        }
    });

    audit::log_async(
        &state.db,
        &auth,
        "execute_job",
        "job_definition",
        &id.to_string(),
        Some(&serde_json::json!({
            "jobRunId": run_id, "assetCount": asset_ids.len(), "jobName": job_name,
        })),
        &addr.ip().to_string(),
        "success",
    )
    .await;

    Ok(Json(serde_json::json!({
        "code": 0,
        "message": "作业已提交执行",
        "data": { "jobRunId": run_id, "targetCount": asset_ids.len() }
    })))
}

/// V1 Mock 执行器：模拟 SSH 执行，所有资产按 10~500ms 延迟返回随机成功/失败
///
/// 真实执行引擎(V1.5)：接 russh SSH 客户端，逐台资产连接并执行脚本
async fn run_job_mock_executor(
    pool: MySqlPool,
    run_id: i64,
    asset_ids: Vec<String>,
    asset_map: HashMap<String, (String, String)>,
    script_content: &str,
    script_type: &str,
) -> Result<(), AppError> {
    let mut success = 0i64;
    let mut failed = 0i64;
    let timeout_secs: i64 = 300;

    for aid in asset_ids {
        let start = SystemTime::now();
        // 更新 target 状态为 running
        sqlx::query(
            "UPDATE job_run_targets SET status='running', started_at=NOW() WHERE job_run_id=? AND asset_id=?",
        )
        .bind(run_id)
        .bind(&aid)
        .execute(&pool)
        .await
        .ok();

        // --- 模拟执行 ---
        // 简单随机：80% 成功，20% 失败；基于 asset_id hash 保持稳定（同资产多次结果相同）
        let pseudo = aid
            .chars()
            .take(8)
            .fold(0u32, |acc, c| acc.wrapping_mul(31).wrapping_add(c as u32))
            .wrapping_add(run_id as u32);
        let is_success = pseudo % 10 < 8;
        let delay = (pseudo % 400) + 50; // 50~450ms
        tokio::time::sleep(Duration::from_millis(delay as u64)).await;

        let (exit_code, stdout, stderr, status) = if is_success {
            let hostname = asset_map
                .get(&aid)
                .map(|(n, _)| n.clone())
                .unwrap_or_else(|| format!("host-{}", aid));
            let ip = asset_map
                .get(&aid)
                .map(|(_, ip)| ip.clone())
                .unwrap_or_default();
            let sample_stdout = mock_script_output(script_content, script_type, &hostname, &ip, pseudo);
            success += 1;
            (0i64, sample_stdout, String::new(), "success".to_string())
        } else {
            failed += 1;
            (1i64, String::new(), format!("bash: command not found: invalid_cmd_{}\nExit status 1", aid), "failed".to_string())
        };

        let dur = start.elapsed().map(|d| d.as_millis() as i64).unwrap_or(0);
        let _ = timeout_secs; // mock 用不上，真实执行时检查

        sqlx::query(
            "UPDATE job_run_targets SET status=?, exit_code=?, stdout=?, stderr=?, \
                    duration_ms=?, finished_at=NOW() \
             WHERE job_run_id=? AND asset_id=?",
        )
        .bind(&status)
        .bind(exit_code)
        .bind(stdout)
        .bind(stderr)
        .bind(dur)
        .bind(run_id)
        .bind(&aid)
        .execute(&pool)
        .await
        .ok();
    }

    // 全部完成，更新 job_run 状态
    let overall = if failed == 0 {
        "success"
    } else if success == 0 {
        "failed"
    } else {
        "partial"
    };
    sqlx::query(
        "UPDATE job_runs SET success_count=?, failed_count=?, overall_status=?, finished_at=NOW() WHERE id=?",
    )
    .bind(success)
    .bind(failed)
    .bind(overall)
    .bind(run_id)
    .execute(&pool)
    .await
    .ok();

    Ok(())
}

/// 根据脚本内容生成模拟 stdout（看起来像真实执行）
fn mock_script_output(script: &str, script_type: &str, hostname: &str, ip: &str, seed: u32) -> String {
    let mut out = String::new();
    out.push_str(&format!("[{}] Connecting to {} ({}) ...\n", now_fmt(), hostname, if ip.is_empty() { "no-ip" } else { ip }));
    out.push_str(&format!("[{}] Executing {} script ({} bytes):\n", now_fmt(), script_type, script.len()));
    out.push_str("$ ");
    out.push_str(&script.lines().next().unwrap_or("..."));
    out.push('\n');
    out.push('\n');
    // 模拟一些常用命令输出
    if script.contains("uname -a") || script.contains("cat /etc/os-release") {
        out.push_str("Linux meridianops-node 5.15.0-92-generic #102-Ubuntu SMP x86_64 GNU/Linux\n");
        out.push_str("PRETTY_NAME=\"Ubuntu 22.04.4 LTS\"\n");
    }
    if script.contains("df") || script.contains("disk") {
        out.push_str("Filesystem     Size  Used Avail Use% Mounted on\n");
        out.push_str("/dev/sda1      500G  183G  317G  37% /\n");
        out.push_str("/dev/sdb1      2.0T   88G  1.9T   5% /data\n");
    }
    if script.contains("free") || script.contains("memory") || script.contains("mem") {
        out.push_str("              total        used        free      shared\n");
        out.push_str("Mem:          32Gi        12Gi       18Gi       1.2Gi\n");
        out.push_str("Swap:          8Gi       234Mi       7.8Gi\n");
    }
    if script.contains("uptime") || script.contains("load") {
        let load = (seed as f64 % 50.0) / 10.0;
        out.push_str(&format!(" {0} up 127 days,  4:32,  1 user,  load average: {1:.2}, {2:.2}, {3:.2}\n",
            now_fmt_short(),
            load, load * 0.9, load * 0.8));
    }
    // 通用：行数 + 状态
    let extra_lines = (seed % 5) as usize;
    for i in 1..=extra_lines {
        out.push_str(&format!("check_item_{}: OK ({})\n", i, hostname));
    }
    out.push('\n');
    out.push_str(&format!("[{}] Exit code 0\n", now_fmt()));
    out
}

// ===== 执行历史 =====

async fn list_runs(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Query(mut query): Query<PagerQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "job:read")?;
    crate::license_routes::require_active_license(&state.db).await?;
    let (page, page_size) = query.normalize();
    let offset = (page - 1) * page_size;

    let mut sql_where = "WHERE 1=1".to_string();
    if !query.keyword.is_empty() {
        sql_where.push_str(&format!(
            " AND (job_name LIKE '%{}%' OR started_by LIKE '%{}%')",
            mysql_like_escape(&query.keyword),
            mysql_like_escape(&query.keyword)
        ));
    }
    if !query.status.is_empty() {
        sql_where.push_str(&format!(" AND overall_status='{}'", mysql_like_escape(&query.status)));
    }

    let total: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM job_runs {}",
        sql_where
    ))
    .fetch_one(&state.db)
    .await?;

    let rows = sqlx::query(&format!(
        "SELECT id, job_id, job_name, trigger_mode, target_count, success_count, failed_count, \
                overall_status, started_by, started_at, finished_at \
         FROM job_runs {} \
         ORDER BY id DESC LIMIT {} OFFSET {}",
        sql_where, page_size, offset
    ))
    .fetch_all(&state.db)
    .await?;

    let list: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let sc = r.try_get::<i64, _>("success_count").unwrap_or(0);
            let fc = r.try_get::<i64, _>("failed_count").unwrap_or(0);
            let dur = match (
                r.try_get::<Option<chrono::NaiveDateTime>, _>("started_at").ok().flatten(),
                r.try_get::<Option<chrono::NaiveDateTime>, _>("finished_at").ok().flatten(),
            ) {
                (Some(s), Some(f)) => (f - s).num_milliseconds(),
                _ => 0,
            };
            serde_json::json!({
                "id": r.try_get::<i64, _>("id").unwrap_or(0),
                "jobId": r.try_get::<i64, _>("job_id").unwrap_or(0),
                "jobName": r.try_get::<String, _>("job_name").unwrap_or_default(),
                "triggerMode": r.try_get::<String, _>("trigger_mode").unwrap_or_default(),
                "targetCount": r.try_get::<i64, _>("target_count").unwrap_or(0),
                "successCount": sc,
                "failedCount": fc,
                "overallStatus": r.try_get::<String, _>("overall_status").unwrap_or_default(),
                "startedBy": r.try_get::<String, _>("started_by").unwrap_or_default(),
                "startedAt": format_dt(&r, "started_at"),
                "finishedAt": format_dt_opt(&r, "finished_at"),
                "durationMs": dur,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": { "list": list, "total": total, "page": page, "pageSize": page_size }
    })))
}

async fn get_run(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "job:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let run_row = sqlx::query(
        "SELECT id, job_id, job_name, script_type, script_content, trigger_mode, target_count, \
                success_count, failed_count, overall_status, started_by, started_at, finished_at \
         FROM job_runs WHERE id=?",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?;
    let run_row = match run_row {
        Some(r) => r,
        None => return Err(AppError::not_found("执行历史不存在")),
    };

    let target_rows = sqlx::query(
        "SELECT id, asset_id, asset_name, asset_ip, status, exit_code, \
                CASE WHEN LENGTH(COALESCE(stdout,''))>500 THEN CONCAT(LEFT(stdout,500),'...(truncated)') ELSE stdout END AS stdout, \
                CASE WHEN LENGTH(COALESCE(stderr,''))>500 THEN CONCAT(LEFT(stderr,500),'...(truncated)') ELSE stderr END AS stderr, \
                duration_ms, started_at, finished_at \
         FROM job_run_targets WHERE job_run_id=? ORDER BY id",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    let targets: Vec<serde_json::Value> = target_rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.try_get::<i64, _>("id").unwrap_or(0),
                "assetId": r.try_get::<String, _>("asset_id").unwrap_or_default(),
                "assetName": r.try_get::<String, _>("asset_name").unwrap_or_default(),
                "assetIp": r.try_get::<String, _>("asset_ip").unwrap_or_default(),
                "status": r.try_get::<String, _>("status").unwrap_or_default(),
                "exitCode": r.try_get::<Option<i64>, _>("exit_code").ok().flatten(),
                "stdout": r.try_get::<Option<String>, _>("stdout").ok().flatten().unwrap_or_default(),
                "stderr": r.try_get::<Option<String>, _>("stderr").ok().flatten().unwrap_or_default(),
                "durationMs": r.try_get::<i64, _>("duration_ms").unwrap_or(0),
                "startedAt": format_dt_opt(&r, "started_at"),
                "finishedAt": format_dt_opt(&r, "finished_at"),
            })
        })
        .collect();

    let run_json = serde_json::json!({
        "id": run_row.try_get::<i64, _>("id").unwrap_or(0),
        "jobId": run_row.try_get::<i64, _>("job_id").unwrap_or(0),
        "jobName": run_row.try_get::<String, _>("job_name").unwrap_or_default(),
        "scriptType": run_row.try_get::<String, _>("script_type").unwrap_or_default(),
        "scriptContent": run_row.try_get::<String, _>("script_content").unwrap_or_default(),
        "triggerMode": run_row.try_get::<String, _>("trigger_mode").unwrap_or_default(),
        "targetCount": run_row.try_get::<i64, _>("target_count").unwrap_or(0),
        "successCount": run_row.try_get::<i64, _>("success_count").unwrap_or(0),
        "failedCount": run_row.try_get::<i64, _>("failed_count").unwrap_or(0),
        "overallStatus": run_row.try_get::<String, _>("overall_status").unwrap_or_default(),
        "startedBy": run_row.try_get::<String, _>("started_by").unwrap_or_default(),
        "startedAt": format_dt(&run_row, "started_at"),
        "finishedAt": format_dt_opt(&run_row, "finished_at"),
    });

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": { "run": run_json, "targets": targets }
    })))
}

async fn get_run_target_output(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path((run_id, tid)): Path<(i64, i64)>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "job:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let row = sqlx::query(
        "SELECT stdout, stderr, exit_code, status FROM job_run_targets WHERE id=? AND job_run_id=?",
    )
    .bind(tid)
    .bind(run_id)
    .fetch_optional(&state.db)
    .await?;
    let row = match row {
        Some(r) => r,
        None => return Err(AppError::not_found("target 不存在")),
    };
    Ok(Json(serde_json::json!({
        "code": 0,
        "data": {
            "stdout": row.try_get::<Option<String>, _>("stdout").ok().flatten().unwrap_or_default(),
            "stderr": row.try_get::<Option<String>, _>("stderr").ok().flatten().unwrap_or_default(),
            "exitCode": row.try_get::<Option<i64>, _>("exit_code").ok().flatten(),
            "status": row.try_get::<String, _>("status").unwrap_or_default(),
        }
    })))
}

// ===== 资产下拉选择（作业执行对话框用）=====

async fn list_assets_for_select(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Query(mut query): Query<PagerQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "job:read")?;
    crate::license_routes::require_active_license(&state.db).await?;
    let (page, page_size) = query.normalize();
    let offset = (page - 1) * page_size;

    let mut sql_where = String::new();
    if !query.keyword.is_empty() {
        let kw = mysql_like_escape(&query.keyword);
        sql_where.push_str(&format!(
            " AND (i.name LIKE '%{0}%' OR JSON_UNQUOTE(JSON_EXTRACT(i.attributes, '$.ip')) LIKE '%{0}%' OR JSON_UNQUOTE(JSON_EXTRACT(i.attributes, '$.mgmt_ip')) LIKE '%{0}%' OR JSON_UNQUOTE(JSON_EXTRACT(i.attributes, '$.bk_host_innerip')) LIKE '%{0}%')",
            kw
        ));
    }

    let total: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM ci_instances i WHERE i.status != 'deleted'{}",
        sql_where
    ))
    .fetch_one(&state.db)
    .await?;

    let rows = sqlx::query(&format!(
        "SELECT i.id, i.name, \
                COALESCE(\
                    JSON_UNQUOTE(JSON_EXTRACT(i.attributes, '$.ip')),\
                    JSON_UNQUOTE(JSON_EXTRACT(i.attributes, '$.mgmt_ip')),\
                    JSON_UNQUOTE(JSON_EXTRACT(i.attributes, '$.bk_host_innerip')),\
                    ''\
                ) AS primary_ip, \
                m.code AS asset_type, \
                i.status \
         FROM ci_instances i \
         LEFT JOIN ci_models m ON m.id = i.model_id \
         WHERE i.status != 'deleted'{} \
         ORDER BY i.created_at DESC LIMIT {} OFFSET {}",
        sql_where, page_size, offset
    ))
    .fetch_all(&state.db)
    .await?;

    let list: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.try_get::<String, _>("id").unwrap_or_default(),
                "assetName": r.try_get::<String, _>("name").unwrap_or_default(),
                "primaryIp": r.try_get::<String, _>("primary_ip").ok().unwrap_or_default(),
                "assetType": r.try_get::<Option<String>, _>("asset_type").ok().flatten().unwrap_or_default(),
                "status": r.try_get::<String, _>("status").unwrap_or("active".into()),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": { "list": list, "total": total, "page": page, "pageSize": page_size }
    })))
}

// ===== 工具函数 =====

async fn load_def(pool: &MySqlPool, id: i64) -> Result<serde_json::Value, AppError> {
    let row = sqlx::query(
        "SELECT id, name, description, script_type, script_content, timeout_secs, target_scope, \
                target_asset_ids, target_cmdb_query, run_as, port, enabled, executor_type, credential_id, \
                created_by, created_at, updated_at \
         FROM job_definitions WHERE id=?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    let row = match row {
        Some(r) => r,
        None => return Err(AppError::not_found("作业定义不存在")),
    };
    let target_ids: Vec<String> = row
        .try_get::<Option<String>, _>("target_asset_ids")
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    Ok(serde_json::json!({
        "id": row.try_get::<i64, _>("id").unwrap_or(0),
        "name": row.try_get::<String, _>("name").unwrap_or_default(),
        "description": row.try_get::<String, _>("description").unwrap_or_default(),
        "scriptType": row.try_get::<String, _>("script_type").unwrap_or_default(),
        "scriptContent": row.try_get::<String, _>("script_content").unwrap_or_default(),
        "timeoutSecs": row.try_get::<i64, _>("timeout_secs").unwrap_or(300),
        "targetScope": row.try_get::<String, _>("target_scope").unwrap_or_default(),
        "targetAssetIds": target_ids,
        "targetCmdbQuery": row.try_get::<Option<String>, _>("target_cmdb_query").ok().flatten().unwrap_or_default(),
        "runAs": row.try_get::<String, _>("run_as").unwrap_or_default(),
        "port": row.try_get::<i64, _>("port").unwrap_or(22),
        "enabled": row.try_get::<i8, _>("enabled").unwrap_or(1) == 1,
        "executorType": row.try_get::<String, _>("executor_type").unwrap_or_else(|_| "ssh".into()),
        "credentialId": row.try_get::<Option<i64>, _>("credential_id").ok().flatten(),
        "createdBy": row.try_get::<String, _>("created_by").unwrap_or_default(),
        "createdAt": format_dt(&row, "created_at"),
        "updatedAt": format_dt(&row, "updated_at"),
    }))
}

fn validate_script_content(s: &str) -> Result<(), AppError> {
    if s.trim().is_empty() {
        return Err(AppError::bad("脚本内容不能为空"));
    }
    if s.len() > 500_000 {
        return Err(AppError::bad("脚本内容过长，上限 500KB"));
    }
    Ok(())
}

fn mysql_like_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "\\'").replace('%', "\\%").replace('_', "\\_")
}

fn format_dt(row: &sqlx::mysql::MySqlRow, col: &str) -> String {
    row.try_get::<String, _>(col)
        .unwrap_or_else(|_| {
            row.try_get::<chrono::NaiveDateTime, _>(col)
                .ok()
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_default()
        })
}

fn format_dt_opt(row: &sqlx::mysql::MySqlRow, col: &str) -> Option<String> {
    let s = row.try_get::<Option<String>, _>(col).ok().flatten();
    if let Some(s) = s {
        if !s.is_empty() { return Some(s); }
    }
    row.try_get::<Option<chrono::NaiveDateTime>, _>(col)
        .ok()
        .flatten()
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
}

fn now_fmt() -> String {
    chrono::Local::now().format("%H:%M:%S").to_string()
}
fn now_fmt_short() -> String {
    chrono::Local::now().format("%H:%M:%S").to_string()
}
