//! 告警中心路由
//!
//! 提供告警事件管理（CRUD + 认领/解决/备注）、统计聚合、静默规则 CRUD。
//! 告警可关联 CMDB 资产（ci_instances.id），形成「告警→资产→工单/作业」运维闭环。
//!
//! ## 端点
//!   GET    /api/alerts/events                列出告警事件（分页+筛选）  (alert:read)
//!   GET    /api/alerts/events/:id            获取告警详情              (alert:read)
//!   POST   /api/alerts/events                新建/合并告警（去重）     (alert:create)
//!   PUT    /api/alerts/events/:id/acknowledge 领告警                  (alert:update)
//!   PUT    /api/alerts/events/:id/resolve    解决告警                  (alert:update)
//!   PUT    /api/alerts/events/:id/suppress   手动标记为静默（单条）    (alert:update)
//!   PUT    /api/alerts/events/:id/note       添加解决备注              (alert:update)
//!   DELETE /api/alerts/events/:id            删除告警                  (alert:delete)
//!   GET    /api/alerts/stats                  统计卡片数据              (alert:read)
//!   GET    /api/alerts/silences              列出静默规则              (alert:read)
//!   POST   /api/alerts/silences              新建静默规则              (alert:update)
//!   PUT    /api/alerts/silences/:id          编辑静默规则              (alert:update)
//!   DELETE /api/alerts/silences/:id          删除静默规则              (alert:update)

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{ConnectInfo, Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::Row;
use uuid::Uuid;

use crate::audit;
use crate::auth;
use crate::db;
use crate::error::AppError;
use crate::routes::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        // 告警事件
        .route("/api/alerts/events", get(list_events).post(create_event))
        .route(
            "/api/alerts/events/:id",
            axum::routing::get(get_event).delete(delete_event),
        )
        .route("/api/alerts/events/:id/acknowledge", axum::routing::put(acknowledge_event))
        .route("/api/alerts/events/:id/resolve", axum::routing::put(resolve_event))
        .route("/api/alerts/events/:id/suppress", axum::routing::put(suppress_event))
        .route("/api/alerts/events/:id/note", axum::routing::put(add_note))
        // 统计
        .route("/api/alerts/stats", get(get_stats))
        // 静默规则
        .route("/api/alerts/silences", get(list_silences).post(create_silence))
        .route(
            "/api/alerts/silences/:id",
            axum::routing::put(update_silence).delete(delete_silence),
        )
        // 接入来源概览
        .route("/api/alerts/ingress-overview", get(ingress_overview))
        // Eventide webhook 接收端（无 JWT 鉴权，用共享 token）
        .route("/api/alerts/ingress/eventide", axum::routing::post(ingress_eventide))
}

