//! HTTP 出站推送：把主机投影或自定义 SQL POST/PUT 到其它平台。
//!
//! 一个通道可配多张外表（与 Eventide `targets` 相同）。
//! 样例：优云 CMDB、AxleOps、理想自动化。

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::db;
use crate::eventide_lookup_sync;
use crate::routes::AppState;

const SYNC_SOURCE: &str = "MeridianOps";
const SNAPSHOT_LIMIT: usize = 200;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpPushTarget {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub path: String,
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default = "default_body_mode")]
    pub body_mode: String,
    #[serde(default = "default_content_source")]
    pub content_source: String,
    #[serde(default)]
    pub sql: String,
    #[serde(default)]
    pub key_column: String,
    #[serde(default = "default_true")]
    pub reject_empty: bool,
}

impl Default for HttpPushTarget {
    fn default() -> Self {
        Self {
            name: "hosts".into(),
            path: String::new(),
            method: default_method(),
            body_mode: default_body_mode(),
            content_source: default_content_source(),
            sql: String::new(),
            key_column: String::new(),
            reject_empty: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpPushConfig {
    #[serde(default = "default_auth_style")]
    pub auth_style: String,
    #[serde(default)]
    pub extra_headers: Value,
    #[serde(default)]
    pub platform: String,
    #[serde(default)]
    pub targets: Vec<HttpPushTarget>,
    /// 兼容旧版单外表字段。
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default)]
    pub path: String,
    #[serde(default = "default_body_mode")]
    pub body_mode: String,
    #[serde(default = "default_true")]
    pub reject_empty: bool,
    #[serde(default = "default_content_source")]
    pub content_source: String,
    #[serde(default)]
    pub sql: String,
    #[serde(default)]
    pub key_column: String,
    /// 定时推送（与 Eventide intervalSecs 同类）。默认关闭。
    #[serde(default)]
    pub schedule_enabled: bool,
    #[serde(default = "default_interval_secs")]
    pub interval_secs: u64,
}

fn default_method() -> String {
    "POST".into()
}
fn default_body_mode() -> String {
    "items".into()
}
fn default_auth_style() -> String {
    "bearer".into()
}
fn default_true() -> bool {
    true
}
fn default_content_source() -> String {
    "hosts".into()
}
fn default_interval_secs() -> u64 {
    300
}

impl Default for HttpPushConfig {
    fn default() -> Self {
        Self {
            auth_style: default_auth_style(),
            extra_headers: Value::Null,
            platform: String::new(),
            targets: Vec::new(),
            method: default_method(),
            path: String::new(),
            body_mode: default_body_mode(),
            reject_empty: true,
            content_source: default_content_source(),
            sql: String::new(),
            key_column: String::new(),
            schedule_enabled: false,
            interval_secs: default_interval_secs(),
        }
    }
}

impl HttpPushConfig {
    pub fn resolved_targets(&self) -> Vec<HttpPushTarget> {
        if !self.targets.is_empty() {
            return self.targets.clone();
        }
        vec![HttpPushTarget {
            name: if self.content_source.eq_ignore_ascii_case("sql") {
                "sql_lookup".into()
            } else {
                "hosts".into()
            },
            path: self.path.clone(),
            method: if self.method.trim().is_empty() {
                default_method()
            } else {
                self.method.clone()
            },
            body_mode: if self.body_mode.trim().is_empty() {
                default_body_mode()
            } else {
                self.body_mode.clone()
            },
            content_source: if self.content_source.trim().is_empty() {
                default_content_source()
            } else {
                self.content_source.clone()
            },
            sql: self.sql.clone(),
            key_column: self.key_column.clone(),
            reject_empty: self.reject_empty,
        }]
    }
}

fn content_is_sql(t: &HttpPushTarget) -> bool {
    t.content_source.eq_ignore_ascii_case("sql")
}

pub fn parse_config(raw: Option<&str>) -> HttpPushConfig {
    raw.and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default()
}

pub fn is_http_push(source_type: &str) -> bool {
    source_type.eq_ignore_ascii_case("http_push")
}

pub fn validate_targets_sql(cfg: &HttpPushConfig) -> Result<(), String> {
    for t in cfg.resolved_targets() {
        if content_is_sql(&t) {
            eventide_lookup_sync::validate_lookup_sql(&t.sql)?;
        }
    }
    Ok(())
}

fn join_url(base: &str, path: &str) -> String {
    let base = base.trim().trim_end_matches('/');
    let path = path.trim();
    if path.is_empty() {
        return base.to_string();
    }
    if path.starts_with('/') {
        format!("{base}{path}")
    } else {
        format!("{base}/{path}")
    }
}

fn hosts_from_rows(rows: &Map<String, Value>) -> Vec<Value> {
    rows.iter()
        .map(|(ip, cell)| {
            let obj = cell.as_object();
            let get = |k: &str| {
                obj.and_then(|m| m.get(k))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
            };
            let pick = |keys: &[&str]| {
                for k in keys {
                    let v = get(k);
                    if !v.is_empty() {
                        return v.to_string();
                    }
                }
                String::new()
            };
            let mut out = Map::new();
            out.insert("manageIp".into(), Value::String(ip.clone()));
            out.insert(
                "name".into(),
                Value::String(pick(&["主机名", "name", "hostname"])),
            );
            out.insert(
                "idc".into(),
                Value::String(pick(&["机房", "idc", "datacenter"])),
            );
            out.insert(
                "contacts".into(),
                Value::String(pick(&["联系人", "contacts", "owner"])),
            );
            out.insert(
                "phone".into(),
                Value::String(pick(&["电话", "phone", "mobile"])),
            );
            out.insert("email".into(), Value::String(pick(&["邮箱", "email"])));
            out.insert(
                "bizLine".into(),
                Value::String(pick(&["业务线", "bizLine", "biz"])),
            );
            if let Some(m) = obj {
                for (k, v) in m {
                    if !out.contains_key(k) {
                        out.insert(k.clone(), v.clone());
                    }
                }
            }
            Value::Object(out)
        })
        .collect()
}

pub fn build_body(target: &HttpPushTarget, rows: &Map<String, Value>) -> Value {
    let lookup = if target.name.trim().is_empty() {
        "hosts"
    } else {
        target.name.trim()
    };
    match target.body_mode.as_str() {
        "rows" => json!({
            "syncSource": SYNC_SOURCE,
            "lookup": lookup,
            "rows": rows,
        }),
        "hosts" => json!({
            "source": SYNC_SOURCE,
            "lookup": lookup,
            "hosts": hosts_from_rows(rows),
        }),
        _ => json!({
            "source": SYNC_SOURCE,
            "model": lookup,
            "lookup": lookup,
            "items": eventide_lookup_sync::rows_preview_list(rows, usize::MAX),
        }),
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpPushPreviewTarget {
    pub name: String,
    pub method: String,
    pub url: String,
    pub body_mode: String,
    pub content_source: String,
    pub row_count: usize,
    pub rows: Vec<Value>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpPushPreview {
    pub source: String,
    pub name: String,
    pub row_count: usize,
    pub targets: Vec<HttpPushPreviewTarget>,
}

pub async fn preview(pool: &db::DbPool, src: &db::SyncSource) -> Result<HttpPushPreview, String> {
    let cfg = parse_config(src.pull_config.as_deref());
    let targets = cfg.resolved_targets();
    let need_hosts = targets.iter().any(|t| !content_is_sql(t));
    let host_rows = if need_hosts {
        Some(eventide_lookup_sync::collect_default_host_rows(pool).await?)
    } else {
        None
    };

    let mut out = Vec::new();
    let mut total = 0usize;
    for t in &targets {
        let (rows, err) = collect_target_rows(pool, t, host_rows.as_ref()).await;
        let n = rows.len();
        total += n;
        out.push(HttpPushPreviewTarget {
            name: display_name(t),
            method: t.method.to_uppercase(),
            url: join_url(&src.api_url, &t.path),
            body_mode: t.body_mode.clone(),
            content_source: if content_is_sql(t) {
                "sql".into()
            } else {
                "hosts".into()
            },
            row_count: n,
            rows: eventide_lookup_sync::rows_preview_list(&rows, 3000),
            error: err,
        });
    }
    Ok(HttpPushPreview {
        source: src.code.clone(),
        name: src.name.clone(),
        row_count: total,
        targets: out,
    })
}

async fn collect_target_rows(
    pool: &db::DbPool,
    t: &HttpPushTarget,
    host_rows: Option<&Map<String, Value>>,
) -> (Map<String, Value>, Option<String>) {
    if content_is_sql(t) {
        match eventide_lookup_sync::collect_sql_lookup_rows(pool, &t.sql, &t.key_column).await {
            Ok(v) => (v, None),
            Err(e) => (Map::new(), Some(e)),
        }
    } else if let Some(rows) = host_rows {
        (rows.clone(), None)
    } else {
        match eventide_lookup_sync::collect_default_host_rows(pool).await {
            Ok(v) => (v, None),
            Err(e) => (Map::new(), Some(e)),
        }
    }
}

fn display_name(t: &HttpPushTarget) -> String {
    let n = t.name.trim();
    if n.is_empty() {
        if content_is_sql(t) {
            "sql_lookup".into()
        } else {
            "hosts".into()
        }
    } else {
        n.to_string()
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpPushTargetReport {
    pub name: String,
    pub url: String,
    pub method: String,
    pub ok: bool,
    pub skipped: bool,
    pub http_status: Option<u16>,
    pub row_count: usize,
    pub latency_ms: u64,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpPushReport {
    pub ok: bool,
    pub skipped: bool,
    pub source: String,
    pub row_count: usize,
    pub message: String,
    pub targets: Vec<HttpPushTargetReport>,
}

pub async fn push(state: &AppState, src: &db::SyncSource) -> HttpPushReport {
    let cfg = parse_config(src.pull_config.as_deref());
    let targets = cfg.resolved_targets();
    let mut report = HttpPushReport {
        ok: true,
        skipped: true,
        source: src.code.clone(),
        row_count: 0,
        message: String::new(),
        targets: Vec::new(),
    };

    let need_hosts = targets.iter().any(|t| !content_is_sql(t));
    let host_rows = if need_hosts {
        match eventide_lookup_sync::collect_default_host_rows(&state.db).await {
            Ok(v) => Some(v),
            Err(e) => {
                report.ok = false;
                report.skipped = false;
                report.message = e;
                write_channel_log(state, src, &report, None).await;
                return report;
            }
        }
    } else {
        None
    };

    let batch_id = uuid::Uuid::new_v4().to_string();
    for t in &targets {
        let (one, rows) = push_target(state, src, &cfg, t, host_rows.as_ref()).await;
        write_target_log(state, src, &batch_id, t, &one, &rows).await;
        report.row_count += one.row_count;
        if !one.ok && !one.skipped {
            report.ok = false;
        }
        if !one.skipped {
            report.skipped = false;
        }
        report.targets.push(one);
    }

    if report.targets.is_empty() {
        report.ok = false;
        report.skipped = false;
        report.message = "没有配置外表".into();
    } else if report.ok {
        report.message = format!(
            "已推送 {} 张外表、{} 行",
            report.targets.len(),
            report.row_count
        );
    } else {
        report.message = "部分或全部外表同步失败".into();
    }

    let status = if report.ok {
        "success"
    } else if report.skipped {
        "skipped"
    } else if report.targets.iter().any(|t| t.ok) {
        "partial"
    } else {
        "failed"
    };
    let _ = db::update_sync_source_status(&state.db, &src.code, report.row_count as i32, status)
        .await;
    tracing::info!(
        target: "http_outbound",
        source = %src.code,
        platform = %cfg.platform,
        ok = report.ok,
        targets = report.targets.len(),
        rows = report.row_count,
        "http outbound finished"
    );
    report
}

async fn push_target(
    state: &AppState,
    src: &db::SyncSource,
    cfg: &HttpPushConfig,
    t: &HttpPushTarget,
    host_rows: Option<&Map<String, Value>>,
) -> (HttpPushTargetReport, Map<String, Value>) {
    let method = t.method.to_uppercase();
    let url = join_url(&src.api_url, &t.path);
    let mut one = HttpPushTargetReport {
        name: display_name(t),
        url: url.clone(),
        method: method.clone(),
        ok: false,
        skipped: false,
        http_status: None,
        row_count: 0,
        latency_ms: 0,
        message: String::new(),
    };

    let (rows, err) = collect_target_rows(&state.db, t, host_rows).await;
    one.row_count = rows.len();
    if let Some(e) = err {
        one.message = e;
        return (one, rows);
    }
    if rows.is_empty() && t.reject_empty {
        one.skipped = true;
        one.message = "rows 为空，跳过推送（防止清空对端）".into();
        return (one, rows);
    }
    if method != "POST" && method != "PUT" {
        one.message = format!("出站只允许 POST/PUT，当前是 {method}");
        return (one, rows);
    }

    let body = build_body(t, &rows);
    let started = std::time::Instant::now();
    let mut req = match method.as_str() {
        "PUT" => state.client.put(&url),
        _ => state.client.post(&url),
    };
    req = req.header("Content-Type", "application/json");
    match cfg.auth_style.as_str() {
        "none" => {}
        "axleops" if !src.api_token.is_empty() => {
            req = req
                .header("Authorization", format!("Bearer {}", src.api_token))
                .header("X-AxleOps-Token", src.api_token.clone());
        }
        _ if !src.api_token.is_empty() => {
            req = req.header("Authorization", format!("Bearer {}", src.api_token));
        }
        _ => {}
    }
    if let Some(extra) = cfg.extra_headers.as_object() {
        for (k, v) in extra {
            if let Some(s) = v.as_str() {
                if !s.is_empty() {
                    req = req.header(k, s);
                }
            }
        }
    }

    match req.json(&body).send().await {
        Ok(resp) => {
            one.http_status = Some(resp.status().as_u16());
            one.latency_ms = started.elapsed().as_millis() as u64;
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            if status.is_success() {
                one.ok = true;
                one.message = format!("已推送 {} 行", one.row_count);
            } else {
                let snippet: String = text.chars().take(240).collect();
                one.message = format!("对端返回 {status}: {snippet}");
            }
        }
        Err(e) => {
            one.latency_ms = started.elapsed().as_millis() as u64;
            one.message = format!("调用失败: {e}");
        }
    }
    (one, rows)
}

async fn write_target_log(
    state: &AppState,
    src: &db::SyncSource,
    batch_id: &str,
    t: &HttpPushTarget,
    one: &HttpPushTargetReport,
    rows: &Map<String, Value>,
) {
    let snapshot = eventide_lookup_sync::rows_preview_list(rows, SNAPSHOT_LIMIT);
    let truncated = rows.len() > SNAPSHOT_LIMIT;
    let status = if one.ok {
        "success"
    } else if one.skipped {
        "skipped"
    } else {
        "failed"
    };
    let payload = json!({
        "lookup": one.name,
        "rowCount": one.row_count,
        "httpStatus": one.http_status,
        "latencyMs": one.latency_ms,
        "url": one.url,
        "method": one.method,
        "contentSource": t.content_source,
        "rows": snapshot,
        "truncated": truncated,
    });
    let _ = db::insert_sync_log(
        &state.db,
        &src.code,
        batch_id,
        "push",
        &one.name,
        &one.name,
        None,
        &one.name,
        status,
        &one.message,
        Some(&payload),
    )
    .await;
}

async fn write_channel_log(
    state: &AppState,
    src: &db::SyncSource,
    report: &HttpPushReport,
    rows: Option<&Map<String, Value>>,
) {
    let snapshot = rows
        .map(|r| eventide_lookup_sync::rows_preview_list(r, SNAPSHOT_LIMIT))
        .unwrap_or_default();
    let payload = json!({
        "rowCount": report.row_count,
        "message": report.message,
        "rows": snapshot,
    });
    let batch_id = uuid::Uuid::new_v4().to_string();
    let _ = db::insert_sync_log(
        &state.db,
        &src.code,
        &batch_id,
        "push",
        "host",
        &src.code,
        None,
        &src.name,
        "failed",
        &report.message,
        Some(&payload),
    )
    .await;
    let _ = db::update_sync_source_status(&state.db, &src.code, 0, "failed").await;
}

/// 出站定时推送：每个启用了 `scheduleEnabled` 的通道按 intervalSecs 推一轮。
pub async fn start_scheduler(state: std::sync::Arc<AppState>) {
    tracing::info!(target: "http_outbound", "scheduler loop online");
    let mut last: std::collections::HashMap<String, std::time::Instant> =
        std::collections::HashMap::new();
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        let sources = match db::list_sync_sources(&state.db).await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(target: "http_outbound", "list sources failed: {e}");
                continue;
            }
        };
        for src in sources {
            if !is_http_push(&src.source_type) || !src.is_enabled() {
                last.remove(&src.code);
                continue;
            }
            let cfg = parse_config(src.pull_config.as_deref());
            if !cfg.schedule_enabled || src.api_url.trim().is_empty() {
                last.remove(&src.code);
                continue;
            }
            let interval = std::time::Duration::from_secs(cfg.interval_secs.max(30));
            match last.get(&src.code) {
                None => {
                    last.insert(src.code.clone(), std::time::Instant::now());
                }
                Some(t) if t.elapsed() >= interval => {
                    last.insert(src.code.clone(), std::time::Instant::now());
                    tracing::info!(
                        target: "http_outbound",
                        source = %src.code,
                        interval_secs = cfg.interval_secs.max(30),
                        "scheduled push"
                    );
                    let report = push(&state, &src).await;
                    if !report.ok {
                        tracing::warn!(
                            target: "http_outbound",
                            source = %src.code,
                            message = %report.message,
                            "scheduled push failed"
                        );
                    }
                }
                Some(_) => {}
            }
        }
    }
}
