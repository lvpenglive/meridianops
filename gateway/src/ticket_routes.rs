//! Ticket Routes
//!  - POST   /api/tickets                          创建工单 + 启动流程
//!  - GET    /api/tickets                          分页列表（筛选 + 搜索）
//!  - GET    /api/tickets/kpis                     KPI 汇总
//!  - GET    /api/tickets/:id                      详情 + 运行时节点 + 评论 + 附件 + isWatched
//!  - PUT    /api/tickets/:id                      编辑元数据
//!  - DELETE /api/tickets/:id                      软删除
//!  - POST   /api/tickets/:id/assign               指定受理人
//!  - POST   /api/tickets/:id/link-alert           关联告警
//!  - POST   /api/tickets/:id/unlink-alert/:alertId  取消关联
//!  - POST   /api/tickets/:id/actions/:nodeId      approve/reject/reassign/comment
//!  - POST   /api/tickets/:id/close                强制关闭 + 审计
//!  - POST   /api/tickets/:id/cancel              取消工单
//!  - GET    /api/tickets/:id/custom-fields       获取工单自定义字段列表
//!  - PUT    /api/tickets/:id/custom-fields       批量更新/插入自定义字段
//!  - GET    /api/tickets/:id/knowledge-suggestions  知识库关联推荐
//!  - GET    /api/tickets/my-todos                我的待办（当前用户是激活节点审批人）
//!  - GET    /api/tickets/my-done                 我的已办（当前用户已操作的工单）
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::NaiveDateTime;

use axum::{
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{HeaderValue, header},
    response::Response,
    Json, Router, routing::{get, post, delete, put},
};
use serde::{Deserialize};
use serde_json::{json, Value};
use sqlx::{MySqlPool, Row};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::routes::AppState;
use crate::workflow_engine::{
    compile_definition, compute_sla_due, derive_status, generate_ticket_no, parse_dt,
    pick_next_node, resolve_approvers, CompiledNode, LfDefinition,
};

pub fn dt_str(r: &sqlx::mysql::MySqlRow, col: &str) -> String {
    r.try_get::<Option<NaiveDateTime>, _>(col)
        .ok()
        .flatten()
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default()
}

pub fn dt_opt(r: &sqlx::mysql::MySqlRow, col: &str) -> Option<String> {
    r.try_get::<Option<NaiveDateTime>, _>(col)
        .ok()
        .flatten()
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/tickets", post(create_ticket).get(list_tickets))
        .route("/api/tickets/kpis", get(ticket_kpis))
        .route("/api/tickets/stats/overview", get(ticket_stats_overview))
        .route("/api/tickets/export", get(export_tickets_csv))
        .route("/api/tickets/my-watching", get(my_watching_tickets))
        .route("/api/tickets/:id", get(get_ticket).put(update_ticket).delete(delete_ticket))
        .route("/api/tickets/:id/assign", post(assign_ticket))
        .route("/api/tickets/:id/attachments", post(upload_attachment).get(list_attachments))
        .route("/api/tickets/:id/watch", post(watch_ticket))
        .route("/api/tickets/:id/link-alert", post(link_alert))
        .route("/api/tickets/:id/unlink-alert/:alertId", post(unlink_alert))
        .route("/api/tickets/:id/actions/:nodeId", post(action_on_node))
        .route("/api/tickets/:id/close", post(close_ticket))
        .route("/api/tickets/:id/cancel", post(cancel_ticket))
        .route("/api/tickets/:id/custom-fields", get(get_custom_fields).put(update_custom_fields))
        .route("/api/tickets/:id/knowledge-suggestions", get(knowledge_suggestions))
        .route("/api/tickets/my-todos", get(my_todo_tickets))
        .route("/api/tickets/my-done", get(my_done_tickets))
        .route("/api/tickets/batch-action", post(batch_action))
        .route("/api/tickets/attachments/:id/download", get(download_attachment))
        .route("/api/tickets/attachments/:id", delete(delete_attachment))
}

#[inline]
fn uid(me: &AuthUser) -> &str { &me.0.uid }

// ---------------- 分页查询 ----------------

#[derive(Debug, Deserialize)]
pub struct PageQ {
    #[serde(default = "one")] pub page: i64,
    #[serde(default = "psize")] pub pageSize: i64,
    pub ticketType: Option<String>,
    pub status: Option<String>,
    pub priority: Option<i64>,
    pub category: Option<String>,
    pub assigneeId: Option<String>,
    pub reporterId: Option<String>,
    pub templateId: Option<String>,
    pub keyword: Option<String>,
    pub slaState: Option<String>,
    pub createdAtFrom: Option<String>,
    pub createdAtTo: Option<String>,
}
fn one() -> i64 { 1 }
fn psize() -> i64 { 20 }