// ============ 请求 / 响应结构 ============

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub source: Option<String>,
    pub severity: Option<String>,
    pub status: Option<String>,
    pub ci_id: Option<String>,
    pub keyword: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateEventRequest {
    pub source: Option<String>,
    pub severity: String,
    pub title: String,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub labels: Option<serde_json::Value>,
    #[serde(default)]
    pub ci_id: Option<String>,
    #[serde(default)]
    pub ci_name_snapshot: Option<String>,
    /// 触发时间，可选，缺省取当前时间
    #[serde(default)]
    pub fired_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveRequest {
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteRequest {
    pub note: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSilenceRequest {
    pub name: String,
    #[serde(default)]
    pub reason: Option<String>,
    /// 匹配条件 JSON：{source:[...], severity:[...], ciId:[...], labelKey:labelVal}
    #[serde(default)]
    pub match_labels: Option<serde_json::Value>,
    pub starts_at: String,
    pub ends_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSilenceRequest {
    pub name: String,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub match_labels: Option<serde_json::Value>,
    pub starts_at: String,
    pub ends_at: String,
    #[serde(default)]
    pub active: Option<bool>,
}

// ============ 工具函数 ============

/// 计算告警去重指纹：prefix + sha256(source + ciId + title + (metric 标签可选))[:16]
/// prefix 用于区分接入路径，避免跨路径碰撞：
/// - "local:"  本地创建（API 令牌 / 人工上报）
/// - "eventide:"  Eventide Webhook 推送
fn calc_fingerprint(prefix: &str, source: &str, ci_id: &Option<String>, title: &str, labels: &Option<serde_json::Value>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    hasher.update(b"|");
    hasher.update(ci_id.as_deref().unwrap_or("").as_bytes());
    hasher.update(b"|");
    hasher.update(title.as_bytes());
    // 标签里的 metric 用于区分同类指标的不同阈值告警
    if let Some(serde_json::Value::Object(map)) = labels {
        if let Some(metric) = map.get("metric") {
            hasher.update(b"|");
            hasher.update(metric.to_string().as_bytes());
        }
    }
    let digest = hasher.finalize();
    format!("{}{}", prefix, hex::encode(&digest[..8])) // 前缀 + 16 hex 字符
}

/// 把 serde_json::Value 统一规整为可入库的 JSON 字符串
fn json_to_str(v: &Option<serde_json::Value>) -> String {
    match v {
        Some(val) if val.is_object() || val.is_array() => val.to_string(),
        Some(serde_json::Value::String(s)) => {
            // 字符串内若是 JSON，提取纯净对象字符串；否则包装为对象
            if s.starts_with('{') || s.starts_with('[') {
                serde_json::from_str::<serde_json::Value>(s)
                    .map(|x| x.to_string())
                    .unwrap_or_else(|_| serde_json::json!({"value": s}).to_string())
            } else {
                serde_json::json!({"value": s}).to_string()
            }
        }
        Some(other) => other.to_string(),
        None => "{}".to_string(),
    }
}

// ============ 告警事件 handlers ============

/// GET /api/alerts/events — 列出告警事件（分页 + 多条件筛选）
async fn list_events(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Query(q): Query<EventQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "alert:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(20).clamp(1, 200);
    let offset = (page - 1) * page_size;

    // 动态拼装 WHERE 子句（列名带 e. 前缀，与列表 JOIN 查询一致）
    let mut where_clauses: Vec<String> = Vec::new();
    let mut bind_values: Vec<String> = Vec::new();
    if let Some(s) = &q.source {
        if !s.is_empty() {
            where_clauses.push("e.source = ?".to_string());
            bind_values.push(s.clone());
        }
    }
    if let Some(s) = &q.severity {
        if !s.is_empty() {
            where_clauses.push("e.severity = ?".to_string());
            bind_values.push(s.clone());
        }
    }
    if let Some(s) = &q.status {
        if !s.is_empty() {
            // 支持逗号分隔的多状态过滤（例如 "firing,acknowledged"）
            let parts: Vec<&str> = s.split(',').map(|p| p.trim()).filter(|p| !p.is_empty()).collect();
            if parts.len() == 1 {
                where_clauses.push("e.status = ?".to_string());
                bind_values.push(parts[0].to_string());
            } else if !parts.is_empty() {
                let placeholders = parts.iter().map(|_| "?").collect::<Vec<_>>().join(",");
                where_clauses.push(format!("e.status IN ({})", placeholders));
                for p in parts {
                    bind_values.push(p.to_string());
                }
            }
        }
    }
    if let Some(s) = &q.ci_id {
        if !s.is_empty() {
            where_clauses.push("e.ci_id = ?".to_string());
            bind_values.push(s.clone());
        }
    }
    if let Some(kw) = &q.keyword {
        let kw = kw.trim();
        if !kw.is_empty() {
            where_clauses.push("(e.title LIKE ? OR e.message LIKE ? OR e.ci_name_snapshot LIKE ?)".to_string());
            let pat = format!("%{}%", kw);
            bind_values.push(pat.clone());
            bind_values.push(pat.clone());
            bind_values.push(pat);
        }
    }
    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", where_clauses.join(" AND "))
    };

    // 统计总数
    let count_sql = format!("SELECT COUNT(*) AS cnt FROM alert_events e{}", where_sql);
    let mut count_q = sqlx::query(&count_sql);
    for v in &bind_values {
        count_q = count_q.bind(v);
    }
    let total: i64 = count_q
        .fetch_one(&state.db)
        .await?
        .try_get::<i64, _>("cnt")
        .unwrap_or(0);

    // 列表查询：LEFT JOIN ci_instances 获取资产责任人(owner_id → users.username)
    let list_sql = format!(
        "SELECT e.id, e.fingerprint, e.source, e.ingress_channel, e.ingress_actor, e.severity, e.status, e.title, e.message, e.labels, e.ci_id, e.ci_name_snapshot, \
         e.fire_count, e.first_fired_at, e.fired_at, e.ends_at, e.acknowledged_by, e.acknowledged_at, e.resolved_by, e.resolved_at, \
         e.resolution_note, e.created_at, e.updated_at, \
         u.username AS contact_name \
         FROM alert_events e \
         LEFT JOIN ci_instances ci ON e.ci_id = ci.id \
         LEFT JOIN users u ON ci.owner_id = u.id{} \
         ORDER BY CASE LOWER(e.severity) \
             WHEN '5' THEN 5 WHEN 'p5' THEN 5 WHEN 'disaster' THEN 5 WHEN 'dis' THEN 5 \
             WHEN '4' THEN 4 WHEN 'p4' THEN 4 WHEN 'high' THEN 4 WHEN 'major' THEN 4 WHEN 'critical' THEN 4 WHEN 'crit' THEN 4 \
             WHEN '3' THEN 3 WHEN 'p3' THEN 3 WHEN 'average' THEN 3 WHEN 'avg' THEN 3 WHEN 'medium' THEN 3 \
             WHEN '2' THEN 2 WHEN 'p2' THEN 2 WHEN 'warning' THEN 2 WHEN 'warn' THEN 2 \
             WHEN '1' THEN 1 WHEN 'p1' THEN 1 WHEN 'information' THEN 1 WHEN 'info' THEN 1 WHEN 'notice' THEN 1 WHEN 'informational' THEN 1 \
             WHEN '0' THEN 0 WHEN 'p0' THEN 0 WHEN 'notclassified' THEN 0 WHEN 'not_classified' THEN 0 \
             ELSE 0 END DESC, \
                  e.fired_at DESC \
         LIMIT ? OFFSET ?",
        where_sql
    );
    let mut list_q = sqlx::query(&list_sql);
    for v in &bind_values {
        list_q = list_q.bind(v);
    }
    list_q = list_q.bind(page_size as i64).bind(offset as i64);
    let rows = list_q.fetch_all(&state.db).await?;

    let items: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let labels_val: serde_json::Value = r
                .try_get::<Option<serde_json::Value>, _>("labels")
                .unwrap_or(None)
                .unwrap_or(serde_json::Value::Null);
            serde_json::json!({
                "id": r.try_get::<String, _>("id").unwrap_or_default(),
                "fingerprint": r.try_get::<String, _>("fingerprint").unwrap_or_default(),
                "source": r.try_get::<String, _>("source").unwrap_or_default(),
                "ingressChannel": r.try_get::<String, _>("ingress_channel").unwrap_or_else(|_| "manual".to_string()),
                "ingressActor": r.try_get::<Option<String>, _>("ingress_actor").unwrap_or(None),
                "severity": r.try_get::<String, _>("severity").unwrap_or_default(),
                "status": r.try_get::<String, _>("status").unwrap_or_default(),
                "title": r.try_get::<String, _>("title").unwrap_or_default(),
                "message": r.try_get::<Option<String>, _>("message").unwrap_or(None),
                "labels": labels_val,
                "ciId": r.try_get::<Option<String>, _>("ci_id").unwrap_or(None),
                "ciName": r.try_get::<Option<String>, _>("ci_name_snapshot").unwrap_or(None),
                "fireCount": r.try_get::<i64, _>("fire_count").unwrap_or(0),
                "firstFiredAt": r.try_get::<String, _>("first_fired_at").unwrap_or_default(),
                "firedAt": r.try_get::<String, _>("fired_at").unwrap_or_default(),
                "endsAt": r.try_get::<Option<String>, _>("ends_at").unwrap_or(None),
                "acknowledgedBy": r.try_get::<Option<String>, _>("acknowledged_by").unwrap_or(None),
                "acknowledgedAt": r.try_get::<Option<String>, _>("acknowledged_at").unwrap_or(None),
                "resolvedBy": r.try_get::<Option<String>, _>("resolved_by").unwrap_or(None),
                "resolvedAt": r.try_get::<Option<String>, _>("resolved_at").unwrap_or(None),
                "resolutionNote": r.try_get::<Option<String>, _>("resolution_note").unwrap_or(None),
                "createdAt": r.try_get::<String, _>("created_at").unwrap_or_default(),
                "updatedAt": r.try_get::<String, _>("updated_at").unwrap_or_default(),
                "contactName": r.try_get::<Option<String>, _>("contact_name").unwrap_or(None),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": {
            "total": total,
            "page": page,
            "pageSize": page_size,
            "items": items,
        }
    })))
}

/// GET /api/alerts/events/:id — 获取告警详情
async fn get_event(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "alert:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let row = sqlx::query(
        "SELECT id, fingerprint, source, ingress_channel, ingress_actor, severity, status, title, message, labels, ci_id, ci_name_snapshot, \
         fire_count, first_fired_at, fired_at, ends_at, acknowledged_by, acknowledged_at, resolved_by, resolved_at, \
         resolution_note, created_at, updated_at \
         FROM alert_events WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await?;

    match row {
        Some(r) => {
            let labels_val: serde_json::Value = r
                .try_get::<Option<serde_json::Value>, _>("labels")
                .unwrap_or(None)
                .unwrap_or(serde_json::Value::Null);
            Ok(Json(serde_json::json!({
                "code": 0,
                "data": {
                    "id": r.try_get::<String, _>("id").unwrap_or_default(),
                    "fingerprint": r.try_get::<String, _>("fingerprint").unwrap_or_default(),
                    "source": r.try_get::<String, _>("source").unwrap_or_default(),
                    "ingressChannel": r.try_get::<String, _>("ingress_channel").unwrap_or_else(|_| "manual".to_string()),
                    "ingressActor": r.try_get::<Option<String>, _>("ingress_actor").unwrap_or(None),
                    "severity": r.try_get::<String, _>("severity").unwrap_or_default(),
                    "status": r.try_get::<String, _>("status").unwrap_or_default(),
                    "title": r.try_get::<String, _>("title").unwrap_or_default(),
                    "message": r.try_get::<Option<String>, _>("message").unwrap_or(None),
                    "labels": labels_val,
                    "ciId": r.try_get::<Option<String>, _>("ci_id").unwrap_or(None),
                    "ciName": r.try_get::<Option<String>, _>("ci_name_snapshot").unwrap_or(None),
                    "fireCount": r.try_get::<i64, _>("fire_count").unwrap_or(0),
                    "firstFiredAt": r.try_get::<String, _>("first_fired_at").unwrap_or_default(),
                    "firedAt": r.try_get::<String, _>("fired_at").unwrap_or_default(),
                    "endsAt": r.try_get::<Option<String>, _>("ends_at").unwrap_or(None),
                    "acknowledgedBy": r.try_get::<Option<String>, _>("acknowledged_by").unwrap_or(None),
                    "acknowledgedAt": r.try_get::<Option<String>, _>("acknowledged_at").unwrap_or(None),
                    "resolvedBy": r.try_get::<Option<String>, _>("resolved_by").unwrap_or(None),
                    "resolvedAt": r.try_get::<Option<String>, _>("resolved_at").unwrap_or(None),
                    "resolutionNote": r.try_get::<Option<String>, _>("resolution_note").unwrap_or(None),
                    "createdAt": r.try_get::<String, _>("created_at").unwrap_or_default(),
                    "updatedAt": r.try_get::<String, _>("updated_at").unwrap_or_default(),
                }
            })))
        }
        None => Err(AppError::not_found("告警事件不存在")),
    }
}

