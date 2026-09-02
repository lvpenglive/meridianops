//! 日志代理 API：通过 HTTP POST SQL 直接查询 ClickHouse。
//!
//! 三层检索能力（详见 README「日志查询能力」）：
//! - GET /api/logs          基础检索（L1）：WHERE + ORDER BY + LIMIT
//! - GET /api/logs/stats    聚合统计（L2）：GROUP BY service, level → 计数
//! - GET /api/logs/patterns 模式挖掘（L3）：regexp_replace 数字归一化 → GROUP BY template
//! - GET /api/logs/grafana-link  生成 Grafana Explore 跳转 URL（Loki 数据源）
//!
//! 所有 SQL 参数手动转义单引号（ClickHouse HTTP 接口不支持 prepared statement）。
//! 表结构见 README「ClickHouse 表结构」：meridianops_logs.all_logs

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::auth;
use crate::error::AppError;
use crate::routes::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/logs", get(list_logs))
        .route("/api/logs/stats", get(log_stats))
        .route("/api/logs/patterns", get(log_patterns))
        .route("/api/logs/grafana-link", get(grafana_link))
}

// ===== 请求参数 =====

#[derive(Debug, Deserialize)]
pub struct LogQuery {
    pub hostname: Option<String>,
    pub service: Option<String>,
    pub level: Option<String>, // 逗号分隔：error,warn,info
    pub keyword: Option<String>,
    pub start_time: Option<String>, // RFC3339 或 '2026-09-01 00:00:00'
    pub end_time: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    pub hostname: Option<String>,
    pub service: Option<String>,
    pub level: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub group_by: Option<String>, // service / level / hostname，默认 service
    pub top: Option<u32>,         // 返回 TOP N，默认 10
}

