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
        .route("/api/sms-strategies/match", get(match_strategies))
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

// ---- 运行时匹配查询（对齐老系统 getPhones 接口）----

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MatchQuery {
    /// 事件类型：支持老系统数字（1-12）或 MeridianOps 字符串（host/database/network/...）；空=全部
    event_type: Option<String>,
    /// 事件子类（db2/oracle/tomcat/...）；空=全部
    event_sub_type: Option<String>,
    /// 事件级别：1-5 数字（MeridianOps severity 体系：1=信息,2=警告,3=一般,4=重要,5=灾难）
    event_level: Option<u8>,
    /// 事件 IP；支持逗号多值；空=不按 IP 过滤
    event_ip: Option<String>,
    /// 事件名称；空=不按名称过滤
    event_name: Option<String>,
}

/// 老系统 eventType 数字 → MeridianOps event_type 字符串
fn normalize_event_type(raw: &str) -> String {
    let s = raw.trim().to_lowercase();
    if ["host","database","network","middleware","software","storage","security",
        "application","hardware","ticket","job","other"].contains(&s.as_str()) {
        return s;
    }
    match raw.trim() {
        "1" => "host",
        "2" => "database",
        "3" => "network",
        "4" => "middleware",
        "5" => "software",
        "6" => "storage",
        "7" => "security",
        "8" => "application",
        "9" => "hardware",
        "10" => "ticket",
        "11" => "job",
        "12" => "other",
        _ => s.as_str(),
    }.to_string()
}

/// 把 severity_filter（"info"/"1"-"5" 等）解析为 u8；解析失败返回 None
fn parse_severity_num(filter: &str) -> Option<u8> {
    match filter.trim().to_lowercase().as_str() {
        "info" | "1" => Some(1),
        "warning" | "warn" | "2" => Some(2),
        "average" | "3" => Some(3),
        "important" | "4" => Some(4),
        "critical" | "5" => Some(5),
        _ => None,
    }
}

/// 判断一个事件是否命中一条策略的 5 维条件
fn strategy_matches(
    ev_type: Option<&str>,
    ev_sub_type: Option<&str>,
    ev_level: Option<u8>,
    ev_ips: &[String],
    ev_name: Option<&str>,
    row: &sqlx::mysql::MySqlRow,
) -> bool {
    // 1. event_type：策略为空则匹配全部
    if let Some(t) = ev_type {
        if let Ok(filter) = row.try_get::<Option<String>, _>("event_type") {
            if let Some(f) = filter {
                if !f.is_empty() && f != t {
                    return false;
                }
            }
        }
    }

    // 2. event_sub_type
    if let Some(st) = ev_sub_type {
        if let Ok(filter) = row.try_get::<Option<String>, _>("event_sub_type") {
            if let Some(f) = filter {
                if !f.is_empty() && f != st {
                    return false;
                }
            }
        }
    }

    // 3. severity_filter + trigger_op
    if let Some(level) = ev_level {
        if let (Ok(op), Ok(filter)) = (
            row.try_get::<String, _>("trigger_op"),
            row.try_get::<Option<String>, _>("severity_filter"),
        ) {
            if let Some(f) = filter {
                if !f.is_empty() {
                    if let Some(filter_num) = parse_severity_num(&f) {
                        let cmp = match op.as_str() {
                            "gte" => level >= filter_num,
                            "gt"  => level >  filter_num,
                            "lte" => level <= filter_num,
                            "lt"  => level <  filter_num,
                            _     => level == filter_num,
                        };
                        if !cmp { return false; }
                    }
                }
            }
        }
    }

    // 4. host_filter：逗号分隔多值，支持 % 通配
    if !ev_ips.is_empty() {
        if let Ok(filter) = row.try_get::<Option<String>, _>("host_filter") {
            if let Some(f) = filter {
                if !f.is_empty() {
                    let filters: Vec<&str> = f.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
                    let mut any_match = false;
                    for pat in &filters {
                        for ip in ev_ips {
                            if ip.contains(pat) {
                                any_match = true;
                                break;
                            }
                            if pat.contains('%') || pat.contains('_') {
                                if wildcard_match(ip, pat) {
                                    any_match = true;
                                    break;
                                }
                            }
                        }
                        if any_match { break; }
                    }
                    if !any_match { return false; }
                }
            }
        }
    }

    // 5. name_keyword：事件名称包含关键字（大小写不敏感）
    if let Some(name) = ev_name {
        if !name.is_empty() {
            if let Ok(kw) = row.try_get::<Option<String>, _>("name_keyword") {
                if let Some(k) = kw {
                    if !k.is_empty() && !name.to_lowercase().contains(&k.to_lowercase()) {
                        return false;
                    }
                }
            }
        }
    }

    true
}