/// POST /api/alerts/events — 新建告警；若 fingerprint 已存在则合并（fire_count+1，fired_at 更新，状态重置为 firing）
async fn create_event(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    Json(req): Json<CreateEventRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "alert:create")?;
    crate::license_routes::require_active_license(&state.db).await?;

    if req.title.trim().is_empty() {
        return Err(AppError::bad("告警标题不能为空"));
    }

    let source = req.source.as_deref().unwrap_or("manual").trim();
    if source.is_empty() {
        return Err(AppError::bad("告警来源不能为空"));
    }

    // 校验并归一化 severity：Zabbix 对齐 0-5 六级
    // 兼容输入：数字 0-5 / P0-P5 / disaster/critical/...，非法兜底 0 并提示
    let severity_canonical = normalize_level_canonical(Some(&req.severity));
    if req.severity.trim().is_empty() {
        return Err(AppError::bad("告警级别不能为空"));
    }
    // 兜底：如果原输入完全无法识别（被归一到 0 且原值不在合法 alias 集），给提示
    let is_known_alias = {
        let lower = req.severity.trim().to_ascii_lowercase();
        matches!(lower.as_str(),
            "0"|"1"|"2"|"3"|"4"|"5"|
            "p0"|"p1"|"p2"|"p3"|"p4"|"p5"|
            "notclassified"|"not_classified"|"not classified"|"classified"|
            "information"|"info"|"notice"|"informational"|
            "warning"|"warn"|
            "average"|"avg"|"medium"|
            "high"|"major"|"critical"|"crit"|
            "disaster"|"dis"
        )
    };
    if !is_known_alias {
        return Err(AppError::bad(&format!(
            "告警级别必须为 0(未分类)/1(信息)/2(警告)/3(一般)/4(重要)/5(灾难) 或其等价别名(P0-P5 / disaster-warning/information)，当前：{}",
            req.severity
        )));
    }

    // 决定接入渠道和接入者：
    // - JWT 用户（前端操作）→ sub 为普通用户名 → manual + 用户名
    // - API 令牌调用 → sub 格式为 "api-token:xxx" → api_token + token 名
    let (ing_channel, ing_actor): (String, Option<String>) = if let Some(name) = auth.0.sub.strip_prefix("api-token:") {
        ("api_token".to_string(), Some(name.to_string()))
    } else {
        ("manual".to_string(), Some(auth.0.sub.clone()))
    };

    // 若提供 ciId，校验资产存在并补全 ciName 快照（前端若已传 ciNameSnapshot 优先用前端值）
    let ci_id = req.ci_id.as_deref().and_then(|s| {
        if s.trim().is_empty() {
            None
        } else {
            Some(s.trim().to_string())
        }
    });
    let mut ci_name_snapshot = req.ci_name_snapshot.clone();
    if let Some(cid) = &ci_id {
        if ci_name_snapshot.is_none() {
            let row = sqlx::query("SELECT name FROM ci_instances WHERE id = ?")
                .bind(cid)
                .fetch_optional(&state.db)
                .await?;
            if let Some(r) = row {
                ci_name_snapshot = r.try_get::<Option<String>, _>("name").unwrap_or(None);
            }
        }
    }

    let now = chrono::Utc::now().to_rfc3339();
    let fired_at = req.fired_at.clone().unwrap_or_else(|| now.clone());
    let labels_str = json_to_str(&req.labels);
    let fingerprint = calc_fingerprint("local:", source, &ci_id, req.title.trim(), &req.labels);

    // 查询是否已存在同 fingerprint（仅 local: 前缀，避免与 eventide: 前缀跨路径碰撞）
    let existing = sqlx::query("SELECT id FROM alert_events WHERE fingerprint = ? AND fingerprint LIKE 'local:%'")
        .bind(&fingerprint)
        .fetch_optional(&state.db)
        .await?;
    let was_merged = existing.is_some();

    // 新建告警默认状态为 firing（自动静默匹配已移除，压制改为手动 suppress 接口）
    let write_status = "firing";

    let id = if let Some(r) = existing {
        // 合并：fire_count+1，fired_at 更新，状态重置为 firing，清空认领/解决信息（新一轮触发）
        let existing_id: String = r.try_get::<String, _>("id").unwrap_or_default();
        sqlx::query(
            "UPDATE alert_events \
             SET fire_count = fire_count + 1, fired_at = ?, status = ?, \
                 acknowledged_by = NULL, acknowledged_at = NULL, \
                 resolved_by = NULL, resolved_at = NULL, resolution_note = NULL, \
                 updated_at = ? \
             WHERE id = ?",
        )
        .bind(&fired_at)
        .bind(write_status)
        .bind(&now)
        .bind(&existing_id)
        .execute(&state.db)
        .await?;
        existing_id
    } else {
        // 新建：写入接入渠道和接入者
        let new_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO alert_events \
             (id, fingerprint, source, ingress_channel, ingress_actor, severity, status, title, message, labels, ci_id, ci_name_snapshot, \
              fire_count, first_fired_at, fired_at, acknowledged_by, acknowledged_at, resolved_by, resolved_at, \
              resolution_note, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, ?, ?, NULL, NULL, NULL, NULL, NULL, ?, ?)",
        )
        .bind(&new_id)
        .bind(&fingerprint)
        .bind(source)
        .bind(&ing_channel)
        .bind(ing_actor.as_deref())
        .bind(&severity_canonical)
        .bind(write_status)
        .bind(req.title.trim())
        .bind(req.message.as_deref())
        .bind(&labels_str)
        .bind(ci_id.as_deref())
        .bind(ci_name_snapshot.as_deref())
        .bind(&fired_at)
        .bind(&fired_at)
        .bind(&now)
        .bind(&now)
        .execute(&state.db)
        .await?;
        new_id
    };

    let detail = serde_json::json!({
        "id": id, "fingerprint": fingerprint, "source": source,
        "severity": severity_canonical, "title": req.title,
        "merged": was_merged,
    });
    let ip = audit::extract_ip(&headers, Some(addr));
    audit::log_async(
        &state.db, &auth, "create_alert_event", "alert_events",
        &id, Some(&detail), &ip, "success",
    ).await;

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": { "id": id, "fingerprint": fingerprint, "merged": was_merged }
    })))
}