#[derive(Debug, Deserialize)]
pub struct CreateTicketReq {
    pub ticketType: String,
    pub title: String,
    pub description: Option<String>,
    pub priority: i8,
    #[serde(default)] pub category: Option<String>,
    #[serde(default)] pub assigneeId: Option<String>,
    #[serde(default)] pub templateId: Option<String>,
    #[serde(default)] pub alertIds: Vec<String>,
    #[serde(default)] pub extra: Value,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTicketReq {
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: Option<i8>,
    pub category: Option<String>,
    pub status: Option<String>,
    pub assigneeId: Option<String>,
    pub resolution: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AssignReq {
    pub assigneeId: String,
    #[serde(default)] pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LinkAlertReq {
    pub alertId: String,
    #[serde(default)] pub relation: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ActionReq {
    pub decision: String, // approve / reject / reassign / comment
    pub userId: Option<String>,
    pub comment: Option<String>,
    #[serde(default)] pub reason: Option<String>,
    pub toNodeKey: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CloseReq {
    #[serde(default)] pub resolution: Option<String>,
    #[serde(default)] pub comment: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BatchActionReq {
    pub ticketIds: Vec<String>,
    pub decision: String,
    #[serde(default)] pub comment: Option<String>,
    #[serde(default)] pub userId: Option<String>,
    #[serde(default)] pub resolution: Option<String>,
    #[serde(default)] pub priority: Option<i8>,
}

pub async fn batch_action(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(req): Json<BatchActionReq>,
) -> Result<Json<Value>, AppError> {
    crate::auth::require_permission(&auth, "ticket:update")?;
    let pool = &state.db;
    let me_id = uid(&auth);
    if req.ticketIds.is_empty() {
        return Err(AppError::bad("ticketIds 不能为空"));
    }

    // ---- 批量分派 / 批量关闭 / 批量改优先级 ----
    match req.decision.as_str() {
        "assign" => {
            let target_user = req.userId.clone().ok_or_else(|| AppError::bad("缺少 userId"))?;
            let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM users WHERE id=? AND enabled=1 LIMIT 1")
                .bind(&target_user).fetch_optional(pool).await?;
            if exists.is_none() { return Err(AppError::bad("受理人不存在")); }
            let mut results: Vec<Value> = Vec::new();
            for tid in &req.ticketIds {
                let affected = sqlx::query(
                    "UPDATE tickets SET assignee_id=?, updated_at=NOW() WHERE id=? AND deleted_at IS NULL"
                ).bind(&target_user).bind(tid).execute(pool).await
                    .map(|r| r.rows_affected()).unwrap_or(0);
                if affected > 0 {
                    let cid = Uuid::new_v4().to_string();
                    let cmt = format!("批量指派给受理人 {}", target_user);
                    sqlx::query("INSERT INTO ticket_comments (id, ticket_id, user_id, action, node_key, content, created_at) VALUES (?,?,?,?,?,?,NOW())")
                        .bind(&cid).bind(tid).bind(me_id).bind("assign").bind("").bind(&cmt)
                        .execute(pool).await.ok();
                    results.push(json!({"ticketId": tid, "ok": true}));
                } else {
                    results.push(json!({"ticketId": tid, "ok": false, "error": "工单不存在"}));
                }
            }
            let ok_count = results.iter().filter(|v| v.get("ok").and_then(|v| v.as_bool()) == Some(true)).count();
            return Ok(Json(json!({"code":0, "message":"ok", "data": {"total": req.ticketIds.len(), "success": ok_count, "results": results}})));
        }
        "close" => {
            let resolution = req.resolution.clone().unwrap_or_default();
            let mut results: Vec<Value> = Vec::new();
            for tid in &req.ticketIds {
                let affected = sqlx::query(
                    "UPDATE tickets SET status='closed', closed_at=NOW(), updated_at=NOW(),
                        current_node_key='__end__', resolution=COALESCE(?, resolution)
                     WHERE id=? AND deleted_at IS NULL"
                ).bind(&resolution).bind(tid).execute(pool).await
                    .map(|r| r.rows_affected()).unwrap_or(0);
                if affected > 0 {
                    sqlx::query("UPDATE ticket_workflow_nodes SET status='done', done_at=NOW(), updated_at=NOW() WHERE ticket_id=? AND status IN ('active','pending')")
                        .bind(tid).execute(pool).await.ok();
                    let cid = Uuid::new_v4().to_string();
                    let cmt = format!("批量关闭工单{}", if resolution.is_empty() { String::new() } else { format!("：{}", resolution) });
                    sqlx::query("INSERT INTO ticket_comments (id, ticket_id, user_id, action, node_key, content, created_at) VALUES (?,?,?,?,?,?,NOW())")
                        .bind(&cid).bind(tid).bind(me_id).bind("close").bind("__end__").bind(&cmt)
                        .execute(pool).await.ok();
                    results.push(json!({"ticketId": tid, "ok": true}));
                } else {
                    results.push(json!({"ticketId": tid, "ok": false, "error": "工单不存在"}));
                }
            }
            let ok_count = results.iter().filter(|v| v.get("ok").and_then(|v| v.as_bool()) == Some(true)).count();
            return Ok(Json(json!({"code":0, "message":"ok", "data": {"total": req.ticketIds.len(), "success": ok_count, "results": results}})));
        }
        "priority" => {
            let new_priority = req.priority.ok_or_else(|| AppError::bad("缺少 priority"))?;
            if !(1..=4).contains(&new_priority) { return Err(AppError::bad("priority 必须 1..=4")); }
            let mut results: Vec<Value> = Vec::new();
            for tid in &req.ticketIds {
                let old: Option<(i8,)> = sqlx::query_as("SELECT priority FROM tickets WHERE id=? AND deleted_at IS NULL LIMIT 1")
                    .bind(tid).fetch_optional(pool).await.unwrap_or(None);
                let old_pri = old.map(|(p,)| p).unwrap_or(3);
                let affected = sqlx::query(
                    "UPDATE tickets SET priority=?, updated_at=NOW() WHERE id=? AND deleted_at IS NULL"
                ).bind(new_priority).bind(tid).execute(pool).await
                    .map(|r| r.rows_affected()).unwrap_or(0);
                if affected > 0 {
                    // 重新计算 SLA
                    let created: Option<(Option<String>,)> = sqlx::query_as(
                        "SELECT DATE_FORMAT(created_at, '%Y-%m-%dT%H:%i:%sZ') FROM tickets WHERE id=?"
                    ).bind(tid).fetch_one(pool).await.ok();
                    if let Some((Some(dt),)) = created {
                        if let Some(sla_due) = crate::workflow_engine::compute_sla_due(new_priority, &dt) {
                            if let Some(d) = crate::workflow_engine::parse_dt(&Some(sla_due)) {
                                sqlx::query("UPDATE tickets SET sla_due_at=? WHERE id=?").bind(d).bind(tid).execute(pool).await.ok();
                            }
                        }
                    }
                    let cid = Uuid::new_v4().to_string();
                    let cmt = format!("批量修改优先级：P{} -> P{}", 5 - old_pri as i64, 5 - new_priority as i64);
                    sqlx::query("INSERT INTO ticket_comments (id, ticket_id, user_id, action, node_key, content, created_at) VALUES (?,?,?,?,?,?,NOW())")
                        .bind(&cid).bind(tid).bind(me_id).bind("priority").bind("").bind(&cmt)
                        .execute(pool).await.ok();
                    results.push(json!({"ticketId": tid, "ok": true}));
                } else {
                    results.push(json!({"ticketId": tid, "ok": false, "error": "工单不存在"}));
                }
            }
            let ok_count = results.iter().filter(|v| v.get("ok").and_then(|v| v.as_bool()) == Some(true)).count();
            return Ok(Json(json!({"code":0, "message":"ok", "data": {"total": req.ticketIds.len(), "success": ok_count, "results": results}})));
        }
        _ => {}
    }

    // ---- 原有工作流节点批量审批逻辑 ----
    let mut results: Vec<Value> = Vec::new();
    for tid in &req.ticketIds {
        let row = sqlx::query(
            "SELECT wn.node_key, wn.approvers, t.current_node_key
             FROM ticket_workflow_nodes wn
             INNER JOIN tickets t ON t.id = wn.ticket_id
             WHERE t.id=? AND t.deleted_at IS NULL AND wn.status='active' LIMIT 1"
        ).bind(tid).fetch_optional(pool).await.ok().flatten();
        let Some(row) = row else {
            results.push(json!({"ticketId": tid, "ok": false, "error": "无激活节点"}));
            continue;
        };
        let node_key: String = row.try_get::<String,_>("node_key").unwrap_or_default();
        let cur_nk: Option<String> = row.try_get::<Option<String>,_>("current_node_key").unwrap_or(None);
        if cur_nk.as_deref() != Some(node_key.as_str()) {
            results.push(json!({"ticketId": tid, "ok": false, "error": "节点未激活"}));
            continue;
        }
        let approvers: Option<Value> = row.try_get::<Option<Value>,_>("approvers").ok().flatten();
        let is_admin = auth.0.role == crate::auth::Role::Admin;
        if !is_admin {
            if let Some(arr) = approvers.as_ref().and_then(|v| v.as_array()) {
                let in_list = arr.iter().any(|a| a.get("id").and_then(|v| v.as_str()) == Some(me_id));
                if !in_list {
                    results.push(json!({"ticketId": tid, "ok": false, "error": "非审批人"}));
                    continue;
                }
            }
        }
        let action_req = ActionReq {
            decision: req.decision.clone(),
            userId: None,
            comment: req.comment.clone(),
            reason: None,
            toNodeKey: None,
        };
        match action_on_node(State(state.clone()), auth.clone(), Path((tid.clone(), node_key)), Json(action_req)).await {
            Ok(resp) => {
                results.push(json!({"ticketId": tid, "ok": true, "currentNodeKey": resp.0.get("currentNodeKey"), "done": resp.0.get("currentNodeKey").and_then(|v| v.as_str()) == Some("__end__")}));
            }
            Err(e) => results.push(json!({"ticketId": tid, "ok": false, "error": e.to_string()})),
        }
    }
    let ok_count = results.iter().filter(|v| v.get("ok").and_then(|v| v.as_bool()) == Some(true)).count();
    Ok(Json(json!({"code":0, "message":"ok", "data": {"total": req.ticketIds.len(), "success": ok_count, "results": results}})))
}

pub async fn list_tickets(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Query(q): Query<PageQ>,
) -> Result<Json<Value>, AppError> {
    crate::auth::require_permission(&auth, "ticket:read")?;
    let pool = &state.db;
    let mut wc: Vec<String> = Vec::new();
    let mut binds: Vec<Value> = Vec::new();
    push_eq(&mut wc, &mut binds, "t.ticket_type", &q.ticketType);
    push_eq(&mut wc, &mut binds, "t.status", &q.status);
    if let Some(v) = q.priority { wc.push("t.priority = ?".into()); binds.push(json!(v)); }
    push_eq(&mut wc, &mut binds, "t.category", &q.category);
    push_eq(&mut wc, &mut binds, "t.assignee_id", &q.assigneeId);
    push_eq(&mut wc, &mut binds, "t.reporter_id", &q.reporterId);
    push_eq(&mut wc, &mut binds, "t.template_id", &q.templateId);
    if let Some(kw) = &q.keyword { if !kw.is_empty() {
        wc.push("(t.ticket_no LIKE ? OR t.title LIKE ?)".into());
        let p = format!("%{}%", kw); binds.push(json!(p.clone())); binds.push(json!(p));
    }}
    if let Some(s) = q.slaState.as_deref() { match s {
        "breached" => wc.push("t.sla_due_at IS NOT NULL AND t.sla_due_at < NOW() AND t.status NOT IN ('closed','cancelled')".into()),
        "safe"     => wc.push("t.sla_due_at IS NOT NULL AND t.sla_due_at >= NOW() AND t.status NOT IN ('closed','cancelled')".into()),
        "today"    => wc.push("t.sla_due_at IS NOT NULL AND DATE(t.sla_due_at)=DATE(NOW()) AND t.status NOT IN ('closed','cancelled')".into()),
        _ => {}
    }}
    if let Some(d) = &q.createdAtFrom { wc.push("t.created_at >= ?".into()); binds.push(json!(d)); }
    if let Some(d) = &q.createdAtTo   { wc.push("t.created_at <= ?".into()); binds.push(json!(d)); }
    let where_sql = if wc.is_empty() { " WHERE t.deleted_at IS NULL".into() } else { format!(" WHERE t.deleted_at IS NULL AND {}", wc.join(" AND ")) };
    let count_sql = format!("SELECT COUNT(*) AS cnt FROM tickets t LEFT JOIN users ua ON ua.id = t.assignee_id LEFT JOIN users ur ON ur.id = t.reporter_id LEFT JOIN workflow_templates wt ON wt.id = t.template_id {}", where_sql);
    let mut cq = sqlx::query(&count_sql);
    for b in &binds { cq = bind_val(cq, b); }
    let total: i64 = cq.fetch_one(pool).await.and_then(|r| r.try_get::<i64,_>("cnt")).unwrap_or(0);

    let lim = q.pageSize.max(1).min(500);
    let off = ((q.page.max(1)-1)*lim).max(0);
    let sql = format!("SELECT t.id, t.ticket_no, t.ticket_type, t.title, t.status, t.priority,
            t.category, t.assignee_id, t.reporter_id, t.sla_due_at, t.current_node_key,
            t.template_id, t.created_at, t.updated_at, t.closed_at, t.resolution,
            ua.display_name AS assignee_name, ur.display_name AS reporter_name,
            wt.name AS template_name
        FROM tickets t
        LEFT JOIN users ua ON ua.id = t.assignee_id
        LEFT JOIN users ur ON ur.id = t.reporter_id
        LEFT JOIN workflow_templates wt ON wt.id = t.template_id
        {}
        ORDER BY t.created_at DESC
        LIMIT ? OFFSET ?", where_sql);
    let mut list_q = sqlx::query(&sql);
    for b in &binds { list_q = bind_val(list_q, b); }
    list_q = list_q.bind(lim).bind(off);
    let rows = list_q.fetch_all(pool).await.unwrap_or_default();
    let items: Vec<Value> = rows.iter().map(|r| json!({
        "id": r.try_get::<String,_>("id").unwrap_or_default(),
        "ticketNo": r.try_get::<String,_>("ticket_no").unwrap_or_default(),
        "ticketType": r.try_get::<String,_>("ticket_type").unwrap_or_default(),
        "title": r.try_get::<String,_>("title").unwrap_or_default(),
        "status": r.try_get::<String,_>("status").unwrap_or_default(),
        "priority": r.try_get::<i8,_>("priority").unwrap_or(4),
        "category": r.try_get::<Option<String>,_>("category").unwrap_or(None),
        "assigneeId": r.try_get::<Option<String>,_>("assignee_id").unwrap_or(None),
        "reporterId": r.try_get::<Option<String>,_>("reporter_id").unwrap_or(None),
        "slaDueAt": dt_opt(r, "sla_due_at"),
        "currentNodeKey": r.try_get::<Option<String>,_>("current_node_key").unwrap_or(None),
        "templateId": r.try_get::<Option<String>,_>("template_id").unwrap_or(None),
        "resolution": r.try_get::<Option<String>,_>("resolution").unwrap_or(None),
        "createdAt": dt_str(r, "created_at"),
        "updatedAt": dt_str(r, "updated_at"),
        "closedAt": dt_opt(r, "closed_at"),
        "assigneeName": r.try_get::<Option<String>,_>("assignee_name").unwrap_or(None),
        "reporterName": r.try_get::<Option<String>,_>("reporter_name").unwrap_or(None),
        "templateName": r.try_get::<Option<String>,_>("template_name").unwrap_or(None),
    })).collect();
    Ok(Json(json!({
        "code":0,"message":"ok",
        "data":{"total":total,"page":q.page,"pageSize":q.pageSize,"list":items}
    })))
}

fn push_eq(wc: &mut Vec<String>, binds: &mut Vec<Value>, col: &str, v: &Option<String>) {
    if let Some(s) = v { if !s.is_empty() { wc.push(format!("{} = ?", col)); binds.push(json!(s)); } } }
fn bind_val<'q>(q: sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>, b: &'q Value) -> sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments> {
    match b {
        Value::String(s) => q.bind(s),
        Value::Number(n) => if let Some(i) = n.as_i64() { q.bind(i) } else if let Some(f) = n.as_f64() { q.bind(f) } else { q.bind(n.to_string()) },
        Value::Bool(x) => q.bind(x),
        Value::Null => q.bind(None::<String>),
        _ => q.bind(b.to_string()),
    }
}

pub async fn my_todo_tickets(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Query(q): Query<MyTodoQ>,
) -> Result<Json<Value>, AppError> {
    crate::auth::require_permission(&auth, "ticket:read")?;
    let pool = &state.db;
    let me_id = uid(&auth);
    let lim = q.pageSize.unwrap_or(5).min(20);
    let like_pat = format!("%\"id\":\"{}\"%", me_id);
    let sql = format!(
        "SELECT t.id, t.ticket_no, t.ticket_type, t.title, t.status, t.priority,
         t.category, t.assignee_id, t.reporter_id, t.sla_due_at, t.current_node_key,
         t.template_id, t.created_at, t.updated_at, t.resolution,
         ua.display_name AS assignee_name, ur.display_name AS reporter_name,
         wt.name AS template_name,
         wn.node_key AS active_node_key, wn.node_name AS active_node_name
         FROM tickets t
         LEFT JOIN users ua ON ua.id = t.assignee_id
         LEFT JOIN users ur ON ur.id = t.reporter_id
         LEFT JOIN workflow_templates wt ON wt.id = t.template_id
         JOIN ticket_workflow_nodes wn ON wn.ticket_id = t.id
         WHERE t.deleted_at IS NULL AND wn.status = 'active'
           AND CAST(wn.approvers AS CHAR) LIKE ?
         ORDER BY t.created_at DESC LIMIT ?"
    );
    let rows = sqlx::query(&sql).bind(&like_pat).bind(lim as i64).fetch_all(pool).await.unwrap_or_default();
    let items: Vec<Value> = rows.iter().map(|r| {
        let p = r.try_get::<i8,_>("priority").unwrap_or(3);
        json!({
            "id": r.try_get::<String,_>("id").unwrap_or_default(),
            "ticketNo": r.try_get::<String,_>("ticket_no").unwrap_or_default(),
            "ticketType": r.try_get::<String,_>("ticket_type").unwrap_or_default(),
            "title": r.try_get::<String,_>("title").unwrap_or_default(),
            "status": r.try_get::<String,_>("status").unwrap_or_default(),
            "priority": p,
            "category": r.try_get::<Option<String>,_>("category").unwrap_or(None),
            "assigneeId": r.try_get::<Option<String>,_>("assignee_id").unwrap_or(None),
            "reporterId": r.try_get::<Option<String>,_>("reporter_id").unwrap_or(None),
            "slaDueAt": dt_opt(r, "sla_due_at"),
            "currentNodeKey": r.try_get::<Option<String>,_>("current_node_key").unwrap_or(None),
            "templateId": r.try_get::<Option<String>,_>("template_id").unwrap_or(None),
            "createdAt": dt_str(r, "created_at"),
            "updatedAt": dt_str(r, "updated_at"),
            "assigneeName": r.try_get::<Option<String>,_>("assignee_name").unwrap_or(None),
            "reporterName": r.try_get::<Option<String>,_>("reporter_name").unwrap_or(None),
            "templateName": r.try_get::<Option<String>,_>("template_name").unwrap_or(None),
            "activeNodeKey": r.try_get::<Option<String>,_>("active_node_key").unwrap_or(None),
            "activeNodeName": r.try_get::<Option<String>,_>("active_node_name").unwrap_or(None),
        })
    }).collect();
    Ok(Json(json!({ "code":0, "message":"ok", "data": items })))
}

#[derive(Debug, Deserialize)]
pub struct MyTodoQ {
    pub pageSize: Option<i64>,
}

pub async fn my_done_tickets(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Query(q): Query<MyTodoQ>,
) -> Result<Json<Value>, AppError> {
    crate::auth::require_permission(&auth, "ticket:read")?;
    let pool = &state.db;
    let me_id = uid(&auth);
    let lim = q.pageSize.unwrap_or(5).min(20);
    let sql = format!(
        "SELECT t.id, t.ticket_no, t.ticket_type, t.title, t.status, t.priority,
         t.category, t.assignee_id, t.reporter_id, t.sla_due_at, t.current_node_key,
         t.template_id, t.created_at, t.updated_at, t.resolution,
         ua.display_name AS assignee_name, ur.display_name AS reporter_name,
         wt.name AS template_name,
         wn.node_key AS done_node_key, wn.node_name AS done_node_name,
         wn.decision AS done_decision, wn.done_at AS done_at
         FROM tickets t
         LEFT JOIN users ua ON ua.id = t.assignee_id
         LEFT JOIN users ur ON ur.id = t.reporter_id
         LEFT JOIN workflow_templates wt ON wt.id = t.template_id
         JOIN ticket_workflow_nodes wn ON wn.ticket_id = t.id
         WHERE t.deleted_at IS NULL
           AND wn.decider_id = ?
           AND wn.status = 'done'
         ORDER BY wn.done_at DESC LIMIT ?"
    );
    let rows = sqlx::query(&sql).bind(me_id).bind(lim as i64).fetch_all(pool).await.unwrap_or_default();
    let items: Vec<Value> = rows.iter().map(|r| {
        let p = r.try_get::<i8,_>("priority").unwrap_or(3);
        json!({
            "id": r.try_get::<String,_>("id").unwrap_or_default(),
            "ticketNo": r.try_get::<String,_>("ticket_no").unwrap_or_default(),
            "ticketType": r.try_get::<String,_>("ticket_type").unwrap_or_default(),
            "title": r.try_get::<String,_>("title").unwrap_or_default(),
            "status": r.try_get::<String,_>("status").unwrap_or_default(),
            "priority": p,
            "category": r.try_get::<Option<String>,_>("category").unwrap_or(None),
            "assigneeId": r.try_get::<Option<String>,_>("assignee_id").unwrap_or(None),
            "reporterId": r.try_get::<Option<String>,_>("reporter_id").unwrap_or(None),
            "slaDueAt": dt_opt(r, "sla_due_at"),
            "currentNodeKey": r.try_get::<Option<String>,_>("current_node_key").unwrap_or(None),
            "templateId": r.try_get::<Option<String>,_>("template_id").unwrap_or(None),
            "createdAt": dt_str(r, "created_at"),
            "updatedAt": dt_str(r, "updated_at"),
            "assigneeName": r.try_get::<Option<String>,_>("assignee_name").unwrap_or(None),
            "reporterName": r.try_get::<Option<String>,_>("reporter_name").unwrap_or(None),
            "templateName": r.try_get::<Option<String>,_>("template_name").unwrap_or(None),
            "doneNodeKey": r.try_get::<Option<String>,_>("done_node_key").unwrap_or(None),
            "doneNodeName": r.try_get::<Option<String>,_>("done_node_name").unwrap_or(None),
            "doneDecision": r.try_get::<Option<String>,_>("done_decision").unwrap_or(None),
            "doneAt": dt_opt(r, "done_at"),
        })
    }).collect();
    Ok(Json(json!({ "code":0, "message":"ok", "data": items })))
}

pub async fn ticket_kpis(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Json<Value>, AppError> {
    crate::auth::require_permission(&auth, "ticket:read")?;
    let pool = &state.db;
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tickets WHERE deleted_at IS NULL").fetch_one(pool).await.unwrap_or(0);
    let open:  i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tickets WHERE status IN ('open','assigned','in_progress') AND deleted_at IS NULL").fetch_one(pool).await.unwrap_or(0);
    let review:i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tickets WHERE status='pending_review' AND deleted_at IS NULL").fetch_one(pool).await.unwrap_or(0);
    let closed:i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tickets WHERE status IN ('closed','resolved','cancelled') AND deleted_at IS NULL").fetch_one(pool).await.unwrap_or(0);
    let breach:i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tickets WHERE status NOT IN ('closed','cancelled') AND sla_due_at IS NOT NULL AND sla_due_at < NOW() AND deleted_at IS NULL").fetch_one(pool).await.unwrap_or(0);
    let by_type_vec: Vec<(Option<String>, i64)> = sqlx::query_as("SELECT ticket_type, COUNT(*) FROM tickets WHERE deleted_at IS NULL GROUP BY ticket_type").fetch_all(pool).await.unwrap_or_default();
    let by_type: Value = Value::Object(by_type_vec.into_iter().map(|(k,v)| (k.unwrap_or_else(||"unknown".to_string()), json!(v))).collect());
    let by_priority_vec: Vec<(i8, i64)> = sqlx::query_as("SELECT priority, COUNT(*) FROM tickets WHERE deleted_at IS NULL GROUP BY priority").fetch_all(pool).await.unwrap_or_default();
    let by_priority: Value = Value::Object(by_priority_vec.into_iter().map(|(k,v)| (format!("P{}", 5 - k.max(0) as i64), json!(v))).collect());
    Ok(Json(json!({
        "code":0,"message":"ok","data":{
            "total":total,"open":open,"pendingReview":review,"closed":closed,"slaBreached":breach,
            "byType":by_type,"byPriority":by_priority
        }
    })))
}

// ==================== 工单统计看板 ====================

pub async fn ticket_stats_overview(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Json<Value>, AppError> {
    crate::auth::require_permission(&auth, "ticket:read")?;
    let pool = &state.db;

    // KPI 基础数据
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tickets WHERE deleted_at IS NULL"
    ).fetch_one(pool).await.unwrap_or(0);

    let pending: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tickets WHERE status IN ('open','assigned','in_progress','pending_review') AND deleted_at IS NULL"
    ).fetch_one(pool).await.unwrap_or(0);

    let closed: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tickets WHERE status IN ('closed','resolved','cancelled') AND deleted_at IS NULL"
    ).fetch_one(pool).await.unwrap_or(0);

    // 本月新增
    let new_this_month: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tickets WHERE deleted_at IS NULL \
         AND created_at >= DATE_FORMAT(CURDATE(), '%Y-%m-01 00:00:00')"
    ).fetch_one(pool).await.unwrap_or(0);