#[derive(Debug, Deserialize)]
pub struct PatternsQuery {
    pub hostname: Option<String>,
    pub service: Option<String>,
    pub level: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub top: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct GrafanaLinkQuery {
    pub hostname: Option<String>,
    pub level: Option<String>, // 默认 error
    pub keyword: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
}

// ===== 响应结构 =====

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogRow {
    pub timestamp: String,
    pub hostname: String,
    pub service: String,
    pub source: String,
    pub level: String,
    pub message: String,
    #[serde(default)]
    pub trace_id: Option<String>,
    #[serde(default)]
    pub ip: Option<String>,
    #[serde(default)]
    pub extra: serde_json::Value,
    #[serde(default)]
    pub raw_json: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogListResponse {
    pub total: u64,
    pub limit: u32,
    pub offset: u32,
    pub items: Vec<LogRow>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatRow {
    pub bucket: String,
    pub level: String,
    pub count: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsResponse {
    pub group_by: String,
    pub items: Vec<StatRow>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternRow {
    pub template: String,
    pub count: u64,
    pub sample: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternsResponse {
    pub items: Vec<PatternRow>,
}

// ===== ClickHouse 客户端辅助 =====

/// 转义 ClickHouse 字符串字面量中的单引号：' → \'
/// ClickHouse 默认转义风格是 backslash-style，单引号需写成 \'
fn ch_escape(s: &str) -> String {
    s.replace('\'', "\\'")
}

/// 解析逗号分隔的 level 列表，返回转义后的 IN 值列表
fn level_in_list(levels: &str) -> String {
    levels
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| format!("'{}'", ch_escape(s)))
        .collect::<Vec<_>>()
        .join(",")
}

/// 拼接 WHERE 条件片段，返回带 AND 前缀的子句
fn build_where(
    hostname: Option<&str>,
    service: Option<&str>,
    level: Option<&str>,
    keyword: Option<&str>,
    start_time: Option<&str>,
    end_time: Option<&str>,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(h) = hostname {
        if !h.trim().is_empty() {
            parts.push(format!("hostname = '{}'", ch_escape(h.trim())));
        }
    }
    if let Some(s) = service {
        if !s.trim().is_empty() {
            parts.push(format!("service = '{}'", ch_escape(s.trim())));
        }
    }
    if let Some(lv) = level {
        let trimmed = lv.trim();
        if !trimmed.is_empty() {
            let list = level_in_list(trimmed);
            if !list.is_empty() {
                parts.push(format!("level IN ({})", list));
            }
        }
    }
    if let Some(kw) = keyword {
        let trimmed = kw.trim();
        if !trimmed.is_empty() {
            parts.push(format!("message ILIKE '%{}%'", ch_escape(trimmed)));
        }
    }
    if let Some(t) = start_time {
        let trimmed = t.trim();
        if !trimmed.is_empty() {
            parts.push(format!("timestamp >= '{}'", ch_escape(trimmed)));
        }
    }
    if let Some(t) = end_time {
        let trimmed = t.trim();
        if !trimmed.is_empty() {
            parts.push(format!("timestamp <= '{}'", ch_escape(trimmed)));
        }
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", parts.join(" AND "))
    }
}

/// 通过 HTTP POST 把 SQL 发给 ClickHouse，要求返回 JSONEachRow 格式。
/// 失败返回 AppError。
async fn ch_query_json_each_row(
    state: &AppState,
    sql: &str,
) -> Result<Vec<serde_json::Value>, AppError> {
    let url = format!(
        "{}/?database={}&default_format=JSONEachRow",
        state.config.logs.clickhouse_url.trim_end_matches('/'),
        urlencode(&state.config.logs.clickhouse_database)
    );
    let timeout = std::time::Duration::from_secs(state.config.logs.query_timeout_secs);
    let resp = state
        .client
        .post(&url)
        .header("Content-Type", "text/plain")
        .timeout(timeout)
        .body(sql.to_string())
        .send()
        .await
        .map_err(|e| AppError::internal(format!("ClickHouse query failed: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::internal(format!(
            "ClickHouse HTTP {}: {}",
            status,
            body.chars().take(500).collect::<String>()
        )));
    }

    let body = resp
        .text()
        .await
        .map_err(|e| AppError::internal(format!("read ClickHouse body failed: {}", e)))?;

    // JSONEachRow：每行一个 JSON 对象，空结果返回空字符串
    let mut items = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<serde_json::Value>(line) {
            Ok(v) => items.push(v),
            Err(e) => {
                tracing::warn!("skip invalid JSONEachRow line: {} (err: {})", line, e);
            }
        }
    }
    Ok(items)
}

/// 通过 HTTP POST 发送聚合 SQL（COUNT 等返回标量），返回标量字符串
async fn ch_query_scalar(state: &AppState, sql: &str) -> Result<String, AppError> {
    let url = format!(
        "{}/?database={}",
        state.config.logs.clickhouse_url.trim_end_matches('/'),
        urlencode(&state.config.logs.clickhouse_database)
    );
    let timeout = std::time::Duration::from_secs(state.config.logs.query_timeout_secs);
    let resp = state
        .client
        .post(&url)
        .header("Content-Type", "text/plain")
        .timeout(timeout)
        .body(sql.to_string())
        .send()
        .await
        .map_err(|e| AppError::internal(format!("ClickHouse query failed: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::internal(format!(
            "ClickHouse HTTP {}: {}",
            status,
            body.chars().take(500).collect::<String>()
        )));
    }
    let text = resp
        .text()
        .await
        .map_err(|e| AppError::internal(format!("read ClickHouse body failed: {}", e)))?;
    Ok(text.trim().to_string())
}

/// 简易 URL 编码（仅处理 ClickHouse 库名等安全字符）
fn urlencode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            ' ' => "%20".to_string(),
            '/' => "%2F".to_string(),
            '?' => "%3F".to_string(),
            '#' => "%23".to_string(),
            '&' => "%26".to_string(),
            _ => c.to_string(),
        })
        .collect()
}

// ===== Handlers =====

/// L1 基础检索：GET /api/logs
async fn list_logs(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Query(q): Query<LogQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "log:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let limit = q.limit.unwrap_or(100).clamp(1, state.config.logs.max_query_rows);
    let offset = q.offset.unwrap_or(0);

    let where_clause = build_where(
        q.hostname.as_deref(),
        q.service.as_deref(),
        q.level.as_deref(),
        q.keyword.as_deref(),
        q.start_time.as_deref(),
        q.end_time.as_deref(),
    );

    // 1. 查总数
    let count_sql = format!(
        "SELECT count() FROM meridianops_logs.all_logs {}",
        where_clause
    );
    let total_str = ch_query_scalar(&state, &count_sql).await?;
    let total: u64 = total_str.parse().unwrap_or(0);

    // 2. 查分页数据
    let data_sql = format!(
        "SELECT timestamp, hostname, service, source, level, message, trace_id, ip, extra, raw_json \
         FROM meridianops_logs.all_logs {} \
         ORDER BY timestamp DESC \
         LIMIT {} OFFSET {} \
         FORMAT JSONEachRow",
        where_clause, limit, offset
    );
    let rows = ch_query_json_each_row(&state, &data_sql).await?;

    let items: Vec<LogRow> = rows
        .iter()
        .map(|v| serde_json::from_value::<LogRow>(v.clone()).unwrap_or_else(|e| {
            tracing::warn!("parse LogRow failed: {}", e);
            LogRow {
                timestamp: String::new(),
                hostname: String::new(),
                service: String::new(),
                source: String::new(),
                level: String::new(),
                message: String::new(),
                trace_id: None,
                ip: None,
                extra: serde_json::Value::Null,
                raw_json: String::new(),
            }
        }))
        .collect();

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": LogListResponse { total, limit, offset, items }
    })))
}