/// PUT /api/alerts/events/:id/acknowledge — 认领告警
async fn acknowledge_event(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "alert:update")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let now = chrono::Utc::now().to_rfc3339();
    let actor = &auth.0.sub;
    let result = sqlx::query(
        "UPDATE alert_events SET status = 'acknowledged', acknowledged_by = ?, acknowledged_at = ?, updated_at = ? \
         WHERE id = ? AND status IN ('firing', 'suppressed')",
    )
    .bind(actor)
    .bind(&now)
    .bind(&now)
    .bind(&id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        // 检查是否存在
        let exists = sqlx::query("SELECT 1 FROM alert_events WHERE id = ?")
            .bind(&id)
            .fetch_optional(&state.db)
            .await?;
        if exists.is_none() {
            return Err(AppError::not_found("告警事件不存在"));
        }
        return Err(AppError::bad("当前状态不允许认领（已解决或已认领）"));
    }

    let detail = serde_json::json!({ "id": id, "by": actor, "at": now });
    let ip = audit::extract_ip(&headers, Some(addr));
    audit::log_async(
        &state.db, &auth, "acknowledge_alert", "alert_events",
        &id, Some(&detail), &ip, "success",
    ).await;

    Ok(Json(serde_json::json!({ "code": 0, "data": { "id": id } })))
}

/// PUT /api/alerts/events/:id/resolve — 解决告警
async fn resolve_event(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<ResolveRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "alert:update")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let now = chrono::Utc::now().to_rfc3339();
    let actor = &auth.0.sub;
    let result = sqlx::query(
        "UPDATE alert_events SET status = 'resolved', resolved_by = ?, resolved_at = ?, resolution_note = ?, \
         acknowledged_by = COALESCE(acknowledged_by, ?), acknowledged_at = COALESCE(acknowledged_at, ?), \
         updated_at = ? \
         WHERE id = ? AND status <> 'resolved'",
    )
    .bind(actor)
    .bind(&now)
    .bind(req.note.as_deref())
    .bind(actor)
    .bind(&now)
    .bind(&now)
    .bind(&id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        let exists = sqlx::query("SELECT 1 FROM alert_events WHERE id = ?")
            .bind(&id)
            .fetch_optional(&state.db)
            .await?;
        if exists.is_none() {
            return Err(AppError::not_found("告警事件不存在"));
        }
        return Err(AppError::bad("告警已解决，无需重复操作"));
    }

    let detail = serde_json::json!({ "id": id, "by": actor, "at": now, "note": req.note });
    let ip = audit::extract_ip(&headers, Some(addr));
    audit::log_async(
        &state.db, &auth, "resolve_alert", "alert_events",
        &id, Some(&detail), &ip, "success",
    ).await;

    Ok(Json(serde_json::json!({ "code": 0, "data": { "id": id } })))
}

/// PUT /api/alerts/events/:id/suppress — 手动标记单条告警为静默（值班临时压制，不做自动匹配）
/// 仅可对 firing/acknowledged 状态操作；不改变已解决/已静默的状态。
/// 被 suppress 的告警状态变为 suppressed，列表中依然保留，可再次认领或解决。
async fn suppress_event(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "alert:update")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let now = chrono::Utc::now().to_rfc3339();
    let actor = &auth.0.sub;
    let result = sqlx::query(
        "UPDATE alert_events SET status = 'suppressed', updated_at = ? \
         WHERE id = ? AND status IN ('firing', 'acknowledged')",
    )
    .bind(&now)
    .bind(&id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        let exists = sqlx::query("SELECT 1 FROM alert_events WHERE id = ?")
            .bind(&id)
            .fetch_optional(&state.db)
            .await?;
        if exists.is_none() {
            return Err(AppError::not_found("告警事件不存在"));
        }
        return Err(AppError::bad("当前状态不可静默（仅未解决的告警可手动静默）"));
    }

    let detail = serde_json::json!({ "id": id, "by": actor, "at": now });
    let ip = audit::extract_ip(&headers, Some(addr));
    audit::log_async(
        &state.db, &auth, "suppress_alert_manual", "alert_events",
        &id, Some(&detail), &ip, "success",
    ).await;

    Ok(Json(serde_json::json!({ "code": 0, "data": { "id": id } })))
}

/// PUT /api/alerts/events/:id/note — 仅更新解决备注（不改变状态）
async fn add_note(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<NoteRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "alert:update")?;
    crate::license_routes::require_active_license(&state.db).await?;

    if req.note.trim().is_empty() {
        return Err(AppError::bad("备注内容不能为空"));
    }

    let now = chrono::Utc::now().to_rfc3339();
    let result = sqlx::query(
        "UPDATE alert_events SET resolution_note = ?, updated_at = ? WHERE id = ?",
    )
    .bind(req.note.trim())
    .bind(&now)
    .bind(&id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("告警事件不存在"));
    }

    let detail = serde_json::json!({ "id": id, "note": req.note });
    let ip = audit::extract_ip(&headers, Some(addr));
    audit::log_async(
        &state.db, &auth, "update_alert_note", "alert_events",
        &id, Some(&detail), &ip, "success",
    ).await;

    Ok(Json(serde_json::json!({ "code": 0, "data": { "id": id } })))
}

/// DELETE /api/alerts/events/:id — 删除告警
async fn delete_event(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "alert:delete")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let result = sqlx::query("DELETE FROM alert_events WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("告警事件不存在"));
    }

    let ip = audit::extract_ip(&headers, Some(addr));
    audit::log_async(
        &state.db, &auth, "delete_alert_event", "alert_events",
        &id, None, &ip, "success",
    ).await;

    Ok(Json(serde_json::json!({ "code": 0, "data": null })))
}

// ============ 统计 ============