    // 平均处理时长（小时）
    let avg_resolve_hours: f64 = sqlx::query_scalar::<_, Option<f64>>(
        "SELECT AVG(TIMESTAMPDIFF(SECOND, created_at, closed_at)) / 3600.0 \
         FROM tickets WHERE deleted_at IS NULL \
         AND status IN ('closed','resolved') AND closed_at IS NOT NULL"
    ).fetch_one(pool).await.unwrap_or(None).unwrap_or(0.0);

    // 按类型分布
    let by_type_rows: Vec<(Option<String>, i64)> = sqlx::query_as(
        "SELECT ticket_type, COUNT(*) AS cnt FROM tickets WHERE deleted_at IS NULL GROUP BY ticket_type ORDER BY cnt DESC"
    ).fetch_all(pool).await.unwrap_or_default();
    let by_type: Vec<Value> = by_type_rows.into_iter()
        .map(|(t, c)| json!({ "name": t.unwrap_or_else(||"unknown".to_string()), "value": c }))
        .collect();

    // 按状态分布
    let by_status_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT status, COUNT(*) AS cnt FROM tickets WHERE deleted_at IS NULL GROUP BY status ORDER BY cnt DESC"
    ).fetch_all(pool).await.unwrap_or_default();
    let by_status: Vec<Value> = by_status_rows.into_iter()
        .map(|(s, c)| json!({ "name": s, "value": c }))
        .collect();

    // 近 30 天趋势（按天分组）
    let trend_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT DATE_FORMAT(created_at, '%Y-%m-%d') AS d, COUNT(*) AS cnt \
         FROM tickets WHERE deleted_at IS NULL \
         AND created_at >= DATE_SUB(CURDATE(), INTERVAL 29 DAY) \
         GROUP BY DATE_FORMAT(created_at, '%Y-%m-%d') \
         ORDER BY d"
    ).fetch_all(pool).await.unwrap_or_default();
    let trend_30d: Vec<Value> = trend_rows.into_iter()
        .map(|(d, c)| json!({ "date": d, "count": c }))
        .collect();

    Ok(Json(json!({
        "code": 0,
        "message": "ok",
        "data": {
            "total": total,
            "pending": pending,
            "closed": closed,
            "newThisMonth": new_this_month,
            "avgResolveHours": avg_resolve_hours,
            "byType": by_type,
            "byStatus": by_status,
            "trend30d": trend_30d,
        }
    })))
}

// ----------------- 创建 -----------------

async fn load_template_and_compile(pool: &MySqlPool,
    template_id: Option<&str>, ticket_type: &str) -> Result<(String, Vec<CompiledNode>, Value), AppError>
{
    let row: Option<(String, Value, i32)> = sqlx::query_as(
        "SELECT id, definition, version FROM workflow_templates WHERE id = ? AND enabled=1 LIMIT 1"
    ).bind(template_id).fetch_optional(pool).await?;
    let (tmpl_id, def_val, _ver) = if let Some(r) = row {
        r
    } else {
        let def: Option<(String, Value, i32)> = sqlx::query_as(
            "SELECT id, definition, version FROM workflow_templates
             WHERE enabled=1 AND scope='builtin' AND ticket_type=? ORDER BY created_at DESC LIMIT 1"
        ).bind(ticket_type).fetch_optional(pool).await?;
        def.ok_or_else(|| AppError::bad(&format!("找不到匹配的工作流模板: {}", ticket_type)))?
    };
    let lf: LfDefinition = serde_json::from_value(def_val.clone())
        .map_err(|e| AppError::bad(&format!("definition 解析失败: {}", e)))?;
    let (nodes, errs) = compile_definition(&lf);
    if !errs.is_empty() {
        return Err(AppError::bad(&format!("definition 编译失败: {:?}", errs)));
    }
    Ok((tmpl_id, nodes, def_val))
}