/// L2 聚合统计：GET /api/logs/stats
/// 按 group_by 字段分组计数，返回每个 (bucket, level) 的 count
async fn log_stats(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Query(q): Query<StatsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "log:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let group_field = match q.group_by.as_deref().unwrap_or("service") {
        "hostname" => "hostname",
        "level" => "level",
        _ => "service",
    };
    let group_label = group_field.to_string();
    let top = q.top.unwrap_or(10).clamp(1, 100);

    let where_clause = build_where(
        q.hostname.as_deref(),
        q.service.as_deref(),
        q.level.as_deref(),
        None,
        q.start_time.as_deref(),
        q.end_time.as_deref(),
    );

    // ClickHouse 子查询：先按 (bucket, level) 计数，再按 bucket 总数倒序取 TOP N
    let sql = format!(
        "SELECT bucket, level, cnt \
         FROM ( \
             SELECT {field} AS bucket, level, count() AS cnt \
             FROM meridianops_logs.all_logs {where_clause} \
             GROUP BY bucket, level \
         ) \
         WHERE bucket IN ( \
             SELECT bucket FROM ( \
                 SELECT {field} AS bucket, sum(cnt) AS total \
                 FROM ( \
                     SELECT {field} AS bucket, level, count() AS cnt \
                     FROM meridianops_logs.all_logs {where_clause} \
                     GROUP BY bucket, level \
                 ) \
                 GROUP BY bucket \
                 ORDER BY total DESC \
                 LIMIT {top} \
             ) \
         ) \
         ORDER BY bucket, level \
         FORMAT JSONEachRow",
        field = group_field,
        where_clause = where_clause,
        top = top
    );

    let rows = ch_query_json_each_row(&state, &sql).await?;

    let items: Vec<StatRow> = rows
        .iter()
        .filter_map(|v| {
            let bucket = v
                .get("bucket")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let level = v
                .get("level")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let count = v
                .get("cnt")
                .and_then(|x| x.as_u64())
                .or_else(|| v.get("cnt").and_then(|x| x.as_str()).and_then(|s| s.parse().ok()))
                .unwrap_or(0);
            if bucket.is_empty() && level.is_empty() {
                None
            } else {
                Some(StatRow {
                    bucket,
                    level,
                    count,
                })
            }
        })
        .collect();

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": StatsResponse { group_by: group_label, items }
    })))
}

/// L3 模式挖掘：GET /api/logs/patterns
/// 把 message 中的数字归一化为 N，按模板分组计数，返回 TOP N 模板
async fn log_patterns(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Query(q): Query<PatternsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "log:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let top = q.top.unwrap_or(20).clamp(1, 200);

    let where_clause = build_where(
        q.hostname.as_deref(),
        q.service.as_deref(),
        q.level.as_deref(),
        None,
        q.start_time.as_deref(),
        q.end_time.as_deref(),
    );

    // ClickHouse 归一化：把所有数字替换为 N，再按模板分组计数
    // 用 replaceRegexpAll(message, '\\d+', 'N') 得到模板
    let sql = format!(
        "SELECT template, cnt, sample \
         FROM ( \
             SELECT \
                 replaceRegexpAll(message, '\\\\d+', 'N') AS template, \
                 count() AS cnt, \
                 any(message) AS sample \
             FROM meridianops_logs.all_logs {where_clause} \
             GROUP BY template \
             ORDER BY cnt DESC \
             LIMIT {top} \
         ) \
         FORMAT JSONEachRow",
        where_clause = where_clause,
        top = top
    );

    let rows = ch_query_json_each_row(&state, &sql).await?;

    let items: Vec<PatternRow> = rows
        .iter()
        .filter_map(|v| {
            let template = v
                .get("template")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let count = v
                .get("cnt")
                .and_then(|x| x.as_u64())
                .or_else(|| v.get("cnt").and_then(|x| x.as_str()).and_then(|s| s.parse().ok()))
                .unwrap_or(0);
            let sample = v
                .get("sample")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            if template.is_empty() {
                None
            } else {
                Some(PatternRow {
                    template,
                    count,
                    sample,
                })
            }
        })
        .collect();

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": PatternsResponse { items }
    })))
}