/// GET /api/alerts/stats — 告警统计卡片数据
/// 返回：按 severity 分组、按 status 分组、按 source 分组、近 N 天趋势
async fn get_stats(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "alert:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    // 按 severity 统计 firing/acknowledged（未解决）
    // 统一映射到 0-5 六个 canonical 级别（Zabbix 体系：0=最低, 5=最高）
    let by_severity = sqlx::query(
        "SELECT CASE LOWER(severity) \
            WHEN '5' THEN '5' WHEN 'p5' THEN '5' WHEN 'disaster' THEN '5' WHEN 'dis' THEN '5' \
            WHEN '4' THEN '4' WHEN 'p4' THEN '4' WHEN 'high' THEN '4' WHEN 'major' THEN '4' WHEN 'critical' THEN '4' WHEN 'crit' THEN '4' \
            WHEN '3' THEN '3' WHEN 'p3' THEN '3' WHEN 'average' THEN '3' WHEN 'avg' THEN '3' WHEN 'medium' THEN '3' \
            WHEN '2' THEN '2' WHEN 'p2' THEN '2' WHEN 'warning' THEN '2' WHEN 'warn' THEN '2' \
            WHEN '1' THEN '1' WHEN 'p1' THEN '1' WHEN 'information' THEN '1' WHEN 'info' THEN '1' WHEN 'notice' THEN '1' WHEN 'informational' THEN '1' \
            WHEN '0' THEN '0' WHEN 'p0' THEN '0' WHEN 'notclassified' THEN '0' WHEN 'not_classified' THEN '0' \
            ELSE '0' END AS sev_level, \
         CAST(COUNT(*) AS SIGNED) AS cnt \
         FROM alert_events WHERE status IN ('firing','acknowledged') \
         GROUP BY sev_level",
    )
    .fetch_all(&state.db)
    .await?;
    // 补全 0-5 六键（即使某级别为 0 也输出），方便前端统计卡片稳定取 key
    let mut sev_cnt: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
    for lvl in ["0", "1", "2", "3", "4", "5"] {
        sev_cnt.insert(lvl.to_string(), 0);
    }
    for r in &by_severity {
        let key = r.try_get::<String, _>("sev_level").unwrap_or_else(|_| "5".to_string());
        let cnt = r.try_get::<i64, _>("cnt").unwrap_or(0);
        *sev_cnt.entry(key).or_insert(0) += cnt;
    }
    let mut obj = serde_json::Map::new();
    for (k, v) in sev_cnt {
        obj.insert(k, serde_json::Value::from(v));
    }
    let severity_map = serde_json::Value::Object(obj);

    // 按 status 统计
    let by_status = sqlx::query(
        "SELECT status, CAST(COUNT(*) AS SIGNED) AS cnt FROM alert_events GROUP BY status",
    )
    .fetch_all(&state.db)
    .await?;
    let status_map: serde_json::Value = rows_to_json_map(by_status, "status");

    // 按 source 统计 firing/acknowledged
    let by_source = sqlx::query(
        "SELECT source, CAST(COUNT(*) AS SIGNED) AS cnt \
         FROM alert_events WHERE status IN ('firing','acknowledged') \
         GROUP BY source",
    )
    .fetch_all(&state.db)
    .await?;
    let source_map: serde_json::Value = rows_to_json_map(by_source, "source");

    // 今日新增：fired_at 在今日 UTC00:00 之后
    let today_start = {
        let now = chrono::Utc::now();
        let today = now.date_naive();
        chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(today.and_hms_opt(0, 0, 0).unwrap(), chrono::Utc)
            .to_rfc3339()
    };
    let today_new: i64 = sqlx::query("SELECT COUNT(*) AS cnt FROM alert_events WHERE fired_at >= ?")
        .bind(&today_start)
        .fetch_one(&state.db)
        .await?
        .try_get::<i64, _>("cnt")
        .unwrap_or(0);

    // 活跃告警总数（未解决）
    let active_total: i64 = sqlx::query("SELECT COUNT(*) AS cnt FROM alert_events WHERE status IN ('firing','acknowledged')")
        .fetch_one(&state.db)
        .await?
        .try_get::<i64, _>("cnt")
        .unwrap_or(0);

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": {
            "activeTotal": active_total,
            "todayNew": today_new,
            "bySeverity": severity_map,
            "byStatus": status_map,
            "bySource": source_map,
        }
    })))
}

// ============ 静默规则 handlers ============

/// GET /api/alerts/silences — 列出静默规则
async fn list_silences(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "alert:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    // 在查询时实时计算 active 状态：ends_at 已过或未到 starts_at 都视为 0
    let now = chrono::Utc::now().to_rfc3339();
    let rows = sqlx::query(
        "SELECT id, name, reason, match_labels, starts_at, ends_at, active, created_by, created_at, updated_at \
         FROM alert_silences ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await?;

    let items: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let starts_at = r.try_get::<String, _>("starts_at").unwrap_or_default();
            let ends_at = r.try_get::<String, _>("ends_at").unwrap_or_default();
            let stored_active = r.try_get::<i64, _>("active").unwrap_or(0) != 0;
            // 实时计算：已手动停用 → false；否则 ends_at 未过且 starts_at 已到 → true
            let runtime_active = stored_active && ends_at >= now && starts_at <= now;
            let labels_str = r.try_get::<Option<String>, _>("match_labels").unwrap_or(None);
            let labels_val: serde_json::Value = labels_str
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or(serde_json::Value::Null);
            serde_json::json!({
                "id": r.try_get::<String, _>("id").unwrap_or_default(),
                "name": r.try_get::<String, _>("name").unwrap_or_default(),
                "reason": r.try_get::<Option<String>, _>("reason").unwrap_or(None),
                "matchLabels": labels_val,
                "startsAt": starts_at,
                "endsAt": ends_at,
                "active": runtime_active,
                "createdBy": r.try_get::<String, _>("created_by").unwrap_or_default(),
                "createdAt": r.try_get::<String, _>("created_at").unwrap_or_default(),
                "updatedAt": r.try_get::<String, _>("updated_at").unwrap_or_default(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "code": 0, "data": items })))
}

/// POST /api/alerts/silences — 新建静默规则
async fn create_silence(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    Json(req): Json<CreateSilenceRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "alert:update")?;
    crate::license_routes::require_active_license(&state.db).await?;

    if req.name.trim().is_empty() {
        return Err(AppError::bad("规则名称不能为空"));
    }
    // 校验时间：ends_at > starts_at
    let starts = parse_rfc3339(&req.starts_at)
        .ok_or_else(|| AppError::bad("startsAt 必须为 RFC3339 时间"))?;
    let ends = parse_rfc3339(&req.ends_at)
        .ok_or_else(|| AppError::bad("endsAt 必须为 RFC3339 时间"))?;
    if ends <= starts {
        return Err(AppError::bad("结束时间必须晚于开始时间"));
    }

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let labels_str = json_to_str(&req.match_labels);
    // 新建默认 active=1（启用）；运行期查询时根据 ends_at/starts_at 实时再算，不需要按时间预置。
    let active: i8 = 1;

    sqlx::query(
        "INSERT INTO alert_silences (id, name, reason, match_labels, starts_at, ends_at, active, created_by, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(req.name.trim())
    .bind(req.reason.as_deref())
    .bind(&labels_str)
    .bind(&req.starts_at)
    .bind(&req.ends_at)
    .bind(active)
    .bind(&auth.0.sub)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await?;

    let detail = serde_json::json!({
        "id": id, "name": req.name, "startsAt": req.starts_at, "endsAt": req.ends_at,
        "matchLabels": req.match_labels,
    });
    let ip = audit::extract_ip(&headers, Some(addr));
    audit::log_async(
        &state.db, &auth, "create_alert_silence", "alert_silences",
        &id, Some(&detail), &ip, "success",
    ).await;

    Ok(Json(serde_json::json!({ "code": 0, "data": { "id": id } })))
}

/// PUT /api/alerts/silences/:id — 编辑静默规则
async fn update_silence(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<UpdateSilenceRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "alert:update")?;
    crate::license_routes::require_active_license(&state.db).await?;

    if req.name.trim().is_empty() {
        return Err(AppError::bad("规则名称不能为空"));
    }
    let starts = parse_rfc3339(&req.starts_at)
        .ok_or_else(|| AppError::bad("startsAt 必须为 RFC3339 时间"))?;
    let ends = parse_rfc3339(&req.ends_at)
        .ok_or_else(|| AppError::bad("endsAt 必须为 RFC3339 时间"))?;
    if ends <= starts {
        return Err(AppError::bad("结束时间必须晚于开始时间"));
    }

    let now = chrono::Utc::now().to_rfc3339();
    let labels_str = json_to_str(&req.match_labels);
    // active 字段：若前端传入 false（手动停用）则强制 0；否则根据时间窗判定
    let active_val: i8 = match req.active {
        Some(false) => 0,
        _ => 1,
    };

    let result = sqlx::query(
        "UPDATE alert_silences SET name = ?, reason = ?, match_labels = ?, starts_at = ?, ends_at = ?, active = ?, updated_at = ? \
         WHERE id = ?",
    )
    .bind(req.name.trim())
    .bind(req.reason.as_deref())
    .bind(&labels_str)
    .bind(&req.starts_at)
    .bind(&req.ends_at)
    .bind(active_val)
    .bind(&now)
    .bind(&id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("静默规则不存在"));
    }

    let detail = serde_json::json!({
        "id": id, "name": req.name, "startsAt": req.starts_at, "endsAt": req.ends_at,
        "active": active_val,
    });
    let ip = audit::extract_ip(&headers, Some(addr));
    audit::log_async(
        &state.db, &auth, "update_alert_silence", "alert_silences",
        &id, Some(&detail), &ip, "success",
    ).await;

    Ok(Json(serde_json::json!({ "code": 0, "data": { "id": id } })))
}