pub async fn create_ticket(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(req): Json<CreateTicketReq>,
) -> Result<Json<Value>, AppError> {
    crate::auth::require_permission(&auth, "ticket:create")?;
    let pool = &state.db;
    if req.title.trim().is_empty() { return Err(AppError::bad("title 不能为空")); }
    if !(1..=4).contains(&req.priority) { return Err(AppError::bad("priority 必须 1..=4")); }
    let (tmpl_id, compiled_nodes, _def) =
        load_template_and_compile(pool, req.templateId.as_deref(), &req.ticketType).await?;
    let start_out = compiled_nodes.iter().find(|n| n.key == "__start__")
        .ok_or_else(|| AppError::bad("缺少__start__节点"))?;
    let ctx0 = json!({
        "ticketType": req.ticketType,
        "priority": req.priority,
        "category": req.category,
        "reporterId": uid(&auth),
        "assigneeId": req.assigneeId,
        "extra": req.extra,
    });
    // 找到起始节点后，自动跳过 auto_pass 节点，定位到第一个需要人工处理的节点
    let mut next_key = pick_next_node(&start_out.outs, &ctx0).ok_or_else(|| AppError::bad("start 无出边"))?;
    let mut auto_skipped_keys: Vec<String> = vec![];
    loop {
        let cur_node = compiled_nodes.iter().find(|n| n.key == next_key);
        if let Some(n) = cur_node {
            if n.kind == "auto_pass" {
                auto_skipped_keys.push(n.key.clone());
                if let Some(nk) = pick_next_node(&n.outs, &ctx0) {
                    next_key = nk;
                } else {
                    break;
                }
            } else {
                break;
            }
        } else {
            break;
        }
    }

    let ticket_no = generate_ticket_no(pool).await?;
    let created_rfc = chrono::Utc::now().to_rfc3339();
    let sla_due = compute_sla_due(req.priority, &created_rfc);
    let ticket_id = Uuid::new_v4().to_string();
    let status = derive_status(Some(&next_key), &compiled_nodes);
    let reporter_id = uid(&auth).to_string();
    let relation_def = req.extra.get("relation").and_then(|v|v.as_str()).unwrap_or("caused_by");

    let mut tx = pool.begin().await?;
    sqlx::query(
        "INSERT INTO tickets (id, ticket_no, ticket_type, title, description, priority, category,
            status, assignee_id, reporter_id, sla_due_at, current_node_key, template_id, resolution,
            created_at, updated_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,NOW(),NOW())"
    ).bind(&ticket_id).bind(&ticket_no).bind(&req.ticketType).bind(req.title.trim())
     .bind(req.description.as_deref()).bind(req.priority)
     .bind(req.category.as_deref()).bind(status)
     .bind(req.assigneeId.as_deref()).bind(&reporter_id).bind(sla_due.as_deref())
     .bind(&next_key).bind(&tmpl_id).bind(None::<String>)
     .execute(&mut *tx).await?;

    for n in &compiled_nodes {
        let is_active = n.key == next_key;
        let is_auto_skipped = auto_skipped_keys.contains(&n.key);
        let now = chrono::Utc::now().naive_utc();
        let reached_at = if n.key == "__start__" || is_active || is_auto_skipped { Some(now) } else { None };
        let done_at = if n.key == "__start__" || is_auto_skipped { Some(now) } else { None };
        let approvers = if !matches!(n.kind.as_str(), "auto_pass"|"start"|"end"|"condition_gateway"|"parallel_split"|"parallel_join") {
            let r = resolve_approvers(pool, &n.approver_selector, req.assigneeId.as_deref(), &reporter_id).await;
            Value::Array(r.iter().map(|(id,name)| json!({"id":id,"name":name})).collect())
        } else { Value::Array(vec![]) };
        let node_id = Uuid::new_v4().to_string();
        let outs_json = serde_json::to_string(&n.outs).unwrap_or_else(|_| "[]".into());
        let node_status = if n.key == "__start__" || is_auto_skipped { "done" } else if is_active { "active" } else { "pending" };
        sqlx::query(
            "INSERT INTO ticket_workflow_nodes (id, ticket_id, node_key, node_name, node_type,
                approvers, node_index, status, entered_at, done_at, outs, timeout_hours,
                timeout_action, reject_back_to) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)"
        ).bind(&node_id).bind(&ticket_id).bind(&n.key).bind(&n.name).bind(&n.kind)
         .bind(&approvers).bind(n.index)
         .bind(node_status)
         .bind(reached_at).bind(done_at).bind(&outs_json)
         .bind(n.timeout_hours).bind(n.timeout_action.as_deref()).bind(n.reject_back_to.as_deref())
         .execute(&mut *tx).await?;
    }
    for aid in &req.alertIds {
        let link_id = Uuid::new_v4().to_string();
        sqlx::query("INSERT IGNORE INTO ticket_alert_links (id, ticket_id, alert_id, relation, created_at) VALUES (?,?,?,?,NOW())")
            .bind(&link_id).bind(&ticket_id).bind(aid).bind(relation_def)
            .execute(&mut *tx).await?;
    }
    let cid = Uuid::new_v4().to_string();
    let skip_msg = if auto_skipped_keys.is_empty() {
        String::new()
    } else {
        let skip_names: Vec<String> = auto_skipped_keys.iter()
            .filter_map(|k| compiled_nodes.iter().find(|n| n.key == *k).map(|n| n.name.clone()))
            .collect();
        format!("（自动跳过：{}）", skip_names.join(" → "))
    };
    let first_cmt = format!("工单创建，进入节点「{}」{}",
        compiled_nodes.iter().find(|n| n.key == next_key).map(|n|n.name.as_str()).unwrap_or(""),
        skip_msg
    );
    sqlx::query("INSERT INTO ticket_comments (id, ticket_id, user_id, action, node_key, content, created_at) VALUES (?,?,?,?,?,?,NOW())")
        .bind(&cid).bind(&ticket_id).bind(&reporter_id).bind("create")
        .bind(&next_key).bind(&first_cmt).execute(&mut *tx).await?;
    tx.commit().await?;
    // 通知第一个激活节点的审批人
    let active_approvers: Vec<(String, String)> = sqlx::query(
        "SELECT approvers FROM ticket_workflow_nodes WHERE ticket_id=? AND node_key=? AND status='active'"
    ).bind(&ticket_id).bind(&next_key).fetch_optional(pool).await
        .ok().flatten()
        .and_then(|r| r.try_get::<Option<Value>, _>("approvers").ok().flatten())
        .and_then(|v| v.as_array().map(|a| a.to_vec()))
        .map(|arr| arr.iter().filter_map(|a| {
            let id = a.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let name = a.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if id.is_empty() { None } else { Some((id, name)) }
        }).collect())
        .unwrap_or_default();
    let active_node_name = compiled_nodes.iter().find(|n| n.key == next_key).map(|n| n.name.as_str()).unwrap_or("");
    for (aid, _aname) in &active_approvers {
        let title = format!("新工单待审批：{}", req.title.trim());
        let content = format!("工单 {} 需要您在节点「{}」进行审批。", ticket_no, active_node_name);
        let link = format!("/tickets/{}", ticket_id);
        crate::notification_routes::create_notification(pool, aid, "ticket_assigned", &title, &content, &link).await;
    }
    Ok(Json(json!({"code":0,"message":"ok","data":{"id":ticket_id,"ticketNo":ticket_no,"status":status,"currentNodeKey":next_key}})))
}

// ----------------- 详情 -----------------

pub async fn get_ticket(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    crate::auth::require_permission(&auth, "ticket:read")?;
    let pool = &state.db;
    let row = sqlx::query(
        "SELECT t.id, t.ticket_no, t.ticket_type, t.title, t.description, t.priority, t.category,
            t.status, t.assignee_id, t.reporter_id, t.sla_due_at, t.current_node_key,
            t.template_id, t.resolution, t.created_at, t.updated_at, t.closed_at,
            ua.display_name AS assignee_name, ur.display_name AS reporter_name,
            wt.name AS template_name, wt.definition AS template_definition
         FROM tickets t
         LEFT JOIN users ua ON ua.id = t.assignee_id
         LEFT JOIN users ur ON ur.id = t.reporter_id
         LEFT JOIN workflow_templates wt ON wt.id = t.template_id
         WHERE t.id = ? AND t.deleted_at IS NULL LIMIT 1"
    ).bind(&id).fetch_optional(pool).await?;
    let t_row = row.ok_or(AppError::not_found("工单不存在"))?;
    let t = json!({
        "id": t_row.try_get::<String,_>("id").unwrap_or_default(),
        "ticketNo": t_row.try_get::<String,_>("ticket_no").unwrap_or_default(),
        "ticketType": t_row.try_get::<String,_>("ticket_type").unwrap_or_default(),
        "title": t_row.try_get::<String,_>("title").unwrap_or_default(),
        "description": t_row.try_get::<Option<String>,_>("description").unwrap_or(None),
        "priority": t_row.try_get::<i8,_>("priority").unwrap_or(4),
        "category": t_row.try_get::<Option<String>,_>("category").unwrap_or(None),
        "status": t_row.try_get::<String,_>("status").unwrap_or_default(),
        "assigneeId": t_row.try_get::<Option<String>,_>("assignee_id").unwrap_or(None),
        "reporterId": t_row.try_get::<Option<String>,_>("reporter_id").unwrap_or(None),
        "slaDueAt": dt_opt(&t_row, "sla_due_at"),
        "currentNodeKey": t_row.try_get::<Option<String>,_>("current_node_key").unwrap_or(None),
        "templateId": t_row.try_get::<Option<String>,_>("template_id").unwrap_or(None),
        "resolution": t_row.try_get::<Option<String>,_>("resolution").unwrap_or(None),
        "createdAt": dt_str(&t_row, "created_at"),
        "updatedAt": dt_str(&t_row, "updated_at"),
        "closedAt": dt_opt(&t_row, "closed_at"),
        "assigneeName": t_row.try_get::<Option<String>,_>("assignee_name").unwrap_or(None),
        "reporterName": t_row.try_get::<Option<String>,_>("reporter_name").unwrap_or(None),
        "templateName": t_row.try_get::<Option<String>,_>("template_name").unwrap_or(None),
        "templateDefinition": t_row.try_get::<Option<Value>,_>("template_definition").unwrap_or(None),
    });
    let p = t.get("priority").and_then(|v|v.as_i64()).unwrap_or(3) as i8;
    let (mtta_h, mttr_h) = crate::workflow_engine::sla_hours(p);

    // 查询当前用户是否关注了此工单
    let me_id = uid(&auth);
    let watched_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ticket_watchers WHERE ticket_id = ? AND watcher_id = ?"
    )
    .bind(&id)
    .bind(me_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);
    let is_watched = watched_count > 0;

    let wf_rows = sqlx::query(
        "SELECT id, node_key, node_name, node_type, approvers, status, entered_at, done_at,
            decision, decider_id, timeout_hours, timeout_action, reject_back_to, outs, extra,
            updated_at FROM ticket_workflow_nodes WHERE ticket_id=? ORDER BY node_index ASC"
    ).bind(&id).fetch_all(pool).await.unwrap_or_default();
    let nodes: Vec<Value> = wf_rows.iter().map(|r| json!({
        "id": r.try_get::<String,_>("id").unwrap_or_default(),
        "nodeKey": r.try_get::<String,_>("node_key").unwrap_or_default(),
        "nodeName": r.try_get::<String,_>("node_name").unwrap_or_default(),
        "nodeType": r.try_get::<String,_>("node_type").unwrap_or_default(),
        "approvers": r.try_get::<Option<Value>,_>("approvers").unwrap_or(Some(Value::Array(vec![]))).unwrap_or(Value::Array(vec![])),
        "status": r.try_get::<String,_>("status").unwrap_or_default(),
        "enteredAt": dt_opt(r, "entered_at"),
        "doneAt": dt_opt(r, "done_at"),
        "decision": r.try_get::<Option<String>,_>("decision").unwrap_or(None),
        "deciderId": r.try_get::<Option<String>,_>("decider_id").unwrap_or(None),
        "timeoutHours": r.try_get::<Option<i64>,_>("timeout_hours").unwrap_or(None),
        "timeoutAction": r.try_get::<Option<String>,_>("timeout_action").unwrap_or(None),
        "rejectBackTo": r.try_get::<Option<String>,_>("reject_back_to").unwrap_or(None),
        "outs": r.try_get::<Option<Value>,_>("outs").unwrap_or(Some(Value::Array(vec![]))).unwrap_or(Value::Array(vec![])),
        "extra": r.try_get::<Option<Value>,_>("extra").unwrap_or(None),
        "updatedAt": dt_opt(r, "updated_at"),
    })).collect();

    let c_rows = sqlx::query(
        "SELECT c.id, c.action, c.node_key, c.content, c.extra, c.created_at,
            c.user_id, u.display_name AS user_name
         FROM ticket_comments c LEFT JOIN users u ON u.id = c.user_id
         WHERE c.ticket_id=? ORDER BY c.created_at ASC"
    ).bind(&id).fetch_all(pool).await.unwrap_or_default();
    let comments: Vec<Value> = c_rows.iter().map(|r| json!({
        "id": r.try_get::<String,_>("id").unwrap_or_default(),
        "action": r.try_get::<String,_>("action").unwrap_or_default(),
        "nodeKey": r.try_get::<Option<String>,_>("node_key").unwrap_or(None),
        "content": r.try_get::<Option<String>,_>("content").unwrap_or(None),
        "extra": r.try_get::<Option<Value>,_>("extra").unwrap_or(None),
        "createdAt": dt_str(r, "created_at"),
        "userId": r.try_get::<Option<String>,_>("user_id").unwrap_or(None),
        "userName": r.try_get::<Option<String>,_>("user_name").unwrap_or(None),
    })).collect();

    let l_rows = sqlx::query(
        "SELECT l.alert_id, l.relation, l.created_at, e.title AS alert_title, e.severity AS alert_severity
         FROM ticket_alert_links l LEFT JOIN alert_events e ON e.id = l.alert_id
         WHERE l.ticket_id=? ORDER BY l.created_at DESC"
    ).bind(&id).fetch_all(pool).await.unwrap_or_default();
    let links: Vec<Value> = l_rows.iter().map(|r| json!({
        "alertId": r.try_get::<String,_>("alert_id").unwrap_or_default(),
        "relation": r.try_get::<Option<String>,_>("relation").unwrap_or(None),
        "createdAt": dt_str(r, "created_at"),
        "alertTitle": r.try_get::<Option<String>,_>("alert_title").unwrap_or(None),
        "alertSeverity": r.try_get::<Option<String>,_>("alert_severity").unwrap_or(None),
    })).collect();

    // 查询附件列表
    let att_rows = sqlx::query(
        "SELECT a.id, a.filename, a.file_size, a.file_type, a.uploader_id, a.created_at,
                u.display_name AS uploader_name
         FROM ticket_attachments a
         LEFT JOIN users u ON u.id = a.uploader_id
         WHERE a.ticket_id=? AND a.deleted_at IS NULL
         ORDER BY a.created_at ASC"
    ).bind(&id).fetch_all(pool).await.unwrap_or_default();
    let attachments: Vec<Value> = att_rows.iter().map(|r| json!({
        "id": r.try_get::<String,_>("id").unwrap_or_default(),
        "filename": r.try_get::<String,_>("filename").unwrap_or_default(),
        "fileSize": r.try_get::<i64,_>("file_size").unwrap_or(0),
        "fileType": r.try_get::<String,_>("file_type").unwrap_or_default(),
        "uploaderId": r.try_get::<Option<String>,_>("uploader_id").unwrap_or(None),
        "uploaderName": r.try_get::<Option<String>,_>("uploader_name").unwrap_or(None),
        "createdAt": dt_str(r, "created_at"),
    })).collect();

    // 构建带 isWatched 的 ticket 对象
    let mut ticket_obj = t.as_object().cloned().unwrap_or_default();
    ticket_obj.insert("isWatched".to_string(), json!(is_watched));

    Ok(Json(json!({
        "code":0,"message":"ok",
        "data":{
            "ticket": Value::Object(ticket_obj),
            "workflowNodes": nodes,
            "comments": comments,
            "alertLinks": links,
            "attachments": attachments,
            "sla": {"mttaHours":mtta_h,"mttrHours":mttr_h}
        }
    })))
}

