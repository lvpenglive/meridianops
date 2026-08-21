//! 工单 + 工作流引擎核心：
//!   1. 工单编号生成器 (WO-YYYYMMDD-NNN)
//!   2. SLA 期限计算 (按 priority)
//!   3. LogicFlow definition -> 运行时 nodes 编译校验器
//!   4. 审批人解析 + 条件出边匹配 + status 派生

use chrono::{Duration, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::MySqlPool;

use crate::error::AppError;

// ======= 1. 编号生成器 =======

pub async fn generate_ticket_no(pool: &MySqlPool) -> Result<String, AppError> {
    let today = Utc::now().format("%Y%m%d").to_string();
    let prefix = format!("WO-{}-", today);
    for attempt in 1..=3 {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tickets WHERE ticket_no LIKE CONCAT(?, '%')",
        )
        .bind(&prefix)
        .fetch_one(pool)
        .await
        .unwrap_or(0);
        let next = (count + 1) as u32;
        let no = format!("{}{:03}", prefix, next);
        let id = uuid::Uuid::new_v4().to_string();
        let res = sqlx::query(
            "INSERT IGNORE INTO ticket_number_seq (id, date_prefix, seq, created_at) VALUES (?, ?, ?, NOW())",
        )
        .bind(&id)
        .bind(&today)
        .bind(next as i64)
        .execute(pool)
        .await;
        match res {
            Ok(r) if r.rows_affected() == 1 => return Ok(no),
            _ => {
                if attempt == 3 {
                    let t = Utc::now().format("%Y%m%d-%H%M%S%3f").to_string();
                    return Ok(format!("WO-{}", t));
                }
                tokio::time::sleep(std::time::Duration::from_millis(5 * attempt)).await;
            }
        }
    }
    let t = Utc::now().format("%Y%m%d-%H%M%S%6f").to_string();
    Ok(format!("WO-{}", t))
}

// ======= 2. SLA =======

pub fn sla_hours(priority: i8) -> (i64, i64) {
    match priority {
        1 => (1, 4),
        2 => (4, 48),
        3 => (24, 7 * 24),
        4 => (72, 30 * 24),
        _ => (24, 7 * 24),
    }
}
pub fn compute_sla_due(priority: i8, created_at_rfc: &str) -> Option<String> {
    let (_, mttr) = sla_hours(priority);
    chrono::DateTime::parse_from_rfc3339(created_at_rfc)
        .ok()
        .map(|dt| {
            let dt_utc = dt.with_timezone(&Utc);
            (dt_utc + Duration::hours(mttr)).to_rfc3339()
        })
}

// ======= 3. 编译器 =======