/// DELETE /api/alerts/silences/:id — 删除静默规则
async fn delete_silence(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "alert:update")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let result = sqlx::query("DELETE FROM alert_silences WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("静默规则不存在"));
    }

    let ip = audit::extract_ip(&headers, Some(addr));
    audit::log_async(
        &state.db, &auth, "delete_alert_silence", "alert_silences",
        &id, None, &ip, "success",
    ).await;

    Ok(Json(serde_json::json!({ "code": 0, "data": null })))
}

// ============ 内部工具 ============

/// 宽松解析 RFC3339 字符串为 chrono DateTime
fn parse_rfc3339(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc))
}

/// 把 `(key, cnt)` 行集合转换为 serde_json Map<String, Value>
fn rows_to_json_map(rows: Vec<sqlx::mysql::MySqlRow>, key_col: &str) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for r in &rows {
        let key = r.try_get::<String, _>(key_col).unwrap_or_default();
        let cnt = r.try_get::<i64, _>("cnt").unwrap_or(0);
        map.insert(key, serde_json::Value::from(cnt));
    }
    serde_json::Value::Object(map)
}

// ============ 接入来源概览 ============

/// GET /api/alerts/ingress-overview — 按接入渠道和接入者分组统计，API 令牌渠道附带令牌详情
async fn ingress_overview(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "alert:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    // 1) 按 (ingress_channel, ingress_actor) 分组统计
    let rows = sqlx::query(
        "SELECT ingress_channel, \
                COALESCE(ingress_actor, '') AS ing_actor, \
                CAST(COUNT(*) AS SIGNED) AS total_count, \
                CAST(SUM(CASE WHEN status = 'firing' THEN 1 ELSE 0 END) AS SIGNED) AS firing_count, \
                CAST(SUM(CASE WHEN status = 'acknowledged' THEN 1 ELSE 0 END) AS SIGNED) AS ack_count, \
                CAST(SUM(CASE WHEN status = 'resolved' THEN 1 ELSE 0 END) AS SIGNED) AS resolved_count, \
                MIN(first_fired_at) AS first_fired_at, \
                MAX(fired_at) AS last_fired_at \
         FROM alert_events \
         GROUP BY ingress_channel, COALESCE(ingress_actor, '') \
         ORDER BY ingress_channel, total_count DESC",
    )
    .fetch_all(&state.db)
    .await?;

    // 2) 如果有 api_token 渠道的行，批量查 api_tokens + users 获取令牌详情
    let api_token_actors: Vec<String> = rows
        .iter()
        .filter(|r| {
            r.try_get::<String, _>("ingress_channel").unwrap_or_default() == "api_token"
        })
        .map(|r| r.try_get::<String, _>("ing_actor").unwrap_or_default())
        .collect();

    let mut token_map: std::collections::HashMap<String, serde_json::Value> = std::collections::HashMap::new();
    if !api_token_actors.is_empty() {
        // 构造 IN 子句占位符
        let placeholders = vec!["?"; api_token_actors.len()].join(",");
        let sql = format!(
            "SELECT t.name, t.role, t.scopes, t.expires_at, t.revoked, t.last_used_at, t.created_at, \
                    u.username AS owner_name \
             FROM api_tokens t \
             LEFT JOIN users u ON t.owner_user_id = u.id \
             WHERE t.name IN ({})",
            placeholders
        );
        let mut q = sqlx::query(&sql);
        for name in &api_token_actors {
            q = q.bind(name);
        }
        let token_rows = q.fetch_all(&state.db).await?;
        for tr in &token_rows {
            let name = tr.try_get::<String, _>("name").unwrap_or_default();
            let scopes_str = tr.try_get::<Option<String>, _>("scopes").unwrap_or(None);
            let scopes_val: serde_json::Value = scopes_str
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or(serde_json::Value::Array(vec![]));
            let revoked = tr.try_get::<i8, _>("revoked").unwrap_or(0) != 0;
            let expires_at = tr.try_get::<Option<String>, _>("expires_at").unwrap_or(None);
            let now = chrono::Utc::now();
            let expired = expires_at
                .as_deref()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc) < now)
                .unwrap_or(false);
            token_map.insert(name.clone(), serde_json::json!({
                "name": name,
                "role": tr.try_get::<String, _>("role").unwrap_or_else(|_| "operator".to_string()),
                "scopes": scopes_val,
                "expiresAt": expires_at,
                "revoked": revoked,
                "expired": expired,
                "lastUsedAt": tr.try_get::<Option<String>, _>("last_used_at").unwrap_or(None),
                "createdAt": tr.try_get::<String, _>("created_at").unwrap_or_default(),
                "ownerName": tr.try_get::<Option<String>, _>("owner_name").unwrap_or(None),
            }));
        }
    }

    // 3) 组装响应
    let items: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let channel = r.try_get::<String, _>("ingress_channel").unwrap_or_default();
            let actor = r.try_get::<String, _>("ing_actor").unwrap_or_default();
            let token_info = if channel == "api_token" {
                token_map.get(&actor).cloned()
            } else {
                None
            };
            serde_json::json!({
                "ingressChannel": channel,
                "ingressActor": if actor.is_empty() { None } else { Some(actor) },
                "totalCount": r.try_get::<i64, _>("total_count").unwrap_or(0),
                "firingCount": r.try_get::<i64, _>("firing_count").unwrap_or(0),
                "acknowledgedCount": r.try_get::<i64, _>("ack_count").unwrap_or(0),
                "resolvedCount": r.try_get::<i64, _>("resolved_count").unwrap_or(0),
                "firstFiredAt": r.try_get::<Option<String>, _>("first_fired_at").unwrap_or(None),
                "lastFiredAt": r.try_get::<Option<String>, _>("last_fired_at").unwrap_or(None),
                "tokenInfo": token_info,
            })
        })
        .collect();

    // 4) 汇总各渠道总数
    let channel_summary: serde_json::Value = {
        let mut map: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
        for item in &items {
            let ch = item["ingressChannel"].as_str().unwrap_or("unknown");
            let cnt = item["totalCount"].as_i64().unwrap_or(0);
            *map.entry(ch.to_string()).or_insert(0) += cnt;
        }
        let mut obj = serde_json::Map::new();
        for (k, v) in map {
            obj.insert(k, serde_json::Value::from(v));
        }
        serde_json::Value::Object(obj)
    };

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": {
            "items": items,
            "channelSummary": channel_summary,
            "totalActors": items.len(),
        }
    })))
}

// ============ Eventide webhook 接收端 ============

