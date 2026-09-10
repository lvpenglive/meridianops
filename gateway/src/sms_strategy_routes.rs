//! 告警短信策略（对齐老系统「短信策略」）
//!  - GET    /api/sms-strategies                分页列表（keyword / eventType / enabled）
//!  - POST   /api/sms-strategies                新建策略
//!  - PUT    /api/sms-strategies/:id            更新策略
//!  - PATCH  /api/sms-strategies/:id/enable     启停
//!  - DELETE /api/sms-strategies/:id            删除策略
//!
//! 命中后由 notification_engine::dispatch_event 统一分发：站内信 + 通道通知。

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, patch, post, put},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;

use crate::auth::{self, AuthUser};
use crate::error::AppError;
use crate::routes::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/sms-strategies", get(list_strategies).post(create_strategy))
        .route("/api/sms-strategies/:id", put(update_strategy).delete(delete_strategy))
        .route("/api/sms-strategies/:id/enable", patch(toggle_enable))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListQuery {
    #[serde(default = "default_page")]
    page: u32,
    #[serde(default = "default_page_size")]
    page_size: u32,
    keyword: Option<String>,
    event_type: Option<String>,
    enabled: Option<bool>,
}

fn default_page() -> u32 { 1 }
fn default_page_size() -> u32 { 20 }

async fn list_strategies(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "sms_strategy:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let page = q.page.max(1) as i64;
    let page_size = q.page_size.clamp(1, 500) as i64;
    let offset = (page - 1) * page_size;

    let mut qb = sqlx::QueryBuilder::<sqlx::MySql>::new(
        "SELECT s.id, s.event_type, s.event_sub_type, s.trigger_op, s.severity_filter, s.host_filter, s.name_keyword, \
                s.alert_group_id, g.name AS alert_group_name, s.recipient_user_ids, s.description, s.enabled, \
                s.created_by, s.created_at, s.updated_at \
         FROM alert_sms_strategies s LEFT JOIN alert_groups g ON g.id = s.alert_group_id WHERE 1=1 "
    );
    let mut cqb = sqlx::QueryBuilder::<sqlx::MySql>::new(
        "SELECT COUNT(*) AS c FROM alert_sms_strategies WHERE 1=1 "
    );

    if let Some(kw) = &q.keyword {
        if !kw.is_empty() {
            let like = format!("%{}%", kw);
            // 注意：此处 JOIN 了 alert_groups g，两表都有 description 列，必须限定别名，否则 MySQL 1052 歧义。
            qb.push(" AND (s.name_keyword LIKE "); qb.push_bind(like.clone());
            qb.push(" OR s.description LIKE "); qb.push_bind(like.clone());
            qb.push(")");
            cqb.push(" AND (name_keyword LIKE "); cqb.push_bind(like.clone());
            cqb.push(" OR description LIKE "); cqb.push_bind(like.clone());
            cqb.push(")");
        }
    }
    if let Some(et) = &q.event_type {
        if !et.is_empty() {
            qb.push(" AND event_type = "); qb.push_bind(et);
            cqb.push(" AND event_type = "); cqb.push_bind(et);
        }
    }
    if let Some(en) = q.enabled {
        // 限定 s.enabled：JOIN 的 alert_groups g 也有 enabled 列，否则歧义。
        qb.push(" AND s.enabled = "); qb.push_bind(en);
        cqb.push(" AND enabled = "); cqb.push_bind(en);
    }

    let total: i64 = cqb.build().fetch_one(&state.db).await
        .and_then(|r| r.try_get::<i64, _>("c"))
        .unwrap_or(0);

    // 限定 s.created_at / s.id：两表都有这些列，否则 ORDER BY 歧义。
    qb.push(" ORDER BY s.created_at DESC, s.id DESC LIMIT ");
    qb.push_bind(page_size);
    qb.push(" OFFSET ");
    qb.push_bind(offset);

    let rows = qb.build().fetch_all(&state.db).await?;

    let list: Vec<Value> = rows.iter().map(|r| {
        let rids: Option<serde_json::Value> = r.try_get::<Option<serde_json::Value>, _>("recipient_user_ids").ok().flatten();
        json!({
            "id": r.try_get::<String, _>("id").unwrap_or_default(),
            "eventType": r.try_get::<Option<String>, _>("event_type").ok().flatten().unwrap_or_default(),
            "eventSubType": r.try_get::<Option<String>, _>("event_sub_type").ok().flatten().unwrap_or_default(),
            "triggerOp": r.try_get::<String, _>("trigger_op").unwrap_or_else(|_| "eq".to_string()),
            "severityFilter": r.try_get::<Option<String>, _>("severity_filter").ok().flatten().unwrap_or_default(),
            "hostFilter": r.try_get::<Option<String>, _>("host_filter").ok().flatten().unwrap_or_default(),
            "nameKeyword": r.try_get::<Option<String>, _>("name_keyword").ok().flatten().unwrap_or_default(),
            "alertGroupId": r.try_get::<Option<String>, _>("alert_group_id").ok().flatten(),
            "alertGroupName": r.try_get::<Option<String>, _>("alert_group_name").ok().flatten().unwrap_or_default(),
            "recipientUserIds": rids.unwrap_or_else(|| json!([])),
            "description": r.try_get::<Option<String>, _>("description").ok().flatten().unwrap_or_default(),
            "enabled": r.try_get::<bool, _>("enabled").unwrap_or(true),
            "createdBy": r.try_get::<String, _>("created_by").unwrap_or_default(),
            "createdAt": r.try_get::<String, _>("created_at").unwrap_or_default(),
            "updatedAt": r.try_get::<String, _>("updated_at").unwrap_or_default(),
        })
    }).collect();

    Ok(Json(json!({
        "code": 0,
        "data": { "list": list, "total": total, "page": page, "pageSize": page_size }
    })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StrategyReq {
    event_type: Option<String>,
    event_sub_type: Option<String>,
    #[serde(default = "default_op")]
    trigger_op: String,
    severity_filter: Option<String>,
    host_filter: Option<String>,
    name_keyword: Option<String>,
    alert_group_id: Option<String>,
    recipient_user_ids: Vec<String>,
    description: Option<String>,
    #[serde(default = "default_enabled")]
    enabled: bool,
}

fn default_op() -> String { "eq".to_string() }
fn default_enabled() -> bool { true }

/// 归一化 trigger_op，只允许合法运算符，其余回退 eq
fn norm_op(op: &str) -> String {
    match op {
        "eq" | "gte" | "gt" | "lte" | "lt" => op.to_string(),
        _ => "eq".to_string(),
    }
}

async fn create_strategy(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(req): Json<StrategyReq>,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "sms_strategy:manage")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let rids = serde_json::to_string(&req.recipient_user_ids).unwrap_or_else(|_| "[]".to_string());
    let event_type = req.event_type.as_deref().unwrap_or("").trim().to_string();
    let event_sub_type = req.event_sub_type.as_deref().unwrap_or("").trim().to_string();
    let op = norm_op(&req.trigger_op);

    sqlx::query(
        "INSERT INTO alert_sms_strategies \
         (id, event_type, event_sub_type, trigger_op, severity_filter, host_filter, name_keyword, \
          alert_group_id, recipient_user_ids, description, enabled, created_by, created_at, updated_at) \
         VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
    )
    .bind(&id)
    .bind(if event_type.is_empty() { None } else { Some(&event_type) })
    .bind(if event_sub_type.is_empty() { None } else { Some(&event_sub_type) })
    .bind(&op)
    .bind(req.severity_filter.as_deref())
    .bind(req.host_filter.as_deref())
    .bind(req.name_keyword.as_deref())
    .bind(req.alert_group_id.as_deref())
    .bind(&rids)
    .bind(req.description.as_deref())
    .bind(req.enabled)
    .bind(&auth.0.sub)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "code": 0, "data": { "id": id } })))
}

