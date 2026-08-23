//! SLA 超时自动升级后台调度器
//!
//! 每 5 分钟扫描一次 ticket_workflow_nodes 中已超时的激活节点：
//! - status='active'
//! - timeout_hours IS NOT NULL
//! - entered_at + INTERVAL timeout_hours HOUR < NOW()
//!
//! 对于超时节点：
//!   a. 若 timeout_action='escalate'，自动升级到上级审批人（部门主管加入审批列表）
//!   b. 发送站内通知给相关审批人
//!   c. 在 ticket_comments 表记录超时审计

use serde_json::{json, Value};
use sqlx::{MySqlPool, Row};
use uuid::Uuid;

/// 启动 SLA 超时调度器。应通过 tokio::spawn 调用。
pub async fn start_scheduler(pool: sqlx::MySqlPool) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
    // 第一次 tick 立即触发，不等待 5 分钟
    interval.tick().await;
    tracing::info!("ticket_scheduler started (interval=5min)");

    loop {
        interval.tick().await;
        if let Err(e) = scan_and_handle_timeouts(&pool).await {
            tracing::error!("ticket_scheduler scan error: {}", e);
        }
    }
}

/// 单次扫描：找出所有超时的 active 节点并处理。
async fn scan_and_handle_timeouts(pool: &MySqlPool) -> anyhow::Result<()> {
    let rows = sqlx::query(
        "SELECT wn.id, wn.ticket_id, wn.node_key, wn.node_name, wn.approvers,
                wn.timeout_hours, wn.timeout_action, wn.entered_at, wn.extra,
                t.title, t.reporter_id, t.assignee_id, t.ticket_no
         FROM ticket_workflow_nodes wn
         INNER JOIN tickets t ON t.id = wn.ticket_id
         WHERE wn.status = 'active'
           AND wn.timeout_hours IS NOT NULL
           AND wn.entered_at IS NOT NULL
           AND DATE_ADD(wn.entered_at, INTERVAL wn.timeout_hours HOUR) < NOW()
           AND t.deleted_at IS NULL",
    )
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(());
    }

    tracing::info!(
        "ticket_scheduler: found {} timed-out active node(s)",
        rows.len()
    );

    for row in &rows {
        let node_id: String = row.try_get("id")?;
        let ticket_id: String = row.try_get("ticket_id")?;
        let node_key: String = row.try_get("node_key")?;
        let node_name: String = row.try_get("node_name")?;
        let timeout_action: Option<String> = row.try_get("timeout_action").ok().flatten();
        let approvers_val: Option<Value> = row.try_get("approvers").ok().flatten();
        let extra_val: Option<Value> = row.try_get("extra").ok().flatten();
        let ticket_title: String = row.try_get("title")?;
        let ticket_no: String = row.try_get("ticket_no")?;
        let reporter_id: String = row.try_get("reporter_id")?;
        let assignee_id: Option<String> = row.try_get("assignee_id").ok().flatten();

        // 幂等保护：检查 extra 中是否已标记 escalated
        if let Some(extra) = &extra_val {
            if extra.get("escalated").and_then(|v| v.as_bool()).unwrap_or(false) {
                continue;
            }
        }

        let action = timeout_action.as_deref().unwrap_or("");

        match action {
            "escalate" => {
                handle_escalate(
                    pool,
                    &node_id,
                    &ticket_id,
                    &node_key,
                    &node_name,
                    approvers_val.as_ref(),
                    &reporter_id,
                    assignee_id.as_deref(),
                    &ticket_no,
                    &ticket_title,
                    extra_val.as_ref(),
                )
                .await;
            }
            _ => {
                // 其他超时动作（如 auto_pass / auto_close 等）暂不处理，
                // 仅记录审计日志
                record_timeout_comment(
                    pool,
                    &ticket_id,
                    &node_key,
                    &node_name,
                    action,
                    &format!("节点「{}」已超时（动作：{}）", node_name, action),
                )
                .await;
            }
        }
    }

    Ok(())
}