/// Eventide 推送过来的告警 payload（基于 Alertmanager 标准 + Eventide 扩展字段）。
///
/// 字段说明：
/// - `alertId`：Eventide 端的告警 UUID（可选，缺省时由 MeridianOps 生成）
/// - `transition`：状态变更类型 `became_firing` / `became_resolved` / `became_acknowledged`
/// - `status`：`firing` / `resolved` / `pending` / `suppressed` 等
/// - `severity`：`0`-`5` 数字（Zabbix 体系，0=未分类 ~ 5=灾难），也接受 `disaster` / `critical` / `high` / `average` / `warning` / `information`
/// - `fingerprint`：Eventide 计算的指纹，用作幂等去重键
/// - `labels`：标签 JSON（含 alertname / instance / source / alertIp 等）
/// - `annotations`：注解 JSON（含 summary / description / dashboard 等）
/// - `value`：触发时的指标值（可空）
/// - `startsAt` / `endsAt`：RFC3339 时间
/// - `tally`：触发次数累计
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventideAlertPayload {
    #[serde(default, alias = "alert_id")]
    alert_id: Option<String>,
    #[serde(default)]
    transition: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    severity: Option<String>,
    #[serde(default)]
    fingerprint: Option<String>,
    #[serde(default)]
    labels: Option<serde_json::Value>,
    #[serde(default)]
    annotations: Option<serde_json::Value>,
    #[serde(default)]
    value: Option<f64>,
    #[serde(default, alias = "starts_at")]
    starts_at: Option<String>,
    #[serde(default, alias = "ends_at")]
    ends_at: Option<String>,
    #[serde(default)]
    tally: Option<i64>,
}

/// POST /api/alerts/ingress/eventide — 接收 Eventide webhook 推送的告警事件。
///
/// 鉴权：支持两种方式：
///   1. `Authorization: Bearer <ingress_token>` header
///   2. `?token=<ingress_token>` query 参数（兼容 Eventide webhook 不支持自定义 header 的场景）
/// token 在 system_settings 表中存储（前端「告警接入」面板可编辑），
/// 未配置时回退到 gateway-config.toml [alerts] 节。
/// 若 `ingress_enabled = false`，端点返回 404，避免暴露未配置的接收端。
async fn ingress_eventide(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Query(query): Query<std::collections::HashMap<String, String>>,
    Json(payload): Json<EventideAlertPayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    // 1) 从 alerts_runtime 读取当前配置（支持运行时更新）
    let (ingress_enabled, expected_token) = {
        let cfg = state
            .alerts_runtime
            .read()
            .map_err(|e| AppError::internal(&format!("alerts_runtime lock poisoned: {}", e)))?;
        (cfg.ingress_enabled, cfg.ingress_token.clone())
    };

    // 2) 检查 ingress 是否启用
    if !ingress_enabled {
        return Err(AppError::not_found("告警 ingress 端点未启用"));
    }

    // 3) 校验 token — 支持 header 和 query 两种方式
    if expected_token.is_empty() || expected_token.starts_with("change-me") {
        return Err(AppError::internal("ingress token 未正确配置，请在「告警接入」面板设置真实密钥"));
    }
    let provided_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer ").map(|t| t.to_string()));
    let provided = match provided_header {
        Some(t) => t,
        None => query
            .get("token")
            .cloned()
            .ok_or_else(|| AppError::unauthorized("缺少鉴权 token（需 Authorization header 或 ?token= 参数"))?,
    };
    if provided != expected_token {
        return Err(AppError::unauthorized("ingress token 不匹配"));
    }

    // 4) 解析 payload 字段
    let labels = payload.labels.clone().unwrap_or(serde_json::Value::Null);
    let annotations = payload.annotations.clone().unwrap_or(serde_json::Value::Null);

    // 从 labels 提取 source / alertname / ip / hostname
    let (source, alertname, alert_ip, hostname) = extract_label_fields(&labels);
    let title = alertname
        .clone()
        .unwrap_or_else(|| "未命名告警".to_string());
    let message = extract_summary(&annotations).or_else(|| alertname.clone());

    // severity 归一化：与 Eventide 对齐 0/1/2/3/4/5 六级 canonical，缺省 5
    let severity_canonical = normalize_level_canonical(payload.severity.as_deref());

    // fingerprint：统一加 "eventide:" 前缀避免与 local: 跨路径碰撞
    // - Eventide 提供 fingerprint：原样包装（仍加前缀，去重时用 LIKE 过滤）
    // - 否则用本地 calc（同样加前缀）
    // - 超长截断至 255 字符（VARCHAR(255) 限制）
    let fingerprint = match payload.fingerprint.clone() {
        Some(fp) if !fp.is_empty() => {
            let full = format!("eventide:{}", fp);
            if full.len() > 255 {
                let mut hasher = Sha256::new();
                hasher.update(full.as_bytes());
                let digest = hasher.finalize();
                format!("eventide:{}..{}", &full[..220], hex::encode(&digest[..4]))
            } else {
                full
            }
        }
        _ => calc_fingerprint("eventide:", &source, &None, &title, &Some(labels.clone())),
    };

    // 时间：startsAt 缺省用 now；endsAt 仅 resolved 时使用
    let now = chrono::Utc::now().to_rfc3339();
    let starts_at = payload.starts_at.clone().unwrap_or_else(|| now.clone());
    let ends_at = payload.ends_at.clone();

    // status：transition 优先，否则用 status，缺省 firing
    let (new_status, is_resolve) = match payload.transition.as_deref() {
        Some("became_resolved") => ("resolved", true),
        Some("became_acknowledged") => ("acknowledged", false),
        _ => match payload.status.as_deref() {
            Some("resolved") => ("resolved", true),
            Some(s) if !s.is_empty() => (s, false),
            _ => ("firing", false),
        },
    };

    // 4) 关联 CMDB 资产：用 alertIp 在 ci_instances 里反查（按 attrs->>ip）
    //    Eventide 标签里 ip 通常是监控对象的 IP，可与 CMDB 主机实例的 IP 属性匹配。
    let (ci_id, ci_name_snapshot) = match alert_ip.as_deref() {
        Some(ip) if !ip.is_empty() => lookup_ci_by_ip(&state.db, ip).await.unwrap_or((None, None)),
        _ => (None, hostname.clone()),
    };

    let labels_str = json_to_str(&Some(labels));
    let new_id = payload.alert_id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());

    // 5) upsert：按 fingerprint（eventide: 前缀）去重
    let existing = sqlx::query("SELECT id FROM alert_events WHERE fingerprint = ? AND fingerprint LIKE 'eventide:%'")
        .bind(&fingerprint)
        .fetch_optional(&state.db)
        .await?;
    let was_merged = existing.is_some();

    if is_resolve {
        // became_resolved：把已有告警标记为 resolved
        if let Some(r) = existing {
            let existing_id: String = r.try_get::<String, _>("id").unwrap_or_default();
            let actor = "eventide";
            sqlx::query(
                "UPDATE alert_events \
                 SET status = 'resolved', resolved_by = ?, resolved_at = ?, resolution_note = ?, \
                 ends_at = ?, updated_at = ? WHERE id = ?",
            )
            .bind(actor)
            .bind(&now)
            .bind(message.as_deref())
            .bind(ends_at.as_deref())
            .bind(&now)
            .bind(&existing_id)
            .execute(&state.db)
            .await?;
            return Ok(Json(serde_json::json!({
                "code": 0,
                "data": { "id": existing_id, "fingerprint": fingerprint, "action": "resolved", "merged": true }
            })));
        }
        // 若不存在，跳过（无法 resolve 不存在的告警）
        tracing::warn!(target: "alerts_ingress", fingerprint = %fingerprint, "eventide became_resolved 但未找到对应告警，noop");
        return Ok(Json(serde_json::json!({
            "code": 0,
            "data": { "id": null, "fingerprint": fingerprint, "action": "noop_resolved", "merged": false }
        })));
    }

    // 接入渠道信息：webhook + Eventide 通道名（从 raw_source 推断或用 "Eventide 推送"）
    let raw_source_actor = payload.labels.clone().and_then(|l| {
        l.get("source").and_then(|v| v.as_str()).map(|s| s.to_string())
    }).unwrap_or_else(|| "Eventide 推送".to_string());
    let ing_channel = "webhook".to_string();
    let ing_actor = Some(format!("Eventide/{}", raw_source_actor));

    let id = if let Some(r) = existing {
        // 合并：fire_count + tally（Eventide 累计）+ 1，fired_at 更新为 startsAt
        // 注意：合并不修改 ingress_channel / ingress_actor，保留首次接入的溯源信息
        let existing_id: String = r.try_get::<String, _>("id").unwrap_or_default();
        let increment = payload.tally.unwrap_or(1).max(1);
        sqlx::query(
            "UPDATE alert_events \
             SET fire_count = fire_count + ?, fired_at = ?, status = ?, severity = ?, \
                 acknowledged_by = NULL, acknowledged_at = NULL, \
                 resolved_by = NULL, resolved_at = NULL, resolution_note = NULL, \
                 message = COALESCE(?, message), ci_id = COALESCE(?, ci_id), \
                 ci_name_snapshot = COALESCE(?, ci_name_snapshot), labels = ?, \
                 ends_at = ?, updated_at = ? \
             WHERE id = ?",
        )
        .bind(increment)
        .bind(&starts_at)
        .bind(new_status)
        .bind(&severity_canonical)
        .bind(message.as_deref())
        .bind(ci_id.as_deref())
        .bind(ci_name_snapshot.as_deref())
        .bind(&labels_str)
        .bind(ends_at.as_deref())
        .bind(&now)
        .bind(&existing_id)
        .execute(&state.db)
        .await?;
        existing_id
    } else {
        // 新建：写入接入渠道 webhook 和接入者
        sqlx::query(
            "INSERT INTO alert_events \
             (id, fingerprint, source, ingress_channel, ingress_actor, severity, status, title, message, labels, ci_id, ci_name_snapshot, \
              fire_count, first_fired_at, fired_at, ends_at, acknowledged_by, acknowledged_at, resolved_by, resolved_at, \
              resolution_note, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, NULL, NULL, NULL, NULL, ?, ?)",
        )
        .bind(&new_id)
        .bind(&fingerprint)
        .bind(&source)
        .bind(&ing_channel)
        .bind(ing_actor.as_deref())
        .bind(&severity_canonical)
        .bind(new_status)
        .bind(&title)
        .bind(message.as_deref())
        .bind(&labels_str)
        .bind(ci_id.as_deref())
        .bind(ci_name_snapshot.as_deref())
        .bind(payload.tally.unwrap_or(1))
        .bind(&starts_at)
        .bind(&starts_at)
        .bind(ends_at.as_deref())
        .bind(&now)
        .bind(&now)
        .execute(&state.db)
        .await?;
        new_id
    };

    // 6) 审计：用 system 标记，actor=ingress:eventide
    let detail = serde_json::json!({
        "id": id, "fingerprint": fingerprint, "transition": payload.transition,
        "severity": severity_canonical, "source": source, "merged": was_merged,
    });
    let _ = db::insert_audit_log(
        &state.db,
        "ingress:eventide",
        "ingress_alert_eventide",
        "alert_events",
        &id,
        Some(&detail),
        "webhook",
        "success",
    ).await;

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": {
            "id": id, "fingerprint": fingerprint, "action": if was_merged { "merged" } else { "created" },
            "merged": was_merged,
        }
    })))
}