// ----------------- 编辑 / 删除 -----------------

pub async fn update_ticket(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateTicketReq>,
) -> Result<Json<Value>, AppError> {
    crate::auth::require_permission(&auth, "ticket:update")?;
    let pool = &state.db;
    let old: Option<(Option<String>, Option<i8>, Option<String>, Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT title, priority, category, status, assignee_id, description FROM tickets WHERE id=? AND deleted_at IS NULL LIMIT 1"
    ).bind(&id).fetch_optional(pool).await?;
    let (o_title, o_pri, o_cat, o_st, o_asn, o_desc) = old.ok_or(AppError::not_found("工单不存在"))?;
    let title = req.title.clone().or(o_title);
    let priority = req.priority.or(o_pri);
    let category = req.category.or(o_cat);
    let status = req.status.or(o_st);
    let assignee = req.assigneeId.or(o_asn);
    let desc = req.description.or(o_desc);
    let sla_due_dt = if priority != o_pri {
        let created: Option<(Option<String>,)> = sqlx::query_as(
            "SELECT DATE_FORMAT(created_at, '%Y-%m-%dT%H:%i:%sZ') FROM tickets WHERE id=?"
        ).bind(&id).fetch_one(pool).await.ok();
        match created {
            Some((Some(dt),)) => compute_sla_due(priority.unwrap_or(3), &dt).and_then(|d| parse_dt(&Some(d))),
            _ => None,
        }
    } else { None };
    if let Some(d) = sla_due_dt {
        sqlx::query("UPDATE tickets SET sla_due_at=? WHERE id=?").bind(d).bind(&id).execute(pool).await.ok();
    }
    let now = chrono::Utc::now().naive_utc();
    let closed_at = if let Some(s) = status.as_deref() {
        if matches!(s, "closed"|"resolved"|"cancelled") { Some(now) } else { None }
    } else { None };
    sqlx::query(
        "UPDATE tickets SET title=?, description=?, priority=?, category=?, status=?, assignee_id=?,
            resolution=COALESCE(?, resolution), closed_at=COALESCE(?, closed_at), updated_at=NOW()
         WHERE id=?"
    ).bind(title.as_deref()).bind(desc.as_deref()).bind(priority).bind(category.as_deref())
     .bind(status.as_deref()).bind(assignee.as_deref()).bind(req.resolution.as_deref()).bind(closed_at).bind(&id)
     .execute(pool).await?;
    Ok(Json(json!({"code":0,"message":"ok"})))
}

pub async fn delete_ticket(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    crate::auth::require_permission(&auth, "ticket:delete")?;
    sqlx::query("UPDATE tickets SET deleted_at=NOW(), status='cancelled' WHERE id=? AND deleted_at IS NULL")
        .bind(&id).execute(&state.db).await?;
    Ok(Json(json!({"code":0,"message":"ok"})))
}

// ----------------- 指派 / 关联告警 -----------------

pub async fn assign_ticket(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<AssignReq>,
) -> Result<Json<Value>, AppError> {
    crate::auth::require_permission(&auth, "ticket:update")?;
    let pool = &state.db;
    let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM users WHERE id=? AND enabled=1 LIMIT 1")
        .bind(&req.assigneeId).fetch_optional(pool).await?;
    if exists.is_none() { return Err(AppError::bad("受理人不存在")); }
    sqlx::query("UPDATE tickets SET assignee_id=?, updated_at=NOW() WHERE id=? AND deleted_at IS NULL")
        .bind(&req.assigneeId).bind(&id).execute(pool).await?;
    let cid = Uuid::new_v4().to_string();
    let nk: Option<(Option<String>,)> = sqlx::query_as("SELECT current_node_key FROM tickets WHERE id=?")
        .bind(&id).fetch_one(pool).await.ok();
    let nk = nk.and_then(|(x,)| x).unwrap_or_default();
    let cmt = format!("指派给受理人 {}{}", req.assigneeId, req.reason.map(|r| format!("（{}）", r)).unwrap_or_default());
    sqlx::query("INSERT INTO ticket_comments (id, ticket_id, user_id, action, node_key, content, created_at) VALUES (?,?,?,?,?,?,NOW())")
        .bind(&cid).bind(&id).bind(uid(&auth)).bind("assign").bind(&nk).bind(&cmt)
        .execute(pool).await?;
    Ok(Json(json!({"code":0,"message":"ok"})))
}

pub async fn link_alert(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<LinkAlertReq>,
) -> Result<Json<Value>, AppError> {
    crate::auth::require_permission(&auth, "ticket:update")?;
    let pool = &state.db;
    let lid = Uuid::new_v4().to_string();
    sqlx::query("INSERT IGNORE INTO ticket_alert_links (id, ticket_id, alert_id, relation, created_at) VALUES (?,?,?,?,NOW())")
        .bind(&lid).bind(&id).bind(&req.alertId).bind(req.relation.as_deref().unwrap_or("caused_by"))
        .execute(pool).await?;
    Ok(Json(json!({"code":0,"message":"ok"})))
}

pub async fn unlink_alert(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path((id, alert_id)): Path<(String, String)>,
) -> Result<Json<Value>, AppError> {
    crate::auth::require_permission(&auth, "ticket:update")?;
    sqlx::query("DELETE FROM ticket_alert_links WHERE ticket_id=? AND alert_id=?")
        .bind(&id).bind(&alert_id).execute(&state.db).await?;
    Ok(Json(json!({"code":0,"message":"ok"})))
}

// ----------------- 流程动作 -----------------

async fn get_current(pool: &MySqlPool, ticket_id: &str, node_id: &str) -> Result<(Value, Option<Vec<CompiledNode>>), AppError> {
    let row = sqlx::query(
        "SELECT wn.*, t.current_node_key AS ticket_current_node_key, t.template_id, t.assignee_id,
            t.reporter_id, t.ticket_type, t.priority, t.category, t.status
         FROM ticket_workflow_nodes wn
         INNER JOIN tickets t ON t.id = wn.ticket_id
         WHERE t.id=? AND t.deleted_at IS NULL
           AND (wn.node_key = ? OR wn.id = ?)
         LIMIT 1"
    ).bind(ticket_id).bind(node_id).bind(node_id).fetch_optional(pool).await?;
    let row = row.ok_or(AppError::not_found("节点或工单不存在"))?;
    let cur = json!({
        "id": row.try_get::<String,_>("id").ok(),
        "ticketId": row.try_get::<String,_>("ticket_id").ok(),
        "nodeKey": row.try_get::<String,_>("node_key").ok(),
        "nodeType": row.try_get::<String,_>("node_type").ok(),
        "approvers": row.try_get::<Option<Value>,_>("approvers").ok().flatten(),
        "rejectBackTo": row.try_get::<Option<String>,_>("reject_back_to").ok().flatten(),
        "ticket_current_node_key": row.try_get::<Option<String>,_>("ticket_current_node_key").ok().flatten(),
        "template_id": row.try_get::<Option<String>,_>("template_id").ok().flatten(),
        "assignee_id": row.try_get::<Option<String>,_>("assignee_id").ok().flatten(),
        "reporter_id": row.try_get::<Option<String>,_>("reporter_id").ok().flatten(),
        "ticket_type": row.try_get::<Option<String>,_>("ticket_type").ok().flatten(),
        "priority": row.try_get::<i8,_>("priority").ok(),
        "category": row.try_get::<Option<String>,_>("category").ok().flatten(),
        "status": row.try_get::<String,_>("status").ok(),
    });
    let tmpl_id = cur.get("template_id").and_then(|v|v.as_str()).unwrap_or("");
    let def: Option<Value> = sqlx::query_scalar("SELECT definition FROM workflow_templates WHERE id=? LIMIT 1")
        .bind(tmpl_id).fetch_optional(pool).await.unwrap_or(None);
    let compiled = def.and_then(|d| {
        let lf: Result<LfDefinition,_> = serde_json::from_value(d);
        lf.ok().and_then(|lf| {
            let (n, e) = compile_definition(&lf);
            if e.is_empty() { Some(n) } else { None }
        })
    });
    Ok((cur, compiled))
}