/// MySQL LIKE 风格通配匹配：% → 任意序列，_ → 单字符
fn wildcard_match(text: &str, pattern: &str) -> bool {
    let tb: Vec<char> = text.chars().collect();
    let pb: Vec<char> = pattern.chars().collect();
    wild_inner(&tb, 0, &pb, 0)
}

fn wild_inner(tb: &[char], ti: usize, pb: &[char], pi: usize) -> bool {
    if pi == pb.len() { return ti == tb.len(); }
    if pb[pi] == '%' {
        let mut np = pi;
        while np < pb.len() && pb[np] == '%' { np += 1; }
        for start in ti..=tb.len() {
            if wild_inner(tb, start, pb, np) { return true; }
        }
        return false;
    }
    if ti < tb.len() {
        if pb[pi] == '_' || pb[pi] == tb[ti] {
            return wild_inner(tb, ti + 1, pb, pi + 1);
        }
    }
    false
}

async fn match_strategies(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Query(q): Query<MatchQuery>,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "sms_strategy:read")?;

    // 归一化事件类型（兼容老系统数字和 MeridianOps 字符串）
    let ev_type = q.event_type.as_deref().filter(|s| !s.is_empty()).map(normalize_event_type);
    let ev_sub_type = q.event_sub_type.as_deref().filter(|s| !s.is_empty()).map(|s| s.to_string());
    let ev_level = q.event_level;
    let ev_ips: Vec<String> = q.event_ip
        .as_deref()
        .map(|s| s.split(',').map(|ip| ip.trim().to_string()).filter(|ip| !ip.is_empty()).collect())
        .unwrap_or_default();
    let ev_name = q.event_name.as_deref().filter(|s| !s.is_empty()).map(|s| s.to_string());

    // 1. 查询所有启用的策略（按 event_type 预筛选，空值全量）
    let mut qb = sqlx::QueryBuilder::<sqlx::MySql>::new(
        "SELECT id, event_type, event_sub_type, trigger_op, severity_filter, host_filter, \
                name_keyword, recipient_user_ids \
         FROM alert_sms_strategies WHERE enabled = 1 "
    );
    if let Some(t) = &ev_type {
        qb.push(" AND (event_type = "); qb.push_bind(t.clone());
        qb.push(" OR event_type IS NULL OR event_type = '') ");
    }
    qb.push(" ORDER BY created_at DESC");

    let rows = qb.build().fetch_all(&state.db).await?;

    // 2. 逐条匹配
    let mut matched_ids: Vec<String> = Vec::new();
    let mut all_user_ids: Vec<String> = Vec::new();

    for row in &rows {
        if strategy_matches(
            ev_type.as_deref(),
            ev_sub_type.as_deref(),
            ev_level,
            &ev_ips,
            ev_name.as_deref(),
            row,
        ) {
            if let Ok(id) = row.try_get::<String, _>("id") {
                matched_ids.push(id);
            }
            if let Ok(rids) = row.try_get::<Option<serde_json::Value>, _>("recipient_user_ids") {
                if let Some(val) = rids {
                    if let Ok(arr) = serde_json::from_value::<Vec<String>>(val) {
                        all_user_ids.extend(arr);
                    }
                }
            }
        }
    }

    // 3. 去重 user_ids
    all_user_ids.sort();
    all_user_ids.dedup();

    // 4. 查用户手机号（只查在职用户）
    let mut users: Vec<Value> = Vec::new();
    if !all_user_ids.is_empty() {
        let mut qb = sqlx::QueryBuilder::<sqlx::MySql>::new(
            "SELECT id, username, display_name, mobile FROM users WHERE employment_status = 'active' AND id IN ("
        );
        {
            let mut sep = false;
            for uid in &all_user_ids {
                if sep { qb.push(", "); }
                qb.push_bind(uid);
                sep = true;
            }
        }
        qb.push(")");

        let user_rows = qb.build().fetch_all(&state.db).await?;
        for ur in &user_rows {
            let mobile: Option<String> = ur.try_get::<Option<String>, _>("mobile").ok().flatten();
            users.push(json!({
                "userId":      ur.try_get::<String, _>("id").unwrap_or_default(),
                "username":    ur.try_get::<String, _>("username").unwrap_or_default(),
                "displayName": ur.try_get::<Option<String>, _>("display_name").ok().flatten().unwrap_or_default(),
                "mobile":      mobile.clone().unwrap_or_default(),
            }));
        }
    }

    // 5. 汇总手机号（非空 + 去重）
    let mut phones: Vec<String> = users.iter()
        .filter_map(|u| u["mobile"].as_str().map(|s| s.to_string()))
        .filter(|m| !m.is_empty())
        .collect::<Vec<_>>();
    phones.sort();
    phones.dedup();

    Ok(Json(json!({
        "code": 0,
        "data": {
            "matchedStrategyIds": matched_ids,
            "users": users,
            "phones": phones,
        }
    })))
}