async fn update_strategy(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<StrategyReq>,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "sms_strategy:manage")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let now = chrono::Utc::now().to_rfc3339();
    let rids = serde_json::to_string(&req.recipient_user_ids).unwrap_or_else(|_| "[]".to_string());
    let event_type = req.event_type.as_deref().unwrap_or("").trim().to_string();
    let event_sub_type = req.event_sub_type.as_deref().unwrap_or("").trim().to_string();
    let op = norm_op(&req.trigger_op);

    let result = sqlx::query(
        "UPDATE alert_sms_strategies SET \
            event_type = ?, event_sub_type = ?, trigger_op = ?, severity_filter = ?, host_filter = ?, \
            name_keyword = ?, alert_group_id = ?, recipient_user_ids = ?, description = ?, enabled = ?, updated_at = ? \
         WHERE id = ?",
    )
    .bind(if event_type.is_empty() { None } else { Some(&event_type) })
    .bind(if event_sub_type.is_empty() { None } else { Some(&event_sub_type) })
    .bind(&op)
    .bind(req.severity_filter.as_deref())
    .bind(req.host_filter.as_deref())
    .bind(req.name_keyword.as_deref())
    .bind(req.alert_group_id.as_deref())
    .bind(&rids)
    .bind(req.description.as_deref())
    .bind(req.enabled)
    .bind(&now)
    .bind(&id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("策略不存在"));
    }
    Ok(Json(json!({ "code": 0, "message": "ok" })))
}

#[derive(Debug, Deserialize)]
struct ToggleReq {
    enabled: bool,
}

async fn toggle_enable(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<ToggleReq>,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "sms_strategy:manage")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let result = sqlx::query("UPDATE alert_sms_strategies SET enabled = ?, updated_at = ? WHERE id = ?")
        .bind(req.enabled)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(&id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("策略不存在"));
    }
    Ok(Json(json!({ "code": 0, "message": "ok" })))
}

async fn delete_strategy(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "sms_strategy:manage")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let result = sqlx::query("DELETE FROM alert_sms_strategies WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("策略不存在"));
    }
    Ok(Json(json!({ "code": 0, "message": "ok" })))
}