#[allow(clippy::too_many_arguments)]
pub async fn action_on_node(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path((id, node_id)): Path<(String, String)>,
    Json(req): Json<ActionReq>,
) -> Result<Json<Value>, AppError> {
    crate::auth::require_permission(&auth, "ticket:update")?;
    let pool = &state.db;
    let (cur_row, compiled_opt) = get_current(pool, &id, &node_id).await?;
    let n_key = cur_row.get("nodeKey").and_then(|v|v.as_str()).unwrap_or("").to_string();
    let cur_nk_from_ticket = cur_row.get("ticket_current_node_key").and_then(|v|v.as_str()).unwrap_or("");
    if cur_nk_from_ticket != n_key {
        return Err(AppError::bad("当前节点未激活"));
    }
    let me_id = uid(&auth);
    let is_admin = auth.0.role == crate::auth::Role::Admin;
    if !is_admin {
        if let Some(approvers) = cur_row.get("approvers").and_then(|v| v.as_array()) {
            let in_approvers = approvers.iter().any(|a| {
                    a.get("id").and_then(|v| v.as_str()) == Some(me_id)
                });
            if !in_approvers {
                return Err(AppError::forbidden("您不是该节点的审批人"));
            }
        }
    }
    let assignee_id = cur_row.get("assignee_id").and_then(|v|v.as_str()).map(|s|s.to_string());
    let reporter_id = cur_row.get("reporter_id").and_then(|v|v.as_str()).unwrap_or("").to_string();

    match req.decision.as_str() {
        "comment" => {
            let cid = Uuid::new_v4().to_string();
            let cmt = req.comment.clone().unwrap_or_default();
            sqlx::query("INSERT INTO ticket_comments (id, ticket_id, user_id, action, node_key, content, created_at) VALUES (?,?,?,?,?,?,NOW())")
                .bind(&cid).bind(&id).bind(uid(&auth)).bind("comment").bind(&n_key).bind(&cmt)
                .execute(pool).await?;
            return Ok(Json(json!({"code":0,"message":"ok"})));
        }
        "reassign" => {
            let target = req.userId.clone().ok_or_else(|| AppError::bad("缺少 userId"))?;
            if let Some(compiled) = &compiled_opt {
                if compiled.iter().find(|n| n.key == n_key).is_some() {
                    let existing = if let Some(val) = cur_row.get("approvers") {
                        serde_json::from_value::<Vec<Value>>(val.clone()).unwrap_or_default()
                    } else { Vec::new() };
                    let mut next = existing;
                    if !next.iter().any(|v| v.get("id").and_then(|x|x.as_str()) == Some(target.as_str())) {
                        next.push(json!({"id": target, "name": target, "reassigned": true}));
                    }
                    sqlx::query("UPDATE ticket_workflow_nodes SET approvers=?, updated_at=NOW() WHERE ticket_id=? AND node_key=?")
                        .bind(&serde_json::to_string(&next).unwrap_or_else(|_|"[]".into()))
                        .bind(&id).bind(&n_key).execute(pool).await?;
                }
            }
            let cid = Uuid::new_v4().to_string();
            let cmt = format!("转派节点审批人 -> {}{}", target, req.reason.clone().map(|r| format!("（{}）", r)).unwrap_or_default());
            sqlx::query("INSERT INTO ticket_comments (id, ticket_id, user_id, action, node_key, content, created_at) VALUES (?,?,?,?,?,?,NOW())")
                .bind(&cid).bind(&id).bind(uid(&auth)).bind("reassign").bind(&n_key).bind(&cmt)
                .execute(pool).await?;
            return Ok(Json(json!({"code":0,"message":"ok"})));
        }
        "reject" => {
            let to_key = req.toNodeKey.clone().or_else(|| cur_row.get("rejectBackTo").and_then(|v|v.as_str()).map(String::from))
                .unwrap_or_else(||"dispatch".to_string());
            sqlx::query("UPDATE ticket_workflow_nodes SET status='rejected', decision='reject', decider_id=?, done_at=NOW(), updated_at=NOW() WHERE ticket_id=? AND node_key=?")
                .bind(uid(&auth)).bind(&id).bind(&n_key).execute(pool).await?;
            sqlx::query("UPDATE ticket_workflow_nodes SET status='pending', done_at=NULL, decision=NULL, decider_id=NULL WHERE ticket_id=? AND node_key=?")
                .bind(&id).bind(&to_key).execute(pool).await?;
            sqlx::query("UPDATE tickets SET current_node_key=?, updated_at=NOW() WHERE id=?")
                .bind(&to_key).bind(&id).execute(pool).await?;
            let cid = Uuid::new_v4().to_string();
            let cmt = format!("驳回节点「{}」 -> 回跳至节点「{}」{}", n_key, to_key, req.reason.clone().map(|r| format!("（{}）", r)).unwrap_or_default());
            sqlx::query("INSERT INTO ticket_comments (id, ticket_id, user_id, action, node_key, content, created_at) VALUES (?,?,?,?,?,?,NOW())")
                .bind(&cid).bind(&id).bind(uid(&auth)).bind("reject").bind(&n_key).bind(&cmt)
                .execute(pool).await?;
            let st = compiled_opt.as_ref().map(|c| derive_status(Some(&to_key), c)).unwrap_or("in_progress");
            sqlx::query("UPDATE tickets SET status=?, updated_at=NOW() WHERE id=?").bind(st).bind(&id).execute(pool).await.ok();
            // 通知回跳节点审批人工单被驳回
            let reject_approvers: Vec<String> = sqlx::query(
                "SELECT approvers FROM ticket_workflow_nodes WHERE ticket_id=? AND node_key=?"
            ).bind(&id).bind(&to_key).fetch_optional(pool).await
                .ok().flatten()
                .and_then(|r| r.try_get::<Option<Value>, _>("approvers").ok().flatten())
                .and_then(|v| v.as_array().map(|a| a.to_vec()))
                .map(|arr| arr.iter().filter_map(|a| a.get("id").and_then(|v| v.as_str()).map(String::from)).collect())
                .unwrap_or_default();
            let ticket_no_v = cur_row.get("ticket_no").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let title_v = cur_row.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
            for aid in &reject_approvers {
                let n_title = format!("工单已驳回：{}", title_v);
                let n_content = format!("工单 {} 在节点「{}」被驳回，已回退到节点「{}」处理。", ticket_no_v, n_key, to_key);
                let n_link = format!("/tickets/{}", id);
                crate::notification_routes::create_notification(pool, aid, "ticket_rejected", &n_title, &n_content, &n_link).await;
            }
            return Ok(Json(json!({"code":0,"message":"ok","currentNodeKey":to_key})));
        }
        "approve" => {
            sqlx::query("UPDATE ticket_workflow_nodes SET status='done', decision='approve', decider_id=?, done_at=NOW(), updated_at=NOW() WHERE ticket_id=? AND node_key=?")
                .bind(uid(&auth)).bind(&id).bind(&n_key).execute(pool).await?;
            let first_next = if let Some(compiled) = &compiled_opt {
                if let Some(cur_node) = compiled.iter().find(|n| n.key == n_key) {
                    let ctx = json!({
                        "ticketType": cur_row.get("ticket_type"),
                        "priority": cur_row.get("priority"),
                        "category": cur_row.get("category"),
                        "assigneeId": assignee_id,
                        "reporterId": reporter_id,
                    });
                    pick_next_node(&cur_node.outs, &ctx)
                } else { None }
            } else { None };

            let (final_next, derived_st) = match first_next {
                Some(k) if k == "__end__" => {
                    sqlx::query("UPDATE ticket_workflow_nodes SET status='done', done_at=NOW() WHERE ticket_id=? AND node_key='__end__'")
                        .bind(&id).execute(pool).await.ok();
                    (None, "closed")
                }
                Some(k) => {
                    let mut actual_next = Some(k.clone());
                    let mut current_k = k;
                    if let Some(compiled) = &compiled_opt {
                        for _ in 0..20 {
                            if let Some(nd) = compiled.iter().find(|n| n.key == current_k) {
                                sqlx::query("UPDATE ticket_workflow_nodes SET status='active', entered_at=NOW(), updated_at=NOW() WHERE ticket_id=? AND node_key=?")
                                    .bind(&id).bind(&current_k).execute(pool).await.ok();
                                if matches!(nd.kind.as_str(), "auto_pass"|"condition_gateway"|"parallel_split"|"parallel_join") {
                                    sqlx::query("UPDATE ticket_workflow_nodes SET status='done', done_at=NOW() WHERE ticket_id=? AND node_key=?")
                                        .bind(&id).bind(&current_k).execute(pool).await.ok();
                                    let ctx = json!({
                                        "ticketType": cur_row.get("ticket_type"),
                                        "priority": cur_row.get("priority"),
                                        "category": cur_row.get("category"),
                                        "assigneeId": assignee_id.as_deref(),
                                        "reporterId": reporter_id,
                                    });
                                    let nx = pick_next_node(&nd.outs, &ctx);
                                    match nx {
                                        Some(nk) if nk == "__end__" => { actual_next = None; break; }
                                        Some(nk) => { current_k = nk.clone(); actual_next = Some(nk); }
                                        None => { actual_next = None; break; }
                                    }
                                } else {
                                    let approvers = resolve_approvers(pool, &nd.approver_selector, assignee_id.as_deref(), &reporter_id).await;
                                    let av = Value::Array(approvers.iter().map(|(aid,name)| json!({"id":aid,"name":name})).collect());
                                    sqlx::query("UPDATE ticket_workflow_nodes SET approvers=?, updated_at=NOW() WHERE ticket_id=? AND node_key=?")
                                        .bind(&serde_json::to_string(&av).unwrap_or_else(|_|"[]".into())).bind(&id).bind(&current_k)
                                        .execute(pool).await.ok();
                                    break;
                                }
                            } else { break; }
                        }
                    }
                    let st = compiled_opt.as_ref().map(|c| derive_status(actual_next.as_deref(), c)).unwrap_or("in_progress");
                    (actual_next, st)
                }
                None => (None, "in_progress"),
            };
            if let Some(nk) = &final_next {
                sqlx::query("UPDATE tickets SET current_node_key=?, status=?, updated_at=NOW() WHERE id=?")
                    .bind(nk).bind(derived_st).bind(&id).execute(pool).await.ok();
            } else {
                sqlx::query("UPDATE tickets SET current_node_key='__end__', status=?, closed_at=NOW(), updated_at=NOW() WHERE id=?")
                    .bind(derived_st).bind(&id).execute(pool).await.ok();
            }
            let cid = Uuid::new_v4().to_string();
            let cmt = format!("批准节点「{}」 -> {}", n_key, final_next.clone().unwrap_or_else(||"结束".to_string()));
            sqlx::query("INSERT INTO ticket_comments (id, ticket_id, user_id, action, node_key, content, created_at) VALUES (?,?,?,?,?,?,NOW())")
                .bind(&cid).bind(&id).bind(uid(&auth)).bind("approve").bind(&n_key).bind(&cmt)
                .execute(pool).await?;
            // 通知下一节点审批人 / 通知创建人工单已关闭
            if let Some(nk) = &final_next {
                let next_approvers: Vec<(String, String)> = sqlx::query(
                    "SELECT approvers, node_name FROM ticket_workflow_nodes WHERE ticket_id=? AND node_key=?"
                ).bind(&id).bind(nk).fetch_optional(pool).await
                    .ok().flatten()
                    .map(|r| {
                        let name = r.try_get::<String, _>("node_name").unwrap_or_default();
                        let approvers: Vec<Value> = r.try_get::<Option<Value>, _>("approvers").ok().flatten()
                            .and_then(|v| v.as_array().map(|a| a.to_vec()))
                            .unwrap_or_default();
                        approvers.iter().filter_map(|a| {
                            let aid = a.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            if aid.is_empty() { None } else { Some((aid, name.clone())) }
                        }).collect()
                    }).unwrap_or_default();
                let ticket_no = cur_row.get("ticket_no").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let title = cur_row.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
                for (aid, nname) in &next_approvers {
                    let n_title = format!("工单待审批：{}", title);
                    let n_content = format!("工单 {} 已流转到节点「{}」，等待您审批。", ticket_no, nname);
                    let n_link = format!("/tickets/{}", id);
                    crate::notification_routes::create_notification(pool, aid, "ticket_assigned", &n_title, &n_content, &n_link).await;
                }
            } else {
                // 工单结束，通知创建人
                let ticket_no = cur_row.get("ticket_no").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let title = cur_row.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let n_title = format!("工单已办结：{}", title);
                let n_content = format!("工单 {} 已完成全部审批流程。", ticket_no);
                let n_link = format!("/tickets/{}", id);
                crate::notification_routes::create_notification(pool, &reporter_id, "ticket_closed", &n_title, &n_content, &n_link).await;
            }
            return Ok(Json(json!({"code":0,"message":"ok","currentNodeKey":final_next})));
        }
        _ => Err(AppError::bad("未知 decision"))
    }
}