/// 生成 Grafana Explore 跳转 URL：GET /api/logs/grafana-link
/// 返回 Loki 数据源的 Explore URL，前端用 window.open 跳转
async fn grafana_link(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Query(q): Query<GrafanaLinkQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "log:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let level = q.level.as_deref().unwrap_or("error");
    let hostname = q.hostname.as_deref().unwrap_or("");

    // 构建 LogQL 表达式
    let mut labels: Vec<String> = Vec::new();
    if !hostname.is_empty() {
        labels.push(format!("hostname=\"{}\"", ch_escape(hostname)));
    }
    if !level.is_empty() {
        labels.push(format!("level=~\"{}\"", ch_escape(level)));
    }
    let label_str = if labels.is_empty() { "{}".to_string() } else { format!("{{{}}}", labels.join(",")) };

    let mut expr = label_str.clone();
    if let Some(kw) = q.keyword.as_deref() {
        let trimmed = kw.trim();
        if !trimmed.is_empty() {
            expr.push_str(&format!(" |= \"{}\"", ch_escape(trimmed)));
        }
    }

    // 时间范围（Grafana Explore 用 left 参数）
    let from = q
        .start_time
        .as_deref()
        .unwrap_or("now-1h")
        .to_string();
    let to = q.end_time.as_deref().unwrap_or("now").to_string();

    // Grafana Explore URL：left 是 URL 编码的 JSON
    // 格式参考：https://grafana.com/docs/grafana/latest/explore/
    let left_json = serde_json::json!({
        "datasource": "Loki",
        "queries": [
            { "refId": "A", "expr": expr, "queryType": "instant" }
        ],
        "range": { "from": from, "to": to }
    });
    let left_str = serde_json::to_string(&left_json).unwrap_or_default();
    let left_encoded = urlencode(&left_str);

    // Grafana base URL 从 loki_url 推断：把 :3100 替换为 :3000（默认 Grafana 端口）
    // 实际部署时可能需要单独配置 grafana_url，这里先用启发式
    let grafana_base = state
        .config
        .logs
        .loki_url
        .replace(":3100", ":3000")
        .trim_end_matches('/')
        .to_string();

    let url = format!(
        "{}/explore?orgId=1&left={}",
        grafana_base, left_encoded
    );

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": {
            "url": url,
            "expr": expr,
            "from": from,
            "to": to
        }
    })))
}

// ============================================================
// 告警-日志自动关联（Phase 3）
// ============================================================

/// 单条关联日志（紧凑结构，存入 alert_events.clue_logs JSON 字段）
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClueLog {
    pub timestamp: String,
    pub level: String,
    pub service: String,
    pub message: String,
    #[serde(default)]
    pub trace_id: Option<String>,
    #[serde(default)]
    pub ip: Option<String>,
}

/// 给告警引擎调用：查 ClickHouse 中同 hostname + 告警时间前后 window_minutes 分钟内的
/// error/critical/fatal 级别日志，最多 limit 条，按时间升序返回。
///
/// 失败时返回空 Vec（不阻断告警链路）。ClickHouse 未配置/不可达时静默忽略。
pub async fn fetch_clue_logs(
    state: &AppState,
    hostname: &str,
    fired_at: &str,            // RFC3339 字符串，如 2026-09-02T08:30:00+00:00
    window_minutes: i64,       // 默认 5（前后各 5 分钟）
    limit: u32,                // 默认 50
) -> Vec<ClueLog> {
    if hostname.trim().is_empty() {
        return Vec::new();
    }

    // 解析 fired_at，计算前后窗口
    let fired = match chrono::DateTime::parse_from_rfc3339(fired_at) {
        Ok(t) => t.with_timezone(&chrono::Utc),
        Err(_) => match chrono::NaiveDateTime::parse_from_str(fired_at, "%Y-%m-%d %H:%M:%S") {
            Ok(naive) => chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(naive, chrono::Utc),
            Err(e) => {
                tracing::warn!(target: "clue_logs", "parse fired_at failed: {} (raw: {})", e, fired_at);
                return Vec::new();
            }
        },
    };
    let win = chrono::Duration::minutes(window_minutes.max(1));
    let ts_from = fired - win;
    let ts_to = fired + win;
    // ClickHouse DateTime64(3) 接受 'YYYY-MM-DD HH:MM:SS' 字面量
    let from_str = ts_from.format("%Y-%m-%d %H:%M:%S").to_string();
    let to_str = ts_to.format("%Y-%m-%d %H:%M:%S").to_string();

    let sql = format!(
        "SELECT toString(timestamp) AS ts, level, service, message, trace_id, ip \
         FROM meridianops_logs.all_logs \
         WHERE hostname = '{host}' \
           AND timestamp >= '{from}' \
           AND timestamp <= '{to}' \
           AND level IN ('error', 'critical', 'fatal') \
         ORDER BY timestamp ASC \
         LIMIT {limit} \
         FORMAT JSONEachRow",
        host = ch_escape(hostname.trim()),
        from = ch_escape(&from_str),
        to = ch_escape(&to_str),
        limit = limit
    );

    match ch_query_json_each_row(state, &sql).await {
        Ok(rows) => rows
            .iter()
            .filter_map(|v| {
                let ts = v
                    .get("ts")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                let level = v
                    .get("level")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                let service = v
                    .get("service")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                let message = v
                    .get("message")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                let trace_id = v
                    .get("trace_id")
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string());
                let ip = v
                    .get("ip")
                    .and_then(|x| x.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());
                if ts.is_empty() && message.is_empty() {
                    None
                } else {
                    Some(ClueLog {
                        timestamp: ts,
                        level,
                        service,
                        message,
                        trace_id,
                        ip,
                    })
                }
            })
            .collect(),
        Err(e) => {
            // ClickHouse 未配置/不可达时静默忽略，不阻断告警链路
            tracing::warn!(target: "clue_logs", "fetch clue logs failed (host={}): {}", hostname, e);
            Vec::new()
        }
    }
}