#[derive(Debug, Deserialize)]
pub struct LfDefinition {
    pub nodes: Vec<LfNode>,
    pub edges: Vec<LfEdge>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LfNode {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub properties: Value,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LfEdge {
    pub id: String,
    #[serde(rename = "sourceNodeId")]
    pub source: String,
    #[serde(rename = "targetNodeId")]
    pub target: String,
    #[serde(default, rename = "type")]
    pub edge_type: String,
    #[serde(default)]
    pub properties: Value,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledNode {
    pub index: i32,
    pub key: String,
    pub name: String,
    pub kind: String,
    pub approver_selector: Value,
    #[serde(rename = "timeoutHours", default)]
    pub timeout_hours: Option<i64>,
    #[serde(rename = "timeoutAction", default)]
    pub timeout_action: Option<String>,
    #[serde(rename = "rejectBackTo", default)]
    pub reject_back_to: Option<String>,
    #[serde(rename = "onPassAuto", default)]
    pub on_pass_auto: Option<Value>,
    #[serde(default)] pub ins: Vec<String>,
    #[serde(default)] pub outs: Vec<Value>,
}

pub fn compile_definition(def: &LfDefinition) -> (Vec<CompiledNode>, Vec<String>) {
    let mut errors = Vec::<String>::new();
    let mut by_id = std::collections::BTreeMap::new();
    for n in &def.nodes {
        if by_id.contains_key(&n.id) { errors.push(format!("节点ID重复: {}", n.id)); }
        by_id.insert(n.id.clone(), n.clone());
    }
    let starts: Vec<&LfNode> = def.nodes.iter().filter(|n| n.kind == "start").collect();
    if starts.is_empty() { errors.push("缺少开始节点".into()); }
    if starts.len() > 1 { errors.push(format!("开始节点 {} 个，仅允许 1 个", starts.len())); }
    let ends: Vec<&LfNode> = def.nodes.iter().filter(|n| n.kind == "end").collect();
    if ends.is_empty() { errors.push("缺少结束节点".into()); }

    for n in &def.nodes {
        let is_se = n.kind == "start" || n.kind == "end";
        let key = n.properties.get("key").and_then(|v| v.as_str()).unwrap_or("");
        let name = n.properties.get("name").and_then(|v| v.as_str()).unwrap_or("");
        if !is_se {
            if key.is_empty() { errors.push(format!("节点 {} 缺少 key", n.id)); }
            if name.is_empty() { errors.push(format!("节点 {}({}) 缺少 name", n.id, key)); }
            if !matches!(&*n.kind,
                "auto_pass"|"single_approval"|"any_approval"|"all_approval"|
                "countersign"|"parallel_split"|"parallel_join"|"condition_gateway")
            {
                errors.push(format!("节点 {} 类型非法: {}", n.id, n.kind));
            }
        }
    }
    for e in &def.edges {
        if !by_id.contains_key(&e.source) { errors.push(format!("边 {} 源节点不存在: {}", e.id, e.source)); }
        if !by_id.contains_key(&e.target) { errors.push(format!("边 {} 目标不存在: {}", e.id, e.target)); }
    }
    for n in &def.nodes {
        if n.kind == "condition_gateway" {
            let outs_e: Vec<&LfEdge> = def.edges.iter().filter(|e| e.source == n.id).collect();
            if outs_e.is_empty() { errors.push(format!("条件网关 {} 无出边", n.id)); continue; }
            let has_cond = outs_e.iter().any(|e| {
                let c = e.properties.get("condition");
                match c {
                    Some(Value::Null) | None => false,
                    Some(Value::String(s)) => !s.is_empty(),
                    Some(_) => true,
                }
            });
            if !has_cond && outs_e.len() > 1 {
                errors.push(format!("条件网关 {} 多条出边均无条件", n.id));
            }
        }
    }
    if !errors.is_empty() { return (Vec::new(), errors); }

    let mut out_adj: std::collections::BTreeMap<String, Vec<&LfEdge>> = std::collections::BTreeMap::new();
    for e in &def.edges { out_adj.entry(e.source.clone()).or_default().push(e); }
    let start_id = starts[0].id.clone();
    let mut order = Vec::new();
    let mut visited = std::collections::BTreeSet::new();
    let mut q = std::collections::VecDeque::new();
    q.push_back(start_id.clone()); visited.insert(start_id);
    while let Some(id) = q.pop_front() {
        order.push(id.clone());
        if let Some(es) = out_adj.get(&id) {
            for e in es {
                if visited.insert(e.target.clone()) { q.push_back(e.target.clone()); }
            }
        }
    }
    for n in &def.nodes {
        if !visited.contains(&n.id) { errors.push(format!("节点 {} 不可达", n.id)); }
    }
    let end_ids: std::collections::BTreeSet<String> = ends.iter().map(|e| e.id.clone()).collect();
    for n in &def.nodes {
        if end_ids.contains(&n.id) { continue; }
        if !can_reach_end(&n.id, &out_adj, &end_ids) {
            errors.push(format!("节点 {} 无到结束的路径", n.id));
        }
    }
    if !errors.is_empty() { return (Vec::new(), errors); }

    let mut key_to_idx: std::collections::BTreeMap<String, i32> = std::collections::BTreeMap::new();
    let mut compiled: Vec<CompiledNode> = Vec::new();
    let mut idx = 0_i32;
    for id in &order {
        let n = &by_id[id];
        let (key, kind) = if n.kind == "start" { ("__start__".to_string(), "start".to_string()) }
        else if n.kind == "end" { ("__end__".to_string(), "end".to_string()) }
        else {
            let k = n.properties.get("key").and_then(|v| v.as_str()).unwrap_or("").to_string();
            (k, n.kind.clone())
        };
        let name = n.properties.get("name").and_then(|v| v.as_str()).unwrap_or(&key).to_string();
        let approver = n.properties.get("approverSelector").cloned().unwrap_or(Value::Null);
        let to_h = n.properties.get("timeoutHours").and_then(|v| v.as_i64());
        let to_a = n.properties.get("timeoutAction").and_then(|v| v.as_str()).map(String::from);
        let rej  = n.properties.get("rejectBackTo").and_then(|v| v.as_str()).map(String::from);
        let on_pass = n.properties.get("onPassAuto").cloned();
        key_to_idx.insert(key.clone(), idx);
        compiled.push(CompiledNode {
            index: idx, key, name, kind, approver_selector: approver,
            timeout_hours: to_h, timeout_action: to_a, reject_back_to: rej,
            on_pass_auto: on_pass, ins: Vec::new(), outs: Vec::new(),
        });
        idx += 1;
    }
    for e in &def.edges {
        let src_n = by_id.get(&e.source).unwrap();
        let dst_n = by_id.get(&e.target).unwrap();
        let src_key = if src_n.kind == "start" { "__start__".into() }
        else if src_n.kind == "end" { continue }
        else { src_n.properties.get("key").and_then(|v| v.as_str()).unwrap_or("").to_string() };
        let dst_key = if dst_n.kind == "end" { "__end__".into() }
        else if dst_n.kind == "start" { continue }
        else { dst_n.properties.get("key").and_then(|v| v.as_str()).unwrap_or("").to_string() };
        let priority = e.properties.get("priority").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let condition = e.properties.get("condition").cloned().unwrap_or(Value::Null);
        if let Some(i) = key_to_idx.get(&src_key) {
            if let Some(src) = compiled.get_mut(*i as usize) {
                src.outs.push(json!({"to": dst_key, "condition": condition, "priority": priority}));
            }
        }
        if let Some(i) = key_to_idx.get(&dst_key) {
            if let Some(dst) = compiled.get_mut(*i as usize) {
                dst.ins.push(src_key.clone());
            }
        }
    }
    for c in compiled.iter_mut() {
        c.outs.sort_by(|a, b| {
            let pa = a.get("priority").and_then(|v| v.as_i64()).unwrap_or(0);
            let pb = b.get("priority").and_then(|v| v.as_i64()).unwrap_or(0);
            pa.cmp(&pb)
        });
    }
    (compiled, errors)
}

fn can_reach_end(from: &str,
    out_adj: &std::collections::BTreeMap<String, Vec<&LfEdge>>,
    ends: &std::collections::BTreeSet<String>) -> bool
{
    let mut stack = vec![from.to_string()];
    let mut seen = std::collections::BTreeSet::new();
    seen.insert(from.to_string());
    while let Some(cur) = stack.pop() {
        if ends.contains(&cur) { return true; }
        if let Some(es) = out_adj.get(&cur) {
            for e in es {
                if seen.insert(e.target.clone()) { stack.push(e.target.clone()); }
            }
        }
    }
    false
}

// ======= 4. 运行时辅助 =======

pub async fn resolve_approvers(
    pool: &MySqlPool,
    selector: &Value,
    ticket_assignee_id: Option<&str>,
    ticket_reporter_id: &str,
) -> Vec<(String, String)> {
    let mut items = Vec::<String>::new();
    match selector {
        Value::String(s) => items.push(s.clone()),
        Value::Array(arr) => items.extend(arr.iter().filter_map(|v| v.as_str().map(String::from))),
        _ => return Vec::new(),
    }
    let mut out = Vec::<(String, String)>::new();
    let mut user_ids = Vec::<String>::new();
    let mut role_names = Vec::<String>::new();
    let mut dept_ids = Vec::<String>::new();

    for it in items {
        let (prefix, rest) = match it.find(':') {
            Some(i) => (it[..i].to_string(), it[i+1..].to_string()),
            None => (it, String::new()),
        };
        match prefix.as_str() {
            "assignee" => if let Some(a) = ticket_assignee_id { user_ids.push(a.to_string()); },
            "reporter" => user_ids.push(ticket_reporter_id.to_string()),
            "tester" => user_ids.push(ticket_reporter_id.to_string()),
            "user" => if !rest.is_empty() { user_ids.push(rest); },
            "role" => if !rest.is_empty() { role_names.push(rest); },
            "dept" => if !rest.is_empty() { dept_ids.push(rest); },
            "team_leader_of_assignee" => {
                if let Some(aid) = ticket_assignee_id {
                    if let Ok(Some((Some(did),))) = sqlx::query_as::<_, (Option<String>,)>(
                        "SELECT department_id FROM users WHERE id = ?"
                    ).bind(aid).fetch_optional(pool).await {
                        if let Ok(r) = sqlx::query_as::<_, (String, String)>(
                            "SELECT u.id, u.display_name FROM users u
                             INNER JOIN roles r ON r.id = u.role_id
                             WHERE u.department_id = ? AND u.enabled = 1
                               AND (r.name LIKE '%leader%' OR r.display_name LIKE '%组长%') LIMIT 20"
                        ).bind(&did).fetch_all(pool).await { out.extend(r); }
                    }
                }
            }
            "department_head_of_reporter" => {
                if let Ok(Some((Some(did),))) = sqlx::query_as::<_, (Option<String>,)>(
                    "SELECT department_id FROM users WHERE id = ?"
                ).bind(ticket_reporter_id).fetch_optional(pool).await {
                    if let Ok(r) = sqlx::query_as::<_, (String, String)>(
                        "SELECT u.id, u.display_name FROM users u
                         INNER JOIN roles r ON r.id = u.role_id
                         WHERE u.department_id = ? AND u.enabled = 1
                           AND (r.name LIKE '%head%' OR r.display_name LIKE '%经理%' OR r.name='admin') LIMIT 20"
                    ).bind(&did).fetch_all(pool).await { out.extend(r); }
                }
            }
            "incident_manager" | "problem_manager" | "task_dispatcher"
                | "cab_member" | "senior_engineer_group" | "vp_oncall" => {
                if let Ok(r) = sqlx::query_as::<_, (String, String)>(
                    "SELECT u.id, u.display_name FROM users u
                     INNER JOIN roles r ON r.id = u.role_id
                     WHERE u.enabled = 1 AND r.name IN ('admin', 'operator') LIMIT 20"
                ).fetch_all(pool).await { out.extend(r); }
            }
            _ => {}
        }
    }
    for uid in user_ids {
        if let Ok(Some(r)) = sqlx::query_as::<_, (String, String)>(
            "SELECT id, display_name FROM users WHERE id = ? AND enabled = 1"
        ).bind(&uid).fetch_optional(pool).await { out.push(r); }
    }
    for rn in role_names {
        if let Ok(r) = sqlx::query_as::<_, (String, String)>(
            "SELECT u.id, u.display_name FROM users u
             INNER JOIN roles r ON r.id = u.role_id
             WHERE u.enabled = 1 AND (r.name = ? OR r.display_name LIKE CONCAT('%',?,'%')) LIMIT 50"
        ).bind(&rn).bind(&rn).fetch_all(pool).await { out.extend(r); }
    }
    for did in dept_ids {
        if let Ok(r) = sqlx::query_as::<_, (String, String)>(
            "SELECT id, display_name FROM users WHERE enabled=1 AND department_id=? LIMIT 50"
        ).bind(&did).fetch_all(pool).await { out.extend(r); }
    }
    out.sort(); out.dedup(); out
}

pub fn pick_next_node(outs: &[Value], ticket_ctx: &Value) -> Option<String> {
    for o in outs {
        let cond = o.get("condition").unwrap_or(&Value::Null);
        let is_default = match cond {
            Value::Null => true, Value::String(s) => s.is_empty(), _ => false,
        };
        if !is_default {
            if let (Some(field), Some(op)) = (
                cond.get("field").and_then(|v| v.as_str()),
                cond.get("op").and_then(|v| v.as_str()),
            ) {
                let val = cond.get("value").cloned().unwrap_or(Value::Null);
                let left = ticket_ctx.get(field).cloned().unwrap_or(Value::Null);
                if !match_cond(&left, op, &val) { continue; }
                return o.get("to").and_then(|v| v.as_str()).map(String::from);
            }
        }
    }
    for o in outs {
        let cond = o.get("condition").unwrap_or(&Value::Null);
        let is_default = match cond {
            Value::Null => true, Value::String(s) => s.is_empty(), _ => false,
        };
        if is_default { return o.get("to").and_then(|v| v.as_str()).map(String::from); }
    }
    None
}
fn match_cond(left: &Value, op: &str, right: &Value) -> bool {
    match op {
        "==" | "=" => left == right,
        "!=" | "<>" => left != right,
        ">"  => num_cmp(left, right, |a, b| a > b),
        ">=" => num_cmp(left, right, |a, b| a >= b),
        "<"  => num_cmp(left, right, |a, b| a < b),
        "<=" => num_cmp(left, right, |a, b| a <= b),
        "in" => right.as_array().map(|arr| arr.contains(left)).unwrap_or(false),
        "contains" => left.as_str().zip(right.as_str()).map(|(a,b)| a.contains(b)).unwrap_or(false),
        _ => false,
    }
}
fn num_cmp<F: FnOnce(f64, f64) -> bool>(left: &Value, right: &Value, f: F) -> bool {
    let a = match left {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    };
    let b = match right {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    };
    a.zip(b).map(|(x, y)| f(x, y)).unwrap_or(false)
}

pub fn derive_status(current_node_key: Option<&str>, nodes: &[CompiledNode]) -> &'static str {
    let Some(ck) = current_node_key else { return "open"; };
    let node = nodes.iter().find(|n| n.key == ck);
    let kind = node.map(|n| n.kind.as_str()).unwrap_or("");
    let name = node.map(|n| n.name.as_str()).unwrap_or("");
    match (ck, kind) {
        (_, "start") => "open",
        ("dispatch", "auto_pass") => "assigned",
        (_, "end") => "closed",
        (_, "auto_pass") | (_, "condition_gateway") | (_, "parallel_split") | (_, "parallel_join") => "in_progress",
        (_, "single_approval") | (_, "any_approval") | (_, "all_approval") | (_, "countersign") => {
            if name.contains("确认") || name.contains("审核") || name.contains("审批") || name.contains("验收") || name.contains("验证") {
                "pending_review"
            } else {
                "in_progress"
            }
        }
        _ => "open",
    }
}

pub fn parse_dt(s: &Option<String>) -> Option<NaiveDateTime> {
    let s = s.as_ref()?;
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) { return Some(dt.naive_utc()); }
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").ok()
}