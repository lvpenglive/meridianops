//! Ticket Routes
//!  - POST   /api/tickets                          创建工单 + 启动流程
//!  - GET    /api/tickets                          分页列表（筛选 + 搜索）
//!  - GET    /api/tickets/kpis                     KPI 汇总
//!  - GET    /api/tickets/:id                      详情 + 运行时节点 + 评论
//!  - PUT    /api/tickets/:id                      编辑元数据
//!  - DELETE /api/tickets/:id                      软删除
//!  - POST   /api/tickets/:id/assign               指定受理人
//!  - POST   /api/tickets/:id/link-alert           关联告警
//!  - POST   /api/tickets/:id/unlink-alert/:alertId  取消关联
//!  - POST   /api/tickets/:id/actions/:nodeId      approve/reject/reassign/comment
//!  - POST   /api/tickets/:id/close                强制关闭 + 审计
//!  - POST   /api/tickets/:id/cancel               取消工单

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
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

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/tickets", post(create_ticket).get(list_tickets))
        .route("/api/tickets/kpis", get(ticket_kpis))
        .route("/api/tickets/:id", get(get_ticket).put(update_ticket).delete(delete_ticket))
        .route("/api/tickets/:id/assign", post(assign_ticket))
        .route("/api/tickets/:id/link-alert", post(link_alert))
        .route("/api/tickets/:id/unlink-alert/:alertId", post(unlink_alert))
        .route("/api/tickets/:id/actions/:nodeId", post(action_on_node))
        .route("/api/tickets/:id/close", post(close_ticket))
        .route("/api/tickets/:id/cancel", post(cancel_ticket))
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
            t.template_id, t.created_at, t.updated_at, t.resolution,
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
        "slaDueAt": r.try_get::<Option<String>,_>("sla_due_at").unwrap_or(None),
        "currentNodeKey": r.try_get::<Option<String>,_>("current_node_key").unwrap_or(None),
        "templateId": r.try_get::<Option<String>,_>("template_id").unwrap_or(None),
        "resolution": r.try_get::<Option<String>,_>("resolution").unwrap_or(None),
        "createdAt": r.try_get::<String,_>("created_at").unwrap_or_default(),
        "updatedAt": r.try_get::<String,_>("updated_at").unwrap_or_default(),
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
        "slaDueAt": t_row.try_get::<Option<String>,_>("sla_due_at").unwrap_or(None),
        "currentNodeKey": t_row.try_get::<Option<String>,_>("current_node_key").unwrap_or(None),
        "templateId": t_row.try_get::<Option<String>,_>("template_id").unwrap_or(None),
        "resolution": t_row.try_get::<Option<String>,_>("resolution").unwrap_or(None),
        "createdAt": t_row.try_get::<String,_>("created_at").unwrap_or_default(),
        "updatedAt": t_row.try_get::<String,_>("updated_at").unwrap_or_default(),
        "closedAt": t_row.try_get::<Option<String>,_>("closed_at").unwrap_or(None),
        "assigneeName": t_row.try_get::<Option<String>,_>("assignee_name").unwrap_or(None),
        "reporterName": t_row.try_get::<Option<String>,_>("reporter_name").unwrap_or(None),
        "templateName": t_row.try_get::<Option<String>,_>("template_name").unwrap_or(None),
        "templateDefinition": t_row.try_get::<Option<Value>,_>("template_definition").unwrap_or(None),
    });
    let p = t.get("priority").and_then(|v|v.as_i64()).unwrap_or(3) as i8;
    let (mtta_h, mttr_h) = crate::workflow_engine::sla_hours(p);

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
        "enteredAt": r.try_get::<Option<String>,_>("entered_at").unwrap_or(None),
        "doneAt": r.try_get::<Option<String>,_>("done_at").unwrap_or(None),
        "decision": r.try_get::<Option<String>,_>("decision").unwrap_or(None),
        "deciderId": r.try_get::<Option<String>,_>("decider_id").unwrap_or(None),
        "timeoutHours": r.try_get::<Option<i64>,_>("timeout_hours").unwrap_or(None),
        "timeoutAction": r.try_get::<Option<String>,_>("timeout_action").unwrap_or(None),
        "rejectBackTo": r.try_get::<Option<String>,_>("reject_back_to").unwrap_or(None),
        "outs": r.try_get::<Option<Value>,_>("outs").unwrap_or(Some(Value::Array(vec![]))).unwrap_or(Value::Array(vec![])),
        "extra": r.try_get::<Option<Value>,_>("extra").unwrap_or(None),
        "updatedAt": r.try_get::<Option<String>,_>("updated_at").unwrap_or(None),
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
        "createdAt": r.try_get::<String,_>("created_at").unwrap_or_default(),
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
        "createdAt": r.try_get::<String,_>("created_at").unwrap_or_default(),
        "alertTitle": r.try_get::<Option<String>,_>("alert_title").unwrap_or(None),
        "alertSeverity": r.try_get::<Option<String>,_>("alert_severity").unwrap_or(None),
    })).collect();

    Ok(Json(json!({
        "code":0,"message":"ok",
        "data":{
            "ticket": t,
            "workflowNodes": nodes,
            "comments": comments,
            "alertLinks": links,
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
    Ok(Json(json!({"code":0,"message":"ok"})))
}