// ============================================================
// 日志告警联动（Phase 5）：ERROR+ 突增检测
// ============================================================

/// 突增主机行：主机名 + 窗口内 ERROR+ 日志数
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogSurgeRow {
    pub hostname: String,
    pub error_count: u64,
    /// 窗口内最新一条日志的时间戳（用于告警 fired_at）
    pub last_ts: String,
    /// 窗口内日志样本（最多 3 条，用于告警正文）
    pub samples: Vec<String>,
}

/// 检测最近 window_minutes 分钟内 ERROR+ 级别日志突增的主机。
///
/// 返回 error_count >= threshold 的主机列表，按 error_count 降序。
/// 失败时返回空 Vec（不阻断调度器）。
pub async fn fetch_log_surge(
    state: &AppState,
    window_minutes: u32,
    levels_csv: &str, // 如 "error,critical,fatal"
    threshold: u32,
) -> Vec<LogSurgeRow> {
    let trimmed_levels = levels_csv.trim();
    if trimmed_levels.is_empty() {
        return Vec::new();
    }
    let level_list = level_in_list(trimmed_levels);
    if level_list.is_empty() {
        return Vec::new();
    }

    let now = chrono::Utc::now();
    let win = chrono::Duration::minutes(window_minutes.max(1) as i64);
    let from = (now - win).format("%Y-%m-%d %H:%M:%S").to_string();

    // 主查询：按 hostname 分组计数 + 取最新时间戳 + 取样本（groupArray 限制 3 条）
    // ClickHouse groupArrayMax 获取数组前 N 个元素
    let sql = format!(
        "SELECT \
             hostname, \
             count() AS cnt, \
             toString(max(timestamp)) AS last_ts, \
             arrayMap(x -> toString(x), groupArrayMax(message, 3)) AS samples \
         FROM meridianops_logs.all_logs \
         WHERE timestamp >= '{from}' \
           AND level IN ({levels}) \
         GROUP BY hostname \
         HAVING cnt >= {threshold} \
         ORDER BY cnt DESC \
         FORMAT JSONEachRow",
        from = ch_escape(&from),
        levels = level_list,
        threshold = threshold
    );

    match ch_query_json_each_row(state, &sql).await {
        Ok(rows) => rows
            .iter()
            .filter_map(|v| {
                let hostname = v
                    .get("hostname")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                let error_count = v
                    .get("cnt")
                    .and_then(|x| x.as_u64())
                    .or_else(|| v.get("cnt").and_then(|x| x.as_str()).and_then(|s| s.parse().ok()))
                    .unwrap_or(0);
                let last_ts = v
                    .get("last_ts")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                let samples: Vec<String> = v
                    .get("samples")
                    .and_then(|x| x.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|x| x.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                if hostname.is_empty() || error_count < threshold as u64 {
                    None
                } else {
                    Some(LogSurgeRow {
                        hostname,
                        error_count,
                        last_ts,
                        samples,
                    })
                }
            })
            .collect(),
        Err(e) => {
            tracing::warn!(target: "log_alert", "fetch_log_surge failed: {}", e);
            Vec::new()
        }
    }
}
