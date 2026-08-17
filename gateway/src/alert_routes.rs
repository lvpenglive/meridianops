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
        .route("/api/alerts/events/:id/note", axum::routing::put(add_note))
        // 统计
        .route("/api/alerts/stats", get(get_stats))
        // 静默规则
        .route("/api/alerts/silences", get(list_silences).post(create_silence))
        .route(
            "/api/alerts/silences/:id",
            axum::routing::put(update_silence).delete(delete_silence),
        )
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

/// 计算告警去重指纹：sha256(source + ciId + title + (metric 标签可选) )[:16]
fn calc_fingerprint(source: &str, ci_id: &Option<String>, title: &str, labels: &Option<serde_json::Value>) -> String {
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
    hex::encode(&digest[..8]) // 16 个十六进制字符
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

    // 动态拼装 WHERE 子句
    let mut where_clauses: Vec<String> = Vec::new();
    let mut bind_values: Vec<String> = Vec::new();
    if let Some(s) = &q.source {
        if !s.is_empty() {
            where_clauses.push("source = ?".to_string());
            bind_values.push(s.clone());
        }
    }
    if let Some(s) = &q.severity {
        if !s.is_empty() {
            where_clauses.push("severity = ?".to_string());
            bind_values.push(s.clone());
        }
    }
    if let Some(s) = &q.status {
        if !s.is_empty() {
            where_clauses.push("status = ?".to_string());
            bind_values.push(s.clone());
        }
    }
    if let Some(s) = &q.ci_id {
        if !s.is_empty() {
            where_clauses.push("ci_id = ?".to_string());
            bind_values.push(s.clone());
        }
    }
    if let Some(kw) = &q.keyword {
        let kw = kw.trim();
        if !kw.is_empty() {
            where_clauses.push("(title LIKE ? OR message LIKE ? OR ci_name_snapshot LIKE ?)".to_string());
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
    let count_sql = format!("SELECT COUNT(*) AS cnt FROM alert_events{}", where_sql);
    let mut count_q = sqlx::query(&count_sql);
    for v in &bind_values {
        count_q = count_q.bind(v);
    }
    let total: i64 = count_q
        .fetch_one(&state.db)
        .await?
        .try_get::<i64, _>("cnt")
        .unwrap_or(0);

    // 列表查询
    let list_sql = format!(
        "SELECT id, fingerprint, source, severity, status, title, message, labels, ci_id, ci_name_snapshot, \
         fire_count, first_fired_at, fired_at, acknowledged_by, acknowledged_at, resolved_by, resolved_at, \
         resolution_note, created_at, updated_at \
         FROM alert_events{} \
         ORDER BY CASE severity WHEN 'P0' THEN 0 WHEN 'P1' THEN 1 WHEN 'P2' THEN 2 WHEN 'P3' THEN 3 ELSE 4 END, \
                  fired_at DESC \
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
            let labels_str = r.try_get::<Option<String>, _>("labels").unwrap_or(None);
            let labels_val: serde_json::Value = labels_str
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or(serde_json::Value::Null);
            serde_json::json!({
                "id": r.try_get::<String, _>("id").unwrap_or_default(),
                "fingerprint": r.try_get::<String, _>("fingerprint").unwrap_or_default(),
                "source": r.try_get::<String, _>("source").unwrap_or_default(),
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
                "acknowledgedBy": r.try_get::<Option<String>, _>("acknowledged_by").unwrap_or(None),
                "acknowledgedAt": r.try_get::<Option<String>, _>("acknowledged_at").unwrap_or(None),
                "resolvedBy": r.try_get::<Option<String>, _>("resolved_by").unwrap_or(None),
                "resolvedAt": r.try_get::<Option<String>, _>("resolved_at").unwrap_or(None),
                "resolutionNote": r.try_get::<Option<String>, _>("resolution_note").unwrap_or(None),
                "createdAt": r.try_get::<String, _>("created_at").unwrap_or_default(),
                "updatedAt": r.try_get::<String, _>("updated_at").unwrap_or_default(),
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
        "SELECT id, fingerprint, source, severity, status, title, message, labels, ci_id, ci_name_snapshot, \
         fire_count, first_fired_at, fired_at, acknowledged_by, acknowledged_at, resolved_by, resolved_at, \
         resolution_note, created_at, updated_at \
         FROM alert_events WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await?;

    match row {
        Some(r) => {
            let labels_str = r.try_get::<Option<String>, _>("labels").unwrap_or(None);
            let labels_val: serde_json::Value = labels_str
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or(serde_json::Value::Null);
            Ok(Json(serde_json::json!({
                "code": 0,
                "data": {
                    "id": r.try_get::<String, _>("id").unwrap_or_default(),
                    "fingerprint": r.try_get::<String, _>("fingerprint").unwrap_or_default(),
                    "source": r.try_get::<String, _>("source").unwrap_or_default(),
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

    // 校验 severity 合法
    let valid_severity = ["P0", "P1", "P2", "P3", "info"];
    if !valid_severity.contains(&req.severity.as_str()) {
        return Err(AppError::bad(&format!(
            "告警级别必须为 P0/P1/P2/P3/info，当前：{}",
            req.severity
        )));
    }

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
    let fingerprint = calc_fingerprint(source, &ci_id, req.title.trim(), &req.labels);

    // 查询是否已存在同 fingerprint 告警
    let existing = sqlx::query("SELECT id FROM alert_events WHERE fingerprint = ?")
        .bind(&fingerprint)
        .fetch_optional(&state.db)
        .await?;
    let was_merged = existing.is_some();

    let id = if let Some(r) = existing {
        // 合并：fire_count+1，fired_at 更新，status 重置为 firing（如果之前已认领/解决）
        let existing_id: String = r.try_get::<String, _>("id").unwrap_or_default();
        sqlx::query(
            "UPDATE alert_events \
             SET fire_count = fire_count + 1, fired_at = ?, status = 'firing', \
                 acknowledged_by = NULL, acknowledged_at = NULL, \
                 resolved_by = NULL, resolved_at = NULL, resolution_note = NULL, \
                 updated_at = ? \
             WHERE id = ?",
        )
        .bind(&fired_at)
        .bind(&now)
        .bind(&existing_id)
        .execute(&state.db)
        .await?;
        existing_id
    } else {
        // 新建
        let new_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO alert_events \
             (id, fingerprint, source, severity, status, title, message, labels, ci_id, ci_name_snapshot, \
              fire_count, first_fired_at, fired_at, acknowledged_by, acknowledged_at, resolved_by, resolved_at, \
              resolution_note, created_at, updated_at) \
             VALUES (?, ?, ?, ?, 'firing', ?, ?, ?, ?, ?, 1, ?, ?, NULL, NULL, NULL, NULL, NULL, ?, ?)",
        )
        .bind(&new_id)
        .bind(&fingerprint)
        .bind(source)
        .bind(&req.severity)
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
        "severity": req.severity, "title": req.title,
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
    let by_severity = sqlx::query(
        "SELECT severity, CAST(COUNT(*) AS SIGNED) AS cnt \
         FROM alert_events WHERE status IN ('firing','acknowledged') \
         GROUP BY severity",
    )
    .fetch_all(&state.db)
    .await?;
    let severity_map: serde_json::Value = rows_to_json_map(by_severity, "severity");

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
    let active: i8 = if req.starts_at <= now && req.ends_at >= now { 1 } else { 1 }; // 新建默认启用，运行期再算

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