/// 处理 escalate 动作：将部门主管加入审批人列表，发送通知，记录审计。
async fn handle_escalate(
    pool: &MySqlPool,
    node_id: &str,
    ticket_id: &str,
    node_key: &str,
    node_name: &str,
    approvers: Option<&Value>,
    reporter_id: &str,
    assignee_id: Option<&str>,
    ticket_no: &str,
    ticket_title: &str,
    extra: Option<&Value>,
) {
    // 1. 找到提报人所在部门的主管（升级审批人）
    let dept_heads = find_department_heads(pool, reporter_id).await;
    if dept_heads.is_empty() {
        tracing::warn!(
            "ticket_scheduler escalate: no department head found for reporter {} on ticket {}",
            reporter_id,
            ticket_id
        );
        return;
    }

    // 2. 合并到现有审批人列表（去重）
    let mut approver_list: Vec<(String, String)> = Vec::new();
    if let Some(apr) = approvers {
        if let Some(arr) = apr.as_array() {
            for a in arr {
                let id = a
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let name = a
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if !id.is_empty() {
                    approver_list.push((id, name));
                }
            }
        }
    }

    let mut new_approvers_added = Vec::new();
    for (hid, hname) in &dept_heads {
        if !approver_list.iter().any(|(id, _)| id == hid) {
            approver_list.push((hid.clone(), hname.clone()));
            new_approvers_added.push((hid.clone(), hname.clone()));
        }
    }

    if new_approvers_added.is_empty() {
        tracing::debug!(
            "ticket_scheduler escalate: no new approvers to add for node {}",
            node_id
        );
        // 即使没有新增审批人，也标记为已升级避免重复扫描
        mark_node_escalated(pool, node_id, extra).await;
        return;
    }

    let approvers_json = json!(approver_list
        .iter()
        .map(|(id, name)| json!({ "id": id, "name": name }))
        .collect::<Vec<_>>());

    // 3. 更新 ticket_workflow_nodes.approvers 并标记已升级
    let res = update_node_approvers_and_escalated(pool, node_id, &approvers_json, extra).await;
    if res.is_err() {
        tracing::error!(
            "ticket_scheduler escalate: failed to update approvers for node {}: {}",
            node_id,
            res.unwrap_err()
        );
        return;
    }

    // 4. 发送通知给新增的审批人
    let title = format!("【超时升级】{}", ticket_title);
    let content = format!(
        "工单 {} 节点「{}」已超时，已自动升级请您审批。",
        ticket_no, node_name
    );
    let link = format!("/tickets/{}", ticket_id);
    for (uid, _uname) in &new_approvers_added {
        crate::notification_routes::create_notification(
            pool, uid, "ticket_escalated", &title, &content, &link,
        )
        .await;
    }

    // 5. 在 ticket_comments 记录超时审计
    let head_names: Vec<&str> = new_approvers_added.iter().map(|(_, n)| n.as_str()).collect();
    let comment = format!(
        "节点「{}」超时，已自动升级，新增审批人：{}",
        node_name,
        head_names.join("、")
    );
    record_timeout_comment(pool, ticket_id, node_key, node_name, "escalate", &comment).await;

    tracing::info!(
        "ticket_scheduler: escalated ticket {} node '{}', added {} approver(s)",
        ticket_no,
        node_key,
        new_approvers_added.len()
    );

    // 让 未使用的 assignee_id 参数不报警告
    let _ = assignee_id;
}

/// 查询提报人所在部门的主管（经理/管理员）。
async fn find_department_heads(pool: &MySqlPool, reporter_id: &str) -> Vec<(String, String)> {
    let result: Option<(Option<String>,)> = sqlx::query_as::<_, (Option<String>,)>(
        "SELECT department_id FROM users WHERE id = ?",
    )
    .bind(reporter_id)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);

    let Some((Some(dept_id),)) = result else {
        return Vec::new();
    };

    sqlx::query_as::<_, (String, String)>(
        "SELECT u.id, u.display_name
         FROM users u
         INNER JOIN roles r ON r.id = u.role_id
         WHERE u.department_id = ? AND u.enabled = 1
           AND (r.name LIKE '%head%' OR r.display_name LIKE '%经理%' OR r.name = 'admin')
         LIMIT 20",
    )
    .bind(&dept_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

/// 更新节点审批人列表并标记为已升级（通过 extra.escalated=true）。
async fn update_node_approvers_and_escalated(
    pool: &MySqlPool,
    node_id: &str,
    approvers: &Value,
    current_extra: Option<&Value>,
) -> anyhow::Result<()> {
    let mut extra_obj = match current_extra {
        Some(Value::Object(obj)) => obj.clone(),
        _ => serde_json::Map::new(),
    };
    extra_obj.insert("escalated".into(), Value::Bool(true));
    extra_obj.insert(
        "escalated_at".into(),
        Value::String(chrono::Utc::now().to_rfc3339()),
    );
    let extra_json = Value::Object(extra_obj);

    let approvers_str = serde_json::to_string(approvers)?;
    let extra_str = serde_json::to_string(&extra_json)?;

    sqlx::query(
        "UPDATE ticket_workflow_nodes
         SET approvers = ?, extra = ?, updated_at = NOW()
         WHERE id = ?",
    )
    .bind(&approvers_str)
    .bind(&extra_str)
    .bind(node_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// 仅标记节点为已升级（无新增审批人时使用）。
async fn mark_node_escalated(pool: &MySqlPool, node_id: &str, current_extra: Option<&Value>) {
    let mut extra_obj = match current_extra {
        Some(Value::Object(obj)) => obj.clone(),
        _ => serde_json::Map::new(),
    };
    extra_obj.insert("escalated".into(), Value::Bool(true));
    extra_obj.insert(
        "escalated_at".into(),
        Value::String(chrono::Utc::now().to_rfc3339()),
    );
    let extra_json = Value::Object(extra_obj);
    let extra_str = serde_json::to_string(&extra_json).unwrap_or_default();

    let _ = sqlx::query(
        "UPDATE ticket_workflow_nodes SET extra = ?, updated_at = NOW() WHERE id = ?",
    )
    .bind(&extra_str)
    .bind(node_id)
    .execute(pool)
    .await;
}

/// 在 ticket_comments 表记录一条超时审计记录。
async fn record_timeout_comment(
    pool: &MySqlPool,
    ticket_id: &str,
    node_key: &str,
    _node_name: &str,
    action: &str,
    content: &str,
) {
    let id = Uuid::new_v4().to_string();
    let action_str = if action == "escalate" {
        "timeout_escalate"
    } else {
        "timeout"
    };
    let extra = json!({ "timeout_action": action });
    let extra_str = serde_json::to_string(&extra).unwrap_or_default();

    let _ = sqlx::query(
        "INSERT INTO ticket_comments (id, ticket_id, user_id, action, node_key, content, extra, created_at)
         VALUES (?, ?, NULL, ?, ?, ?, ?, NOW())",
    )
    .bind(&id)
    .bind(ticket_id)
    .bind(action_str)
    .bind(node_key)
    .bind(content)
    .bind(&extra_str)
    .execute(pool)
    .await;
}
