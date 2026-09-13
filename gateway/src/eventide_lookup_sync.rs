//! CMDB → Eventide Lookup 同步。
//!
//! 把本系统有管理 IP 的资产投影到 Eventide 外表（整表覆盖）。
//! 不登录 Eventide 管理员；只用 `[eventide_lookup] lookup_sync_token`。

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{json, Map, Value};
use sqlx::{Column, Row};
use tokio::sync::Notify;
use tokio::time::sleep;

use crate::config::{default_lookup_columns, EventideLookupConfig, EventideLookupTarget};
use crate::db;
use crate::routes::AppState;

const SOURCE_CODE: &str = "eventide_lookup";
const SYNC_SOURCE: &str = "MeridianOps";
const MAX_SQL_ROWS: usize = 3000;

pub struct LookupSyncRuntime {
    pub notify: Notify,
    pub lock: tokio::sync::Mutex<()>,
    pub last: RwLock<Option<LookupSyncReport>>,
}

impl LookupSyncRuntime {
    pub fn new() -> Self {
        Self {
            notify: Notify::new(),
            lock: tokio::sync::Mutex::new(()),
            last: RwLock::new(None),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LookupSyncReport {
    pub trigger: String,
    pub started_at: String,
    pub finished_at: String,
    pub ok: bool,
    pub skipped: bool,
    pub message: String,
    pub targets: Vec<LookupTargetReport>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LookupTargetReport {
    pub lookup_id: String,
    pub name: String,
    pub row_count: usize,
    pub http_status: Option<u16>,
    pub latency_ms: u64,
    pub synced_at: Option<String>,
    pub error: Option<String>,
}

pub fn runtime_config(state: &AppState) -> EventideLookupConfig {
    match state.lookup_runtime.read() {
        Ok(g) => g.clone(),
        Err(_) => state.config.eventide_lookup.clone(),
    }
}

pub fn apply_runtime(state: &AppState, cfg: EventideLookupConfig) -> Result<(), String> {
    let mut g = state
        .lookup_runtime
        .write()
        .map_err(|e| format!("lookup_runtime lock poisoned: {e}"))?;
    *g = cfg;
    Ok(())
}

pub async fn load_runtime(pool: &db::DbPool, toml: &EventideLookupConfig) -> EventideLookupConfig {
    match db::get_setting(pool, "eventide_lookup_config").await {
        Ok(Some(s)) if !s.trim().is_empty() => match serde_json::from_str::<EventideLookupConfig>(&s)
        {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(target: SOURCE_CODE, "ignore bad eventide_lookup_config: {e}");
                toml.clone()
            }
        },
        _ => toml.clone(),
    }
}

pub async fn persist_runtime(
    pool: &db::DbPool,
    cfg: &EventideLookupConfig,
    updated_by: &str,
) -> anyhow::Result<()> {
    let json = serde_json::to_string(cfg)?;
    db::upsert_settings(
        pool,
        &[(
            "eventide_lookup_config".into(),
            json,
            updated_by.to_string(),
        )],
    )
    .await
}

fn is_ready(cfg: &EventideLookupConfig) -> bool {
    cfg.enabled && !resolve_token(cfg).is_empty() && !usable_targets(cfg).is_empty()
}

/// 资产/负责人变更后通知调度器（debounce 后全量推一次）。
pub fn bump(state: &AppState) {
    if !runtime_config(state).enabled {
        return;
    }
    state.lookup_sync.notify.notify_waiters();
}

pub async fn start_scheduler(state: Arc<AppState>) {
    tracing::info!(target: SOURCE_CODE, "scheduler loop online");
    let cron_state = state.clone();
    tokio::spawn(async move {
        let mut last = Instant::now();
        loop {
            sleep(Duration::from_secs(5)).await;
            let cfg = runtime_config(&cron_state);
            if !is_ready(&cfg) {
                continue;
            }
            let interval = Duration::from_secs(cfg.interval_secs.max(30));
            if last.elapsed() >= interval {
                last = Instant::now();
                run_locked(&cron_state, "cron").await;
            }
        }
    });

    loop {
        state.lookup_sync.notify.notified().await;
        let cfg = runtime_config(&state);
        if !is_ready(&cfg) {
            continue;
        }
        let debounce = Duration::from_secs(cfg.debounce_secs.clamp(5, 300));
        loop {
            tokio::select! {
                _ = sleep(debounce) => break,
                _ = state.lookup_sync.notify.notified() => {}
            }
        }
        if is_ready(&runtime_config(&state)) {
            run_locked(&state, "debounce").await;
        }
    }
}

pub async fn run_locked(state: &AppState, trigger: &str) -> LookupSyncReport {
    let _guard = state.lookup_sync.lock.lock().await;
    let report = run_once(state, trigger).await;
    if let Ok(mut slot) = state.lookup_sync.last.write() {
        *slot = Some(report.clone());
    }
    report
}

pub fn status_json(state: &AppState) -> Value {
    let cfg = runtime_config(state);
    let base = resolve_base_url(state, &cfg);
    let last = state
        .lookup_sync
        .last
        .read()
        .ok()
        .and_then(|g| g.clone());
    let targets: Vec<Value> = cfg
        .targets
        .iter()
        .map(|t| {
            json!({
                "lookupId": t.lookup_id,
                "name": t.name,
                "source": if target_is_sql(t) { "sql" } else { "hosts" },
                "sql": t.sql,
                "keyColumn": t.key_column,
            })
        })
        .collect();
    let token = resolve_token(&cfg);
    json!({
        "enabled": cfg.enabled,
        "configured": is_ready(&cfg),
        "baseUrl": base,
        "intervalSecs": cfg.interval_secs,
        "debounceSecs": cfg.debounce_secs,
        "tokenSet": !token.is_empty(),
        "tokenMasked": mask_token(&token),
        "targets": targets,
        "last": last,
    })
}

fn mask_token(token: &str) -> String {
    if token.is_empty() {
        return String::new();
    }
    let suffix: String = token.chars().rev().take(4).collect::<String>().chars().rev().collect();
    format!("lks_••••{suffix}")
}

/// 当前将要推送的投影（不写 Eventide），供页面预览。
pub async fn preview_json(state: &AppState) -> Result<Value, String> {
    let cfg = runtime_config(state);
    let mut targets_out = Vec::new();
    let mut total = 0usize;
    let host_projections = if cfg.targets.iter().any(|t| !target_is_sql(t)) {
        Some(
            collect_projections(&state.db)
                .await
                .map_err(|e| format!("查询 CMDB 失败: {e}"))?,
        )
    } else {
        None
    };
    for t in usable_targets(&cfg) {
        let rows = collect_target_rows(&state.db, t, host_projections.as_deref()).await?;
        let items = rows_to_preview_items(&rows);
        total += items.len();
        targets_out.push(json!({
            "lookupId": t.lookup_id,
            "name": t.name,
            "source": if target_is_sql(t) { "sql" } else { "hosts" },
            "rowCount": items.len(),
            "rows": items,
        }));
    }
    Ok(json!({
        "rowCount": total,
        "targets": targets_out,
    }))
}

pub async fn preview_sql_json(
    pool: &db::DbPool,
    sql: &str,
    key_column: &str,
) -> Result<Value, String> {
    let mut t = EventideLookupTarget::default();
    t.source = "sql".into();
    t.sql = sql.to_string();
    t.key_column = key_column.to_string();
    let rows = collect_sql_rows(pool, &t).await?;
    let items = rows_to_preview_items(&rows);
    let columns = items
        .first()
        .and_then(|v| v.as_object())
        .map(|o| o.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    Ok(json!({
        "rowCount": items.len(),
        "columns": columns,
        "rows": items.into_iter().take(50).collect::<Vec<_>>(),
    }))
}

fn rows_to_preview_items(rows: &Map<String, Value>) -> Vec<Value> {
    let mut items = Vec::new();
    for (key, cell) in rows {
        let mut obj = Map::new();
        obj.insert("ip".into(), Value::String(key.clone()));
        obj.insert("_key".into(), Value::String(key.clone()));
        if let Value::Object(m) = cell {
            for (k, v) in m {
                obj.insert(k.clone(), v.clone());
            }
        }
        items.push(Value::Object(obj));
    }
    items
}

async fn run_once(state: &AppState, trigger: &str) -> LookupSyncReport {
    let started = chrono::Utc::now().to_rfc3339();
    let cfg = runtime_config(state);
    let mut report = LookupSyncReport {
        trigger: trigger.to_string(),
        started_at: started,
        finished_at: String::new(),
        ok: false,
        skipped: false,
        message: String::new(),
        targets: Vec::new(),
    };

    if !cfg.enabled {
        report.skipped = true;
        report.message = "未启用".into();
        report.finished_at = chrono::Utc::now().to_rfc3339();
        return report;
    }
    let token = resolve_token(&cfg);
    let base = resolve_base_url(state, &cfg);
    if token.is_empty() || base.is_empty() {
        report.message = "未配置 lookup_sync_token 或 Eventide 地址".into();
        report.finished_at = chrono::Utc::now().to_rfc3339();
        tracing::error!(target: SOURCE_CODE, "{}", report.message);
        return report;
    }
    let targets = usable_targets(&cfg);
    if targets.is_empty() {
        report.message = "没有有效 targets".into();
        report.finished_at = chrono::Utc::now().to_rfc3339();
        tracing::error!(target: SOURCE_CODE, "{}", report.message);
        return report;
    }

    let need_hosts = targets.iter().any(|t| !target_is_sql(t));
    let host_projections = if need_hosts {
        match collect_projections(&state.db).await {
            Ok(v) => Some(v),
            Err(e) => {
                report.message = format!("查询 CMDB 失败: {e}");
                report.finished_at = chrono::Utc::now().to_rfc3339();
                tracing::error!(target: SOURCE_CODE, "{}", report.message);
                return report;
            }
        }
    } else {
        None
    };

    let batch_id = uuid::Uuid::new_v4().to_string();
    let mut all_ok = true;
    for t in targets {
        let rows = match collect_target_rows(&state.db, t, host_projections.as_deref()).await {
            Ok(v) => v,
            Err(e) => {
                let mut one = LookupTargetReport {
                    lookup_id: t.lookup_id.clone(),
                    name: t.name.clone(),
                    row_count: 0,
                    http_status: None,
                    latency_ms: 0,
                    synced_at: None,
                    error: Some(e),
                };
                let _ = db::insert_sync_log(
                    &state.db,
                    SOURCE_CODE,
                    &batch_id,
                    "push",
                    &t.name,
                    &t.lookup_id,
                    None,
                    &t.name,
                    "failed",
                    one.error.as_deref().unwrap_or("failed"),
                    None,
                )
                .await;
                all_ok = false;
                report.targets.push(one);
                continue;
            }
        };
        let snapshot = lookup_rows_snapshot(&rows, 200);
        let truncated = rows.len() > 200;
        let one = push_target(&state.client, &base, &token, t, rows).await;
        let ok_msg = format!("已推送 {} 行", one.row_count);
        let log_msg = one.error.as_deref().unwrap_or(&ok_msg);
        let _ = db::insert_sync_log(
            &state.db,
            SOURCE_CODE,
            &batch_id,
            "push",
            &t.name,
            &t.lookup_id,
            None,
            &t.name,
            if one.error.is_none() { "success" } else { "failed" },
            log_msg,
            Some(&json!({
                "rowCount": one.row_count,
                "httpStatus": one.http_status,
                "latencyMs": one.latency_ms,
                "syncedAt": one.synced_at,
                "trigger": trigger,
                "rows": snapshot,
                "truncated": truncated,
            })),
        )
        .await;
        if one.error.is_some() {
            all_ok = false;
        }
        report.targets.push(one);
    }
    report.ok = all_ok;
    let total_rows: usize = report.targets.iter().map(|t| t.row_count).sum();
    report.message = if all_ok {
        format!("已推送 {total_rows} 行")
    } else {
        "部分或全部外表同步失败".into()
    };
    report.finished_at = chrono::Utc::now().to_rfc3339();
    tracing::info!(
        target: SOURCE_CODE,
        trigger,
        ok = all_ok,
        "sync finished targets={}",
        report.targets.len()
    );
    report
}

#[derive(Debug, Clone)]
struct HostProjection {
    ip: String,
    fields: BTreeMap<String, String>,
}

async fn collect_projections(pool: &db::DbPool) -> anyhow::Result<Vec<HostProjection>> {
    let lites = db::list_all_ci_instances_lite(pool).await?;
    let ids: Vec<String> = lites.iter().map(|x| x.0.clone()).collect();
    let owners = db::list_ci_owner_contacts_map(pool, &ids).await?;
    let mut out = Vec::new();
    for (id, name, attrs_raw) in lites {
        let attrs: Value = attrs_raw
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or(Value::Null);
        let Some(ip) = first_ip(&attrs) else {
            continue;
        };
        let hostname = attr_str(
            &attrs,
            &["hostname", "hostName", "bk_host_name", "host_name"],
        );
        let display = if hostname.is_empty() {
            name.clone()
        } else {
            hostname
        };
        let contacts = owners.get(&id).cloned().unwrap_or_default();
        let owner_names: Vec<String> = contacts
            .iter()
            .map(|c| {
                let n = c.display_name.trim();
                if n.is_empty() {
                    c.username.clone()
                } else {
                    n.to_string()
                }
            })
            .filter(|s| !s.is_empty())
            .collect();
        let owner_phone = contacts
            .iter()
            .filter_map(|c| c.mobile.as_deref())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("、");
        let owner_email = contacts
            .iter()
            .filter_map(|c| c.email.as_deref())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("、");

        let mut fields = BTreeMap::new();
        fields.insert("name".into(), display);
        fields.insert(
            "datacenter".into(),
            attr_str(&attrs, &["datacenter", "idc", "机房", "location", "dc"]),
        );
        fields.insert("owner_name".into(), owner_names.join("、"));
        fields.insert("owner_phone".into(), owner_phone);
        fields.insert("owner_email".into(), owner_email);
        fields.insert(
            "biz_line".into(),
            attr_str(
                &attrs,
                &[
                    "biz",
                    "biz_line",
                    "business",
                    "business_line",
                    "app",
                    "system_code",
                ],
            ),
        );
        out.push(HostProjection { ip, fields });
    }
    Ok(out)
}

fn target_is_sql(t: &EventideLookupTarget) -> bool {
    t.source.eq_ignore_ascii_case("sql")
}

async fn collect_target_rows(
    pool: &db::DbPool,
    target: &EventideLookupTarget,
    hosts: Option<&[HostProjection]>,
) -> Result<Map<String, Value>, String> {
    if target_is_sql(target) {
        collect_sql_rows(pool, target).await
    } else if let Some(items) = hosts {
        Ok(project_rows(items, target))
    } else {
        let items = collect_projections(pool)
            .await
            .map_err(|e| format!("查询 CMDB 失败: {e}"))?;
        Ok(project_rows(&items, target))
    }
}

pub fn validate_lookup_sql(sql: &str) -> Result<String, String> {
    let trimmed = sql.trim();
    if trimmed.is_empty() {
        return Err("SQL 为空".into());
    }
    if trimmed.contains("--") || trimmed.contains("/*") || trimmed.contains("*/") || trimmed.contains('#')
    {
        return Err("SQL 不允许包含注释".into());
    }
    let body = trimmed.trim_end_matches(';').trim();
    if body.contains(';') {
        return Err("只允许一条语句".into());
    }
    let lower = body.to_ascii_lowercase();
    let first = lower.split_whitespace().next().unwrap_or("");
    if first != "select" && first != "with" {
        return Err("只允许 SELECT 或 WITH ... SELECT".into());
    }
    for (pat, label) in [
        (" into outfile", "INTO OUTFILE"),
        (" into dumpfile", "INTO DUMPFILE"),
        (" for update", "FOR UPDATE"),
        (" load data", "LOAD DATA"),
    ] {
        if lower.contains(pat) {
            return Err(format!("SQL 不允许 {label}"));
        }
    }
    for word in [
        "insert", "update", "delete", "drop", "alter", "truncate", "create", "grant", "revoke",
        "call",
    ] {
        if has_sql_word(&lower, word) {
            return Err(format!("SQL 不允许 {word}"));
        }
    }
    Ok(body.to_string())
}

fn has_sql_word(hay: &str, word: &str) -> bool {
    let bytes = hay.as_bytes();
    let w = word.as_bytes();
    let mut start = 0;
    while start + w.len() <= bytes.len() {
        if let Some(rel) = hay[start..].find(word) {
            let abs = start + rel;
            let before_ok = abs == 0 || !bytes[abs - 1].is_ascii_alphanumeric();
            let after = abs + w.len();
            let after_ok = after >= bytes.len() || !bytes[after].is_ascii_alphanumeric();
            if before_ok && after_ok {
                return true;
            }
            start = abs + 1;
        } else {
            break;
        }
    }
    false
}

async fn collect_sql_rows(
    pool: &db::DbPool,
    target: &EventideLookupTarget,
) -> Result<Map<String, Value>, String> {
    let sql = validate_lookup_sql(&target.sql)?;
    let fetched = sqlx::query(&sql)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("执行 SQL 失败: {e}"))?;
    if fetched.is_empty() {
        return Ok(Map::new());
    }
    let col_names: Vec<String> = fetched[0]
        .columns()
        .iter()
        .map(|c| c.name().to_string())
        .collect();
    if col_names.is_empty() {
        return Err("SQL 没有返回列".into());
    }
    let key_name = if target.key_column.trim().is_empty() {
        col_names[0].clone()
    } else {
        target.key_column.trim().to_string()
    };
    let key_idx = col_names
        .iter()
        .position(|c| c.eq_ignore_ascii_case(&key_name))
        .ok_or_else(|| format!("找不到 key 列 `{key_name}`"))?;

    let mut rows = Map::new();
    for (i, row) in fetched.iter().enumerate() {
        if i >= MAX_SQL_ROWS {
            break;
        }
        let key = mysql_cell_string(row, key_idx);
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        let mut cell = Map::new();
        for (idx, name) in col_names.iter().enumerate() {
            if idx == key_idx {
                continue;
            }
            cell.insert(name.clone(), Value::String(mysql_cell_string(row, idx)));
        }
        rows.insert(key.to_string(), Value::Object(cell));
    }
    Ok(rows)
}

fn mysql_cell_string(row: &sqlx::mysql::MySqlRow, i: usize) -> String {
    if let Ok(v) = row.try_get::<Option<String>, _>(i) {
        return v.unwrap_or_default();
    }
    if let Ok(v) = row.try_get::<Option<i64>, _>(i) {
        return v.map(|x| x.to_string()).unwrap_or_default();
    }
    if let Ok(v) = row.try_get::<Option<u64>, _>(i) {
        return v.map(|x| x.to_string()).unwrap_or_default();
    }
    if let Ok(v) = row.try_get::<Option<i32>, _>(i) {
        return v.map(|x| x.to_string()).unwrap_or_default();
    }
    if let Ok(v) = row.try_get::<Option<f64>, _>(i) {
        return v.map(|x| x.to_string()).unwrap_or_default();
    }
    if let Ok(v) = row.try_get::<Option<bool>, _>(i) {
        return v.map(|x| if x { "1" } else { "0" }.to_string()).unwrap_or_default();
    }
    if let Ok(v) = row.try_get::<Option<chrono::NaiveDateTime>, _>(i) {
        return v.map(|x| x.to_string()).unwrap_or_default();
    }
    String::new()
}

/// 默认主机投影（中文列），供其它 HTTP 出站通道复用。
pub async fn collect_default_host_rows(pool: &db::DbPool) -> Result<Map<String, Value>, String> {
    let items = collect_projections(pool)
        .await
        .map_err(|e| format!("查询 CMDB 失败: {e}"))?;
    Ok(project_rows(&items, &EventideLookupTarget::default()))
}

/// 自定义 SELECT 投影，规则与 Eventide SQL 外表相同。
pub async fn collect_sql_lookup_rows(
    pool: &db::DbPool,
    sql: &str,
    key_column: &str,
) -> Result<Map<String, Value>, String> {
    let mut t = EventideLookupTarget::default();
    t.source = "sql".into();
    t.sql = sql.to_string();
    t.key_column = key_column.to_string();
    collect_sql_rows(pool, &t).await
}

/// 把 key→属性 投影摊成带 `ip` 的行列表。
pub fn rows_preview_list(rows: &Map<String, Value>, limit: usize) -> Vec<Value> {
    lookup_rows_snapshot(rows, limit)
}

fn lookup_rows_snapshot(rows: &Map<String, Value>, limit: usize) -> Vec<Value> {
    rows.iter()
        .take(limit)
        .map(|(ip, cell)| {
            let mut obj = Map::new();
            obj.insert("ip".into(), Value::String(ip.clone()));
            if let Value::Object(m) = cell {
                for (k, v) in m {
                    obj.insert(k.clone(), v.clone());
                }
            }
            Value::Object(obj)
        })
        .collect()
}

fn project_rows(items: &[HostProjection], target: &EventideLookupTarget) -> Map<String, Value> {
    let cols = if target.columns.is_empty() {
        default_lookup_columns()
    } else {
        target.columns.clone()
    };
    let mut rows = Map::new();
    for item in items {
        let mut cell = Map::new();
        for (col, src) in &cols {
            let val = item.fields.get(src).cloned().unwrap_or_default();
            cell.insert(col.clone(), Value::String(val));
        }
        rows.insert(item.ip.clone(), Value::Object(cell));
    }
    rows
}

async fn push_target(
    client: &reqwest::Client,
    base: &str,
    token: &str,
    target: &EventideLookupTarget,
    rows: Map<String, Value>,
) -> LookupTargetReport {
    let mut one = LookupTargetReport {
        lookup_id: target.lookup_id.clone(),
        name: target.name.clone(),
        row_count: rows.len(),
        http_status: None,
        latency_ms: 0,
        synced_at: None,
        error: None,
    };
    if rows.is_empty() {
        one.error = Some("rows 为空，跳过 PUT（防止清空 Eventide）".into());
        tracing::error!(
            target: SOURCE_CODE,
            lookup_id = %target.lookup_id,
            "empty rows, skip PUT"
        );
        return one;
    }

    let url = format!(
        "{}/api/lookups/{}/rows",
        base.trim_end_matches('/'),
        target.lookup_id
    );
    let body = json!({
        "rows": rows,
        "reject_empty": true,
        "sync_source": SYNC_SOURCE,
    });

    let mut delay = Duration::from_secs(1);
    for attempt in 0..4 {
        let t0 = Instant::now();
        let send = client
            .put(&url)
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await;
        one.latency_ms = t0.elapsed().as_millis() as u64;
        match send {
            Ok(resp) => {
                let status = resp.status();
                one.http_status = Some(status.as_u16());
                let text = resp.text().await.unwrap_or_default();
                if status.is_success() {
                    if let Ok(v) = serde_json::from_str::<Value>(&text) {
                        one.synced_at = v
                            .get("synced_at")
                            .or_else(|| v.get("syncedAt"))
                            .or_else(|| v.pointer("/data/synced_at"))
                            .and_then(|x| x.as_str())
                            .map(|s| s.to_string());
                    }
                    tracing::info!(
                        target: SOURCE_CODE,
                        lookup_id = %target.lookup_id,
                        row_count = one.row_count,
                        http_status = status.as_u16(),
                        latency_ms = one.latency_ms,
                        "PUT ok"
                    );
                    return one;
                }
                let retryable = status.is_server_error();
                let msg = format!("HTTP {} {}", status.as_u16(), truncate(&text, 300));
                if retryable && attempt < 3 {
                    tracing::warn!(
                        target: SOURCE_CODE,
                        lookup_id = %target.lookup_id,
                        attempt,
                        "{}",
                        msg
                    );
                    sleep(delay).await;
                    delay *= 2;
                    continue;
                }
                one.error = Some(msg);
                tracing::error!(
                    target: SOURCE_CODE,
                    lookup_id = %target.lookup_id,
                    http_status = status.as_u16(),
                    "{}",
                    one.error.as_deref().unwrap_or("")
                );
                return one;
            }
            Err(e) => {
                let msg = format!("请求失败: {e}");
                if attempt < 3 {
                    tracing::warn!(target: SOURCE_CODE, lookup_id = %target.lookup_id, attempt, "{msg}");
                    sleep(delay).await;
                    delay *= 2;
                    continue;
                }
                one.error = Some(msg);
                tracing::error!(target: SOURCE_CODE, lookup_id = %target.lookup_id, "{}", one.error.as_deref().unwrap_or(""));
                return one;
            }
        }
    }
    one
}

fn usable_targets(cfg: &EventideLookupConfig) -> Vec<&EventideLookupTarget> {
    cfg.targets
        .iter()
        .filter(|t| !t.lookup_id.trim().is_empty())
        .collect()
}

fn resolve_token(cfg: &EventideLookupConfig) -> String {
    cfg.lookup_sync_token.trim().to_string()
}

fn resolve_base_url(state: &AppState, cfg: &EventideLookupConfig) -> String {
    let explicit = cfg.base_url.trim();
    if !explicit.is_empty() {
        return explicit.trim_end_matches('/').to_string();
    }
    state
        .config
        .system_by_id("eventide")
        .map(|s| s.base_url.trim().trim_end_matches('/').to_string())
        .unwrap_or_default()
}

fn first_ip(attrs: &Value) -> Option<String> {
    for key in ["ip", "manageIp", "manage_ip", "host_ip", "mgmt_ip", "bk_host_innerip"] {
        if let Some(ip) = attrs.get(key).and_then(value_as_str).and_then(normalize_ipv4) {
            return Some(ip);
        }
    }
    None
}

fn attr_str(attrs: &Value, keys: &[&str]) -> String {
    for k in keys {
        if let Some(s) = attrs.get(*k).and_then(value_as_str) {
            let t = s.trim();
            if !t.is_empty() {
                return t.to_string();
            }
        }
    }
    String::new()
}

fn value_as_str(v: &Value) -> Option<&str> {
    v.as_str().or_else(|| {
        // 数字 IP 不常见，忽略
        None
    })
}

/// 去空格、去掉 :端口；仅接受 IPv4。
pub fn normalize_ipv4(raw: &str) -> Option<String> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    let host = if let Some(rest) = s.strip_prefix('[') {
        rest.split(']').next().unwrap_or(rest)
    } else if s.matches('.').count() == 3 {
        s.split('/').next().unwrap_or(s).split(':').next().unwrap_or(s)
    } else {
        return None;
    };
    let host = host.trim();
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() != 4 {
        return None;
    }
    for p in &parts {
        if p.parse::<u8>().is_err() {
            return None;
        }
    }
    Some(host.to_string())
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect::<String>() + "…"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipv4_strips_port_and_space() {
        assert_eq!(normalize_ipv4(" 10.20.0.1:9100 "), Some("10.20.0.1".into()));
        assert_eq!(normalize_ipv4("10.20.0.1"), Some("10.20.0.1".into()));
        assert_eq!(normalize_ipv4("10.20.0.1/24"), Some("10.20.0.1".into()));
        assert!(normalize_ipv4("").is_none());
        assert!(normalize_ipv4("web01").is_none());
        assert!(normalize_ipv4("::1").is_none());
    }

    #[test]
    fn sql_allows_select_rejects_dml() {
        assert!(validate_lookup_sql("SELECT ip, name AS `主机名` FROM ci_instances").is_ok());
        assert!(validate_lookup_sql(
            "WITH x AS (SELECT 1 AS ip) SELECT ip FROM x"
        )
        .is_ok());
        assert!(validate_lookup_sql("SELECT updated_at FROM users").is_ok());
        assert!(validate_lookup_sql("DELETE FROM users").is_err());
        assert!(validate_lookup_sql("SELECT 1; DROP TABLE users").is_err());
        assert!(validate_lookup_sql("SELECT 1 -- x").is_err());
    }
}