pub async fn close_ticket(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<CloseReq>,
) -> Result<Json<Value>, AppError> {
    crate::auth::require_permission(&auth, "ticket:update")?;
    let pool = &state.db;
    sqlx::query("UPDATE tickets SET status='closed', closed_at=NOW(), updated_at=NOW(),
        current_node_key='__end__', resolution=COALESCE(?, resolution) WHERE id=? AND deleted_at IS NULL")
        .bind(req.resolution.as_deref()).bind(&id).execute(pool).await?;
    sqlx::query("UPDATE ticket_workflow_nodes SET status='done', done_at=NOW(), updated_at=NOW() WHERE ticket_id=? AND status IN ('active','pending')")
        .bind(&id).execute(pool).await.ok();
    let cid = Uuid::new_v4().to_string();
    let cmt = format!("手动关闭工单{}", req.comment.clone().map(|r| format!("：{}", r)).unwrap_or_default());
    sqlx::query("INSERT INTO ticket_comments (id, ticket_id, user_id, action, node_key, content, created_at) VALUES (?,?,?,?,?,?,NOW())")
        .bind(&cid).bind(&id).bind(uid(&auth)).bind("close").bind("__end__").bind(&cmt)
        .execute(pool).await?;
    // 通知创建人工单被关闭
    let row = sqlx::query("SELECT ticket_no, title, reporter_id FROM tickets WHERE id=?")
        .bind(&id).fetch_one(pool).await?;
    let reporter = row.try_get::<String, _>("reporter_id").unwrap_or_default();
    let ticket_no = row.try_get::<String, _>("ticket_no").unwrap_or_default();
    let title = row.try_get::<String, _>("title").unwrap_or_default();
    let n_title = format!("工单已关闭：{}", title);
    let n_content = format!("工单 {} 已被手动关闭。{}", ticket_no, req.comment.clone().map(|c| format!("备注：{}", c)).unwrap_or_default());
    let n_link = format!("/tickets/{}", id);
    if !reporter.is_empty() {
        crate::notification_routes::create_notification(pool, &reporter, "ticket_closed", &n_title, &n_content, &n_link).await;
    }
    Ok(Json(json!({"code":0,"message":"ok"})))
}

pub async fn cancel_ticket(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    crate::auth::require_permission(&auth, "ticket:update")?;
    let pool = &state.db;
    sqlx::query("UPDATE tickets SET status='cancelled', closed_at=NOW(), updated_at=NOW(), current_node_key='__end__' WHERE id=? AND deleted_at IS NULL AND status NOT IN ('closed','cancelled')")
        .bind(&id).execute(pool).await?;
    let cid = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO ticket_comments (id, ticket_id, user_id, action, node_key, content, created_at) VALUES (?,?,?,?,?,?,NOW())")
        .bind(&cid).bind(&id).bind(uid(&auth)).bind("cancel").bind("__end__").bind("取消工单")
        .execute(pool).await?;
    // 通知创建人工单被取消
    if let Ok(row) = sqlx::query("SELECT ticket_no, title, reporter_id FROM tickets WHERE id=?").bind(&id).fetch_one(pool).await {
        let reporter = row.try_get::<String, _>("reporter_id").unwrap_or_default();
        let ticket_no = row.try_get::<String, _>("ticket_no").unwrap_or_default();
        let title = row.try_get::<String, _>("title").unwrap_or_default();
        if !reporter.is_empty() {
            crate::notification_routes::create_notification(pool, &reporter, "ticket_closed",
                &format!("工单已取消：{}", title),
                &format!("工单 {} 已被取消。", ticket_no),
                &format!("/tickets/{}", id)).await;
        }
    }
    Ok(Json(json!({"code":0,"message":"ok"})))
}

// ================= 附件上传 / 列表 / 下载 / 删除 =================

#[derive(Debug, Deserialize)]
pub struct WatchReq {
    pub watched: bool,
}

/// POST /api/tickets/:id/attachments - 附件上传
pub async fn upload_attachment(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(ticket_id): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<Value>, AppError> {
    crate::auth::require_permission(&auth, "ticket:read")?;
    let pool = &state.db;

    // 验证工单存在
    let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM tickets WHERE id=? AND deleted_at IS NULL LIMIT 1")
        .bind(&ticket_id).fetch_optional(pool).await?;
    if exists.is_none() { return Err(AppError::not_found("工单不存在")); }

    let mut filename = String::new();
    let mut content_type = String::new();
    let mut data: Option<Vec<u8>> = None;
    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::bad(&format!("读取上传字段失败: {}", e)))? {
        if field.name() == Some("file") {
            filename = field.file_name().unwrap_or("unnamed").to_string();
            content_type = field.content_type().unwrap_or("application/octet-stream").to_string();
            data = Some(field.bytes().await.map_err(|e| AppError::bad(&format!("读取文件内容失败: {}", e)))?.to_vec());
            break;
        }
    }
    let data = data.ok_or_else(|| AppError::bad("缺少 file 字段"))?;
    let file_size = data.len() as i64;

    // 构建保存路径: ./uploads/tickets/{ticket_id}/{uuid}_{filename}
    let safe_filename = filename.replace('/', "_").replace('\\', "_").replace("..", "_");
    let uuid = Uuid::new_v4().to_string();
    let stored_name = format!("{}_{}", uuid, safe_filename);
    let dir_path = PathBuf::from("./uploads/tickets").join(&ticket_id);
    fs::create_dir_all(&dir_path).map_err(|e| AppError::bad(&format!("创建目录失败: {}", e)))?;
    let file_path = dir_path.join(&stored_name);
    fs::write(&file_path, &data).map_err(|e| AppError::bad(&format!("写入文件失败: {}", e)))?;

    // 写入 ticket_attachments 表
    let att_id = Uuid::new_v4().to_string();
    let uploader_id = uid(&auth).to_string();
    sqlx::query(
        "INSERT INTO ticket_attachments (id, ticket_id, filename, stored_name, file_path, file_size, file_type, uploader_id, created_at)
         VALUES (?,?,?,?,?,?,?,?,NOW())"
    ).bind(&att_id).bind(&ticket_id).bind(&safe_filename).bind(&stored_name)
     .bind(file_path.to_string_lossy().as_ref()).bind(file_size).bind(&content_type).bind(&uploader_id)
     .execute(pool).await?;

    Ok(Json(json!({
        "code": 0,
        "message": "ok",
        "data": {
            "id": att_id,
            "filename": safe_filename,
            "fileSize": file_size,
            "fileType": content_type,
            "uploaderId": uploader_id,
            "createdAt": chrono::Utc::now().naive_utc().format("%Y-%m-%d %H:%M:%S").to_string()
        }
    })))
}

/// GET /api/tickets/:id/attachments - 附件列表
pub async fn list_attachments(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(ticket_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    crate::auth::require_permission(&auth, "ticket:read")?;
    let pool = &state.db;
    let rows = sqlx::query(
        "SELECT a.id, a.filename, a.file_size, a.file_type, a.uploader_id, a.created_at,
                u.display_name AS uploader_name
         FROM ticket_attachments a
         LEFT JOIN users u ON u.id = a.uploader_id
         WHERE a.ticket_id=? AND a.deleted_at IS NULL
         ORDER BY a.created_at ASC"
    ).bind(&ticket_id).fetch_all(pool).await.unwrap_or_default();
    let items: Vec<Value> = rows.iter().map(|r| json!({
        "id": r.try_get::<String,_>("id").unwrap_or_default(),
        "filename": r.try_get::<String,_>("filename").unwrap_or_default(),
        "fileSize": r.try_get::<i64,_>("file_size").unwrap_or(0),
        "fileType": r.try_get::<String,_>("file_type").unwrap_or_default(),
        "uploaderId": r.try_get::<Option<String>,_>("uploader_id").unwrap_or(None),
        "uploaderName": r.try_get::<Option<String>,_>("uploader_name").unwrap_or(None),
        "createdAt": dt_str(r, "created_at"),
    })).collect();
    Ok(Json(json!({"code":0,"message":"ok","data":items})))
}

/// GET /api/tickets/attachments/:id/download - 附件下载
pub async fn download_attachment(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(att_id): Path<String>,
) -> Result<Response<Body>, AppError> {
    crate::auth::require_permission(&auth, "ticket:read")?;
    let pool = &state.db;
    let row = sqlx::query(
        "SELECT filename, file_path, file_type FROM ticket_attachments
         WHERE id=? AND deleted_at IS NULL LIMIT 1"
    ).bind(&att_id).fetch_optional(pool).await?;
    let row = row.ok_or(AppError::not_found("附件不存在"))?;
    let filename: String = row.try_get("filename").unwrap_or_default();
    let file_path: String = row.try_get("file_path").unwrap_or_default();
    let file_type: String = row.try_get("file_type").unwrap_or("application/octet-stream".to_string());

    let data = fs::read(&file_path).map_err(|_| AppError::not_found("文件不存在"))?;

    let disposition = format!("attachment; filename=\"{}\"", filename);
    let body = Body::from(data);
    let response = Response::builder()
        .header(header::CONTENT_TYPE, HeaderValue::from_str(&file_type).unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")))
        .header(header::CONTENT_DISPOSITION, HeaderValue::from_str(&disposition).unwrap_or_else(|_| HeaderValue::from_static("attachment")))
        .body(body)
        .unwrap();
    Ok(response)
}

/// DELETE /api/tickets/attachments/:id - 附件删除（软删除）
pub async fn delete_attachment(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(att_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    crate::auth::require_permission(&auth, "ticket:read")?;
    let pool = &state.db;
    let me_id = uid(&auth);
    let is_admin = auth.0.role == crate::auth::Role::Admin;

    let row = sqlx::query(
        "SELECT uploader_id FROM ticket_attachments WHERE id=? AND deleted_at IS NULL LIMIT 1"
    ).bind(&att_id).fetch_optional(pool).await?;
    let row = row.ok_or(AppError::not_found("附件不存在"))?;
    let uploader: String = row.try_get("uploader_id").unwrap_or_default();

    if !is_admin && uploader != me_id {
        return Err(AppError::forbidden("仅上传者或管理员可删除附件"));
    }

    sqlx::query("UPDATE ticket_attachments SET deleted_at=NOW() WHERE id=?")
        .bind(&att_id).execute(pool).await?;
    Ok(Json(json!({"code":0,"message":"ok"})))
}

// ================= 关注工单 =================

/// POST /api/tickets/:id/watch - 关注/取消关注工单
pub async fn watch_ticket(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(ticket_id): Path<String>,
    Json(req): Json<WatchReq>,
) -> Result<Json<Value>, AppError> {
    crate::auth::require_permission(&auth, "ticket:read")?;
    let pool = &state.db;
    let me_id = uid(&auth);

    // 验证工单存在
    let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM tickets WHERE id=? AND deleted_at IS NULL LIMIT 1")
        .bind(&ticket_id).fetch_optional(pool).await?;
    if exists.is_none() { return Err(AppError::not_found("工单不存在")); }

    if req.watched {
        let wid = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT IGNORE INTO ticket_watchers (id, ticket_id, watcher_id, created_at) VALUES (?,?,?,NOW())"
        ).bind(&wid).bind(&ticket_id).bind(me_id).execute(pool).await?;
    } else {
        sqlx::query(
            "DELETE FROM ticket_watchers WHERE ticket_id=? AND watcher_id=?"
        ).bind(&ticket_id).bind(me_id).execute(pool).await?;
    }
    Ok(Json(json!({"code":0,"message":"ok","watched":req.watched})))
}

/// GET /api/tickets/my-watching - 我的关注列表（分页）
pub async fn my_watching_tickets(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Query(q): Query<PageQ>,
) -> Result<Json<Value>, AppError> {
    crate::auth::require_permission(&auth, "ticket:read")?;
    let pool = &state.db;
    let me_id = uid(&auth);

    let mut wc: Vec<String> = Vec::new();
    let mut binds: Vec<Value> = Vec::new();
    wc.push("tw.watcher_id = ?".into());
    binds.push(json!(me_id));

    push_eq(&mut wc, &mut binds, "t.ticket_type", &q.ticketType);
    push_eq(&mut wc, &mut binds, "t.status", &q.status);
    if let Some(v) = q.priority { wc.push("t.priority = ?".into()); binds.push(json!(v)); }
    push_eq(&mut wc, &mut binds, "t.category", &q.category);
    push_eq(&mut wc, &mut binds, "t.assignee_id", &q.assigneeId);
    push_eq(&mut wc, &mut binds, "t.reporter_id", &q.reporterId);
    push_eq(&mut wc, &mut binds, "t.template_id", &q.templateId);
    if let Some(kw) = &q.keyword { if !kw.is_empty() {
        wc.push("(t.ticket_no LIKE ? OR t.title LIKE ?)".into());
        let p = format!("%{}%", kw); binds.push(json!(p.clone())); binds.push(json!(p));
    }}
    if let Some(s) = q.slaState.as_deref() { match s {
        "breached" => wc.push("t.sla_due_at IS NOT NULL AND t.sla_due_at < NOW() AND t.status NOT IN ('closed','cancelled')".into()),
        "safe"     => wc.push("t.sla_due_at IS NOT NULL AND t.sla_due_at >= NOW() AND t.status NOT IN ('closed','cancelled')".into()),
        "today"    => wc.push("t.sla_due_at IS NOT NULL AND DATE(t.sla_due_at)=DATE(NOW()) AND t.status NOT IN ('closed','cancelled')".into()),
        _ => {}
    }}
    if let Some(d) = &q.createdAtFrom { wc.push("t.created_at >= ?".into()); binds.push(json!(d)); }
    if let Some(d) = &q.createdAtTo   { wc.push("t.created_at <= ?".into()); binds.push(json!(d)); }

    let where_sql = format!(" WHERE t.deleted_at IS NULL AND {}", wc.join(" AND "));
    let count_sql = format!(
        "SELECT COUNT(*) AS cnt FROM tickets t
         INNER JOIN ticket_watchers tw ON tw.ticket_id = t.id
         LEFT JOIN users ua ON ua.id = t.assignee_id
         LEFT JOIN users ur ON ur.id = t.reporter_id
         LEFT JOIN workflow_templates wt ON wt.id = t.template_id
         {}", where_sql);
    let mut cq = sqlx::query(&count_sql);
    for b in &binds { cq = bind_val(cq, b); }
    let total: i64 = cq.fetch_one(pool).await.and_then(|r| r.try_get::<i64,_>("cnt")).unwrap_or(0);

    let lim = q.pageSize.max(1).min(500);
    let off = ((q.page.max(1)-1)*lim).max(0);
    let sql = format!("SELECT t.id, t.ticket_no, t.ticket_type, t.title, t.status, t.priority,
            t.category, t.assignee_id, t.reporter_id, t.sla_due_at, t.current_node_key,
            t.template_id, t.created_at, t.updated_at, t.closed_at, t.resolution,
            ua.display_name AS assignee_name, ur.display_name AS reporter_name,
            wt.name AS template_name
        FROM tickets t
        INNER JOIN ticket_watchers tw ON tw.ticket_id = t.id
        LEFT JOIN users ua ON ua.id = t.assignee_id
        LEFT JOIN users ur ON ur.id = t.reporter_id
        LEFT JOIN workflow_templates wt ON wt.id = t.template_id
        {}
        ORDER BY t.created_at DESC
        LIMIT ? OFFSET ?", where_sql);
    let mut list_q = sqlx::query(&sql);
    for b in &binds { list_q = bind_val(list_q, b); }
    list_q = list_q.bind(lim).bind(off);
    let rows = list_q.fetch_all(pool).await.unwrap_or_default();
    let items: Vec<Value> = rows.iter().map(|r| json!({
        "id": r.try_get::<String,_>("id").unwrap_or_default(),
        "ticketNo": r.try_get::<String,_>("ticket_no").unwrap_or_default(),
        "ticketType": r.try_get::<String,_>("ticket_type").unwrap_or_default(),
        "title": r.try_get::<String,_>("title").unwrap_or_default(),
        "status": r.try_get::<String,_>("status").unwrap_or_default(),
        "priority": r.try_get::<i8,_>("priority").unwrap_or(4),
        "category": r.try_get::<Option<String>,_>("category").unwrap_or(None),
        "assigneeId": r.try_get::<Option<String>,_>("assignee_id").unwrap_or(None),
        "reporterId": r.try_get::<Option<String>,_>("reporter_id").unwrap_or(None),
        "slaDueAt": dt_opt(r, "sla_due_at"),
        "currentNodeKey": r.try_get::<Option<String>,_>("current_node_key").unwrap_or(None),
        "templateId": r.try_get::<Option<String>,_>("template_id").unwrap_or(None),
        "resolution": r.try_get::<Option<String>,_>("resolution").unwrap_or(None),
        "createdAt": dt_str(r, "created_at"),
        "updatedAt": dt_str(r, "updated_at"),
        "closedAt": dt_opt(r, "closed_at"),
        "assigneeName": r.try_get::<Option<String>,_>("assignee_name").unwrap_or(None),
        "reporterName": r.try_get::<Option<String>,_>("reporter_name").unwrap_or(None),
        "templateName": r.try_get::<Option<String>,_>("template_name").unwrap_or(None),
    })).collect();
    Ok(Json(json!({
        "code":0,"message":"ok",
        "data":{"total":total,"page":q.page,"pageSize":q.pageSize,"list":items}
    })))
}

// ================= 工单导出 CSV =================

/// GET /api/tickets/export - 导出 CSV（查询参数和列表一致）
pub async fn export_tickets_csv(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Query(q): Query<PageQ>,
) -> Result<Response<Body>, AppError> {
    crate::auth::require_permission(&auth, "ticket:read")?;
    let pool = &state.db;

    let mut wc: Vec<String> = Vec::new();
    let mut binds: Vec<Value> = Vec::new();
    push_eq(&mut wc, &mut binds, "t.ticket_type", &q.ticketType);
    push_eq(&mut wc, &mut binds, "t.status", &q.status);
    if let Some(v) = q.priority { wc.push("t.priority = ?".into()); binds.push(json!(v)); }
    push_eq(&mut wc, &mut binds, "t.category", &q.category);
    push_eq(&mut wc, &mut binds, "t.assignee_id", &q.assigneeId);
    push_eq(&mut wc, &mut binds, "t.reporter_id", &q.reporterId);
    push_eq(&mut wc, &mut binds, "t.template_id", &q.templateId);
    if let Some(kw) = &q.keyword { if !kw.is_empty() {
        wc.push("(t.ticket_no LIKE ? OR t.title LIKE ?)".into());
        let p = format!("%{}%", kw); binds.push(json!(p.clone())); binds.push(json!(p));
    }}
    if let Some(s) = q.slaState.as_deref() { match s {
        "breached" => wc.push("t.sla_due_at IS NOT NULL AND t.sla_due_at < NOW() AND t.status NOT IN ('closed','cancelled')".into()),
        "safe"     => wc.push("t.sla_due_at IS NOT NULL AND t.sla_due_at >= NOW() AND t.status NOT IN ('closed','cancelled')".into()),
        "today"    => wc.push("t.sla_due_at IS NOT NULL AND DATE(t.sla_due_at)=DATE(NOW()) AND t.status NOT IN ('closed','cancelled')".into()),
        _ => {}
    }}
    if let Some(d) = &q.createdAtFrom { wc.push("t.created_at >= ?".into()); binds.push(json!(d)); }
    if let Some(d) = &q.createdAtTo   { wc.push("t.created_at <= ?".into()); binds.push(json!(d)); }
    let where_sql = if wc.is_empty() { " WHERE t.deleted_at IS NULL".into() } else { format!(" WHERE t.deleted_at IS NULL AND {}", wc.join(" AND ")) };

    let sql = format!("SELECT t.ticket_no, t.ticket_type, t.title, t.priority, t.status,
            ur.display_name AS reporter_name,
            ua.display_name AS assignee_name,
            t.created_at, t.closed_at, t.sla_due_at
        FROM tickets t
        LEFT JOIN users ua ON ua.id = t.assignee_id
        LEFT JOIN users ur ON ur.id = t.reporter_id
        {}
        ORDER BY t.created_at DESC
        LIMIT 10000", where_sql);
    let mut list_q = sqlx::query(&sql);
    for b in &binds { list_q = bind_val(list_q, b); }
    let rows = list_q.fetch_all(pool).await.unwrap_or_default();

    // 构建 CSV（带 UTF-8 BOM，Excel 可直接识别）
    let mut csv = String::new();
    csv.push('\u{FEFF}');
    csv.push_str("工单号,类型,标题,优先级,状态,报告人,处理人,创建时间,关闭时间,SLA截止\n");

    fn csv_escape(s: &str) -> String {
        if s.contains(',') || s.contains('"') || s.contains('\n') {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_string()
        }
    }

    for r in &rows {
        let ticket_no: String = r.try_get("ticket_no").unwrap_or_default();
        let ticket_type: String = r.try_get("ticket_type").unwrap_or_default();
        let title: String = r.try_get("title").unwrap_or_default();
        let priority: i8 = r.try_get("priority").unwrap_or(4);
        let status: String = r.try_get("status").unwrap_or_default();
        let reporter: String = r.try_get::<Option<String>,_>("reporter_name").unwrap_or(None).unwrap_or_default();
        let assignee: String = r.try_get::<Option<String>,_>("assignee_name").unwrap_or(None).unwrap_or_default();
        let created = dt_str(r, "created_at");
        let closed = dt_opt(r, "closed_at").unwrap_or_default();
        let sla_due = dt_opt(r, "sla_due_at").unwrap_or_default();

        let pri_label = format!("P{}", 5 - priority as i64);

        csv.push_str(&format!("{},{},{},{},{},{},{},{},{},{}\n",
            csv_escape(&ticket_no),
            csv_escape(&ticket_type),
            csv_escape(&title),
            csv_escape(&pri_label),
            csv_escape(&status),
            csv_escape(&reporter),
            csv_escape(&assignee),
            csv_escape(&created),
            csv_escape(&closed),
            csv_escape(&sla_due),
        ));
    }

    let body = Body::from(csv.into_bytes());
    let response = Response::builder()
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(header::CONTENT_DISPOSITION, "attachment; filename=\"tickets.csv\"")
        .body(body)
        .unwrap();
    Ok(response)
}

// ================= 工单自定义字段 =================

#[derive(Debug, Deserialize)]
struct CustomFieldItem {
    #[serde(rename = "fieldKey")]
    field_key: String,
    #[serde(rename = "fieldLabel")]
    field_label: String,
    #[serde(rename = "fieldType")]
    field_type: String,
    #[serde(rename = "fieldValue")]
    field_value: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateCustomFieldsReq {
    fields: Vec<CustomFieldItem>,
}

/// GET /api/tickets/:id/custom-fields - 获取工单自定义字段列表
async fn get_custom_fields(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    crate::auth::require_permission(&auth, "ticket:read")?;
    let pool = &state.db;

    // 验证工单存在
    let exists: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM tickets WHERE id=? AND deleted_at IS NULL LIMIT 1"
    )
    .bind(&id)
    .fetch_optional(pool)
    .await?;
    if exists.is_none() {
        return Err(AppError::not_found("工单不存在"));
    }

    let rows = sqlx::query(
        "SELECT id, field_key, field_label, field_type, field_value, created_at, updated_at
         FROM ticket_custom_fields
         WHERE ticket_id = ?
         ORDER BY created_at ASC"
    )
    .bind(&id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let fields: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "id": r.try_get::<String,_>("id").unwrap_or_default(),
                "fieldKey": r.try_get::<String,_>("field_key").unwrap_or_default(),
                "fieldLabel": r.try_get::<String,_>("field_label").unwrap_or_default(),
                "fieldType": r.try_get::<String,_>("field_type").unwrap_or_default(),
                "fieldValue": r.try_get::<Option<String>,_>("field_value").unwrap_or(None),
                "createdAt": dt_str(r, "created_at"),
                "updatedAt": dt_str(r, "updated_at"),
            })
        })
        .collect();

    Ok(Json(json!({ "code": 0, "message": "ok", "data": fields })))
}

/// PUT /api/tickets/:id/custom-fields - 批量更新/插入自定义字段
async fn update_custom_fields(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateCustomFieldsReq>,
) -> Result<Json<Value>, AppError> {
    crate::auth::require_permission(&auth, "ticket:update")?;
    let pool = &state.db;

    // 验证工单存在
    let exists: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM tickets WHERE id=? AND deleted_at IS NULL LIMIT 1"
    )
    .bind(&id)
    .fetch_optional(pool)
    .await?;
    if exists.is_none() {
        return Err(AppError::not_found("工单不存在"));
    }

    let mut tx = pool.begin().await?;

    for field in &req.fields {
        if field.field_key.trim().is_empty() {
            continue;
        }

        // 检查该 field_key 是否已存在
        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM ticket_custom_fields WHERE ticket_id = ? AND field_key = ? LIMIT 1"
        )
        .bind(&id)
        .bind(&field.field_key)
        .fetch_optional(&mut *tx)
        .await?;

        match existing {
            Some(_) => {
                // 更新已存在的字段
                sqlx::query(
                    "UPDATE ticket_custom_fields
                     SET field_label = ?, field_type = ?, field_value = ?, updated_at = NOW()
                     WHERE ticket_id = ? AND field_key = ?"
                )
                .bind(&field.field_label)
                .bind(&field.field_type)
                .bind(&field.field_value)
                .bind(&id)
                .bind(&field.field_key)
                .execute(&mut *tx)
                .await?;
            }
            None => {
                // 插入新字段
                let field_id = Uuid::new_v4().to_string();
                sqlx::query(
                    "INSERT INTO ticket_custom_fields
                     (id, ticket_id, field_key, field_label, field_type, field_value, created_at, updated_at)
                     VALUES (?, ?, ?, ?, ?, ?, NOW(), NOW())"
                )
                .bind(&field_id)
                .bind(&id)
                .bind(&field.field_key)
                .bind(&field.field_label)
                .bind(&field.field_type)
                .bind(&field.field_value)
                .execute(&mut *tx)
                .await?;
            }
        }
    }

    tx.commit().await?;

    Ok(Json(json!({ "code": 0, "message": "ok" })))
}

// ================= 知识库关联推荐 =================

/// GET /api/tickets/:id/knowledge-suggestions - 基于工单内容推荐知识库文章
async fn knowledge_suggestions(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    crate::auth::require_permission(&auth, "ticket:read")?;
    let pool = &state.db;

    // 获取工单标题和描述
    let row = sqlx::query(
        "SELECT title, description FROM tickets WHERE id = ? AND deleted_at IS NULL LIMIT 1"
    )
    .bind(&id)
    .fetch_optional(pool)
    .await?;
    let row = row.ok_or(AppError::not_found("工单不存在"))?;

    let title: String = row.try_get("title").unwrap_or_default();
    let description: Option<String> = row.try_get("description").ok().flatten();

    // 组合搜索文本
    let mut search_text = title.clone();
    if let Some(desc) = &description {
        search_text.push(' ');
        search_text.push_str(desc);
    }

    // 简单关键词提取：取前 100 个字符，按空格和常见标点分割，取非停用词
    let keywords: Vec<String> = search_text
        .chars()
        .take(100)
        .collect::<String>()
        .split(|c: char| c.is_whitespace() || c == ',' || c == '，' || c == '。' || c == '、' || c == ';' || c == '；')
        .filter(|s| !s.is_empty() && s.len() >= 2)
        .take(10)
        .map(|s| s.to_string())
        .collect();

    if keywords.is_empty() {
        return Ok(Json(json!({ "code": 0, "data": [] })));
    }

    // 用 LIKE 匹配标题和摘要，按匹配次数排序，最多返回 5 条
    let mut where_clauses: Vec<String> = Vec::new();
    let mut like_params: Vec<String> = Vec::new();

    for kw in &keywords {
        where_clauses.push("(title LIKE ? OR summary LIKE ? OR content_text LIKE ?)".to_string());
        like_params.push(format!("%{}%", kw));
        like_params.push(format!("%{}%", kw));
        like_params.push(format!("%{}%", kw));
    }

    let where_sql = format!(
        "status = 'published' AND ({})",
        where_clauses.join(" OR ")
    );

    let sql = format!(
        "SELECT id, title, summary FROM knowledge_items
         WHERE {}
         ORDER BY updated_at DESC
         LIMIT 5",
        where_sql
    );

    let mut query = sqlx::query(&sql);
    for param in &like_params {
        query = query.bind(param);
    }

    let rows = query.fetch_all(pool).await.unwrap_or_default();

    let items: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "id": r.try_get::<String,_>("id").unwrap_or_default(),
                "title": r.try_get::<String,_>("title").unwrap_or_default(),
                "summary": r.try_get::<Option<String>,_>("summary").unwrap_or(None),
            })
        })
        .collect();

    Ok(Json(json!({ "code": 0, "data": items })))
}