/// 从 labels JSON 提取 (source, alertname, alertIp, hostname)
fn extract_label_fields(labels: &serde_json::Value) -> (String, Option<String>, Option<String>, Option<String>) {
    let get_str = |key: &str| -> Option<String> {
        labels.get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    };
    // source: Eventide labels.source 通常形如 "ingress:snmptrap" → 截取冒号后部分
    let raw_source = get_str("source").or_else(|| get_str("ingress_source"));
    let source = match raw_source.as_deref() {
        Some(s) if s.starts_with("ingress:") => s.strip_prefix("ingress:").unwrap_or(s).to_string(),
        Some(s) => s.to_string(),
        None => "eventide".to_string(),
    };
    let alertname = get_str("alertname");
    let alert_ip = get_str("alertIp").or_else(|| get_str("ip")).or_else(|| get_str("instance"));
    let hostname = get_str("hostname")
        .or_else(|| get_str("trap_hosts.主机名"))
        .or_else(|| get_str("host"));
    (source, alertname, alert_ip, hostname)
}

/// 从 annotations 提取摘要文本：summary > description > hint
fn extract_summary(annotations: &serde_json::Value) -> Option<String> {
    let get_str = |key: &str| -> Option<String> {
        annotations.get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    };
    get_str("summary").or_else(|| get_str("description")).or_else(|| get_str("hint"))
}

/// 告警级别：与 Zabbix 对齐为 0-5 六级数字。
/// 0=未分类(最低) 1=信息(information) 2=警告(warning) 3=一般(average) 4=重要(high) 5=灾难(disaster/最高)
fn normalize_level_canonical(s: Option<&str>) -> String {
    let raw = match s {
        Some(v) => v.trim(),
        None => return "0".to_string(),
    };
    if raw.is_empty() {
        return "0".to_string();
    }
    let lower = raw.to_ascii_lowercase();
    match lower.as_str() {
        // 0 = 未分类（最低）
        "0" | "p0" | "notclassified" | "not_classified" | "not classified" | "classified" => "0".to_string(),
        // 1 = 信息
        "1" | "p1" | "information" | "info" | "notice" | "informational" => "1".to_string(),
        // 2 = 警告
        "2" | "p2" | "warning" | "warn" => "2".to_string(),
        // 3 = 一般
        "3" | "p3" | "average" | "avg" | "medium" => "3".to_string(),
        // 4 = 重要
        "4" | "p4" | "high" | "major" | "critical" | "crit" => "4".to_string(),
        // 5 = 灾难（最高）
        "5" | "p5" | "disaster" | "dis" => "5".to_string(),
        _ => "0".to_string(),
    }
}

/// （旧函数兼容：Eventide webhook 归一化，内部调用新 canonical 函数）
fn normalize_severity(s: Option<&str>) -> String {
    normalize_level_canonical(s)
}

/// 用 IP 在 ci_instances 表反查资产（按动态属性 attrs 里的 ip 字段匹配）。
/// 返回 (Option<ci_id>, Option<name>)
async fn lookup_ci_by_ip(pool: &sqlx::MySqlPool, ip: &str) -> anyhow::Result<(Option<String>, Option<String>)> {
    // ci_instances.attrs 是 JSON，包含各种动态属性，其中常见键为 ip / manageIp / host_ip
    // 用 JSON_EXTRACT 简单匹配（注意 attrs 是 JSON 字符串）
    let row = sqlx::query(
        "SELECT id, name FROM ci_instances \
         WHERE JSON_EXTRACT(attrs, '$.ip') = ? \
            OR JSON_EXTRACT(attrs, '$.manageIp') = ? \
            OR JSON_EXTRACT(attrs, '$.host_ip') = ? \
         LIMIT 1",
    )
    .bind(ip)
    .bind(ip)
    .bind(ip)
    .fetch_optional(pool)
    .await?;
    match row {
        Some(r) => Ok((
            Some(r.try_get::<String, _>("id").unwrap_or_default()),
            Some(r.try_get::<String, _>("name").unwrap_or_default()),
        )),
        None => Ok((None, None)),
    }
}
