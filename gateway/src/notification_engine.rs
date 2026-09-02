//! 通知引擎 — 通道抽象 + 邮件/飞书/通用 Webhook 实现 + 规则匹配分发
//!
//! 核心流程：
//!   1. 各业务模块调用 `dispatch_event()` 传入事件类型 + 上下文
//!   2. 引擎查询 `notification_rules` 表，匹配 event_type + severity_filter
//!   3. 对匹配的规则，查询关联的 `notification_channels` 记录
//!   4. 根据通道类型（email/feishu/webhook）实例化对应 Channel 并发送
//!   5. 同时通过 `create_notification()` 写站内信（保证不丢消息）

use std::sync::Arc;

use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::AsyncSmtpTransport;
use lettre::{AsyncTransport as _, Message};
use serde_json::{json, Value};
use sqlx::Row;
use tracing;

use crate::routes::AppState;

// ============================================================
// 通道配置解析
// ============================================================

/// 邮件通道配置（camelCase 与前端/DB JSON 一致）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct EmailConfig {
    smtp_host: String,
    smtp_port: u16,
    username: String,
    password: String,
    from_addr: String,
    from_name: String,
    use_tls: bool,
}

/// 飞书通道配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct FeishuConfig {
    webhook_url: String,
    secret: Option<String>,
}

/// 通用 Webhook 配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebhookConfig {
    url: String,
    headers: Option<Value>,
}

/// 从数据库 config_json 解析通道配置
fn parse_channel_config(channel_type: &str, config_str: &str) -> Result<(String, Vec<String>), String> {
    let cfg: Value = serde_json::from_str(config_str)
        .map_err(|e| format!("config_json 解析失败: {}", e))?;

    match channel_type {
        "email" => {
            let smtp_host = cfg["smtpHost"].as_str().unwrap_or("").to_string();
            let smtp_port = cfg["smtpPort"].as_u64().unwrap_or(587) as u16;
            let username = cfg["username"].as_str().unwrap_or("").to_string();
            let password = cfg["password"].as_str().unwrap_or("").to_string();
            let from_addr = cfg["fromAddr"].as_str().unwrap_or("").to_string();
            let from_name = cfg["fromName"].as_str().unwrap_or("MeridianOps").to_string();
            let use_tls = cfg["useTls"].as_bool().unwrap_or(true);

            // 默认接收人列表（规则中的 recipient_list 优先）
            let default_recipients = cfg["defaultRecipients"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            let email_cfg = EmailConfig {
                smtp_host,
                smtp_port,
                username,
                password,
                from_addr,
                from_name,
                use_tls,
            };

            // 序列化回紧凑 JSON 传递
            Ok((serde_json::to_string(&email_cfg).unwrap_or_default(), default_recipients))
        }
        "feishu" => {
            let webhook_url = cfg["webhookUrl"].as_str().unwrap_or("").to_string();
            let secret = cfg.get("secret").and_then(|v| v.as_str()).map(|s| s.to_string());
            let feishu_cfg = FeishuConfig { webhook_url, secret };
            Ok((serde_json::to_string(&feishu_cfg).unwrap_or_default(), vec![]))
        }
        "webhook" => {
            let url = cfg["url"].as_str().unwrap_or("").to_string();
            let headers = cfg.get("headers").cloned();
            let wh_cfg = WebhookConfig { url, headers };
            Ok((serde_json::to_string(&wh_cfg).unwrap_or_default(), vec![]))
        }
        _ => Err(format!("不支持的通道类型: {}", channel_type)),
    }
}

// ============================================================
// 通道发送实现
// ============================================================

/// 发送邮件（pub 供 test_channel 调用）
pub async fn send_email(config_str: &str, recipients: &[String], title: &str, content: &str) -> Result<(), String> {
    let cfg: EmailConfig = serde_json::from_str(config_str)
        .map_err(|e| format!("邮件配置解析失败: {}", e))?;

    if cfg.smtp_host.is_empty() || cfg.from_addr.is_empty() {
        return Err("邮件 SMTP 主机或发件地址未配置".to_string());
    }
    if recipients.is_empty() {
        return Err("收件人列表为空".to_string());
    }

    let email = Message::builder()
        .from(cfg.from_addr.parse().map_err(|e| format!("发件地址解析失败: {}", e))?)
        .to(recipients[0].parse().map_err(|e| format!("收件地址解析失败: {}", e))?)
        .subject(title)
        .header(ContentType::TEXT_HTML)
        .body(content.to_string())
        .map_err(|e| format!("邮件构建失败: {}", e))?;

    let transport = if cfg.use_tls {
        let mut t = AsyncSmtpTransport::<lettre::Tokio1Executor>::relay(&cfg.smtp_host)
            .map_err(|e| format!("SMTP relay 构建失败: {}", e))?
            .port(cfg.smtp_port);
        if !cfg.username.is_empty() {
            t = t.credentials(Credentials::new(cfg.username.clone(), cfg.password.clone()));
        }
        t.build()
    } else {
        let mut t = AsyncSmtpTransport::<lettre::Tokio1Executor>::builder_dangerous(&cfg.smtp_host)
            .port(cfg.smtp_port);
        if !cfg.username.is_empty() {
            t = t.credentials(Credentials::new(cfg.username.clone(), cfg.password.clone()));
        }
        t.build()
    };

    transport.send(email).await.map_err(|e| format!("邮件发送失败: {}", e))?;
    Ok(())
}

/// 发送飞书 webhook 消息（pub 供 test_channel 调用）
pub async fn send_feishu(config_str: &str, title: &str, content: &str) -> Result<(), String> {
    let cfg: FeishuConfig = serde_json::from_str(config_str)
        .map_err(|e| format!("飞书配置解析失败: {}", e))?;

    if cfg.webhook_url.is_empty() {
        return Err("飞书 webhook URL 未配置".to_string());
    }

    // 飞书自定义机器人消息格式（text 卡片）
    let payload = json!({
        "msg_type": "interactive",
        "card": {
            "header": {
                "title": { "tag": "plain_text", "content": title },
                "template": "red"
            },
            "elements": [
                {
                    "tag": "div",
                    "text": { "tag": "lark_md", "content": content }
                }
            ]
        }
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(&cfg.webhook_url)
        .json(&payload)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("飞书 webhook 请求失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("飞书 webhook 返回非 200: {}", resp.status()));
    }

    let body: Value = resp.json().await.unwrap_or(json!({}));
    if body.get("code").and_then(|c| c.as_i64()) != Some(0) {
        return Err(format!("飞书返回错误: {}", body));
    }

    Ok(())
}

/// 发送通用 webhook（pub 供 test_channel 调用）
pub async fn send_webhook(config_str: &str, title: &str, content: &str, event_type: &str) -> Result<(), String> {
    let cfg: WebhookConfig = serde_json::from_str(config_str)
        .map_err(|e| format!("Webhook 配置解析失败: {}", e))?;

    if cfg.url.is_empty() {
        return Err("Webhook URL 未配置".to_string());
    }

    let payload = json!({
        "title": title,
        "content": content,
        "eventType": event_type,
        "source": "MeridianOps",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    let client = reqwest::Client::new();
    let mut req = client
        .post(&cfg.url)
        .json(&payload)
        .timeout(std::time::Duration::from_secs(10));

    // 注入自定义 headers
    if let Some(headers) = &cfg.headers {
        if let Some(h) = headers.as_object() {
            for (k, v) in h {
                if let Some(s) = v.as_str() {
                    req = req.header(k, s);
                }
            }
        }
    }

    let resp = req.send().await.map_err(|e| format!("Webhook 请求失败: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("Webhook 返回非 2xx: {}", resp.status()));
    }

    Ok(())
}

// ============================================================
// 核心分发函数
// ============================================================

/// 事件分发入口 — 供其他模块调用。
///
/// 参数：
/// - `event_type`: 事件分类（来自字典 event_type），如 `host`/`software`/`database`/`ticket`/`job` 等
///   规则 event_type 为空时匹配所有事件分类
/// - `trigger_scene`: 触发场景，6 个生命周期之一：`alert_firing`/`alert_acknowledged`/`alert_resolved`
///   /`ticket_assigned`/`ticket_closed`/`job_failed`；规则 trigger_scene 为空时匹配所有场景
/// - `severity`: 严重程度（可选，用于规则过滤），如 `disaster`, `critical`, `warning`
/// - `title`: 通知标题
/// - `content`: 通知内容（支持简单 Markdown）
/// - `link`: 跳转链接（站内信用）
/// - `user_ids`: 需要发站内信的用户 ID 列表
/// - `host`: 可选，告警关联的设备 IP / 主机名，用于规则 host_filter 匹配
pub async fn dispatch_event(
    state: &Arc<AppState>,
    event_type: &str,
    trigger_scene: &str,
    severity: Option<&str>,
    title: &str,
    content: &str,
    link: &str,
    user_ids: &[String],
    host: Option<&str>,
) {
    // 1) 站内信：对指定用户发（站内信用 trigger_scene 作为 type，便于前端按生命周期分类展示）
    for uid in user_ids {
        crate::notification_routes::create_notification(
            &state.db,
            uid,
            trigger_scene,
            title,
            content,
            link,
        )
        .await;
    }

    // 2) 外部通道：查规则 + 通道
    let rules = match fetch_matching_rules(&state.db, event_type, trigger_scene, severity, title, host).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(target: "notification_engine", "查询通知规则失败: {}", e);
            return;
        }
    };

    if rules.is_empty() {
        return; // 无匹配规则，跳过
    }

    let now_str = chrono::Utc::now().to_rfc3339();

    for rule in &rules {
        let rule_id = &rule.id;
        let rule_name_snapshot = rule.name.clone();
        let channel_ids = &rule.channel_ids;
        let rule_recipients: Vec<String> = rule
            .recipient_list
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        for ch in &rule.channels {
            let started = std::time::Instant::now();
            let (result, actual_recipients) = match ch.channel_type.as_str() {
                "email" => {
                    let recps = if rule_recipients.is_empty() {
                        ch.default_recipients.clone()
                    } else {
                        rule_recipients.clone()
                    };
                    let r = send_email(&ch.config_str, &recps, title, content).await;
                    (r, recps)
                }
                "feishu" => (send_feishu(&ch.config_str, title, content).await, vec![]),
                "webhook" => (send_webhook(&ch.config_str, title, content, event_type).await, vec![]),
                other => (Err(format!("不支持的通道类型: {}", other)), vec![]),
            };
            let duration_ms = started.elapsed().as_millis() as u32;

            // 1. 日志记录
            let (status, error_msg, response_snippet) = match &result {
                Ok(()) => ("success".to_string(), None, None),
                Err(e) => {
                    let truncated: String = e.chars().take(4000).collect();
                    ("failed".to_string(), Some(truncated), None)
                }
            };
            let recipients_str = if actual_recipients.is_empty() {
                None
            } else {
                Some(actual_recipients.join(","))
            };
            let content_snippet: String = content.chars().take(4000).collect();
            let _ = insert_notification_log(
                &state.db,
                NotificationLogEntry {
                    id: uuid::Uuid::new_v4().to_string(),
                    rule_id: Some(rule_id.clone()),
                    rule_name: Some(rule_name_snapshot.clone()),
                    channel_id: ch.id.clone(),
                    channel_name: ch.name.clone(),
                    channel_type: ch.channel_type.clone(),
                    event_type: event_type.to_string(),
                    trigger_scene: Some(trigger_scene.to_string()),
                    severity: severity.map(|s| s.to_string()),
                    recipients: recipients_str,
                    title: title.chars().take(500).collect(),
                    content: Some(content_snippet),
                    link: if link.is_empty() { None } else { Some(link.chars().take(500).collect()) },
                    status,
                    error_msg,
                    response_snippet,
                    duration_ms: Some(duration_ms),
                    triggered_by: Some("rule".to_string()),
                    sent_at: now_str.clone(),
                },
            )
            .await
            .map_err(|e| {
                tracing::warn!(target: "notification_engine", "写通知发送日志失败: {}", e);
            });

            // 2. 运行时日志
            match result {
                Ok(()) => {
                    tracing::info!(
                        target: "notification_engine",
                        rule_id = %rule_id,
                        channel = %ch.name,
                        channel_type = %ch.channel_type,
                        duration_ms,
                        "通知发送成功"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        target: "notification_engine",
                        rule_id = %rule_id,
                        channel = %ch.name,
                        channel_type = %ch.channel_type,
                        duration_ms,
                        error = %e,
                        "通知发送失败"
                    );
                }
            }
        }
    }
}

// ============================================================
// 通知发送日志持久化
// ============================================================

#[derive(Clone)]
pub struct NotificationLogEntry {
    pub id: String,
    pub rule_id: Option<String>,
    pub rule_name: Option<String>,
    pub channel_id: String,
    pub channel_name: String,
    pub channel_type: String,
    pub event_type: String,
    pub trigger_scene: Option<String>,
    pub severity: Option<String>,
    pub recipients: Option<String>,
    pub title: String,
    pub content: Option<String>,
    pub link: Option<String>,
    pub status: String,
    pub error_msg: Option<String>,
    pub response_snippet: Option<String>,
    pub duration_ms: Option<u32>,
    pub triggered_by: Option<String>,
    pub sent_at: String,
}

async fn insert_notification_log(
    pool: &sqlx::MySqlPool,
    entry: NotificationLogEntry,
) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO notification_logs \
         (id, rule_id, rule_name, channel_id, channel_name, channel_type, event_type, trigger_scene, severity, \
          recipients, title, content, link, status, error_msg, response_snippet, duration_ms, triggered_by, sent_at) \
         VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
    )
    .bind(&entry.id)
    .bind(entry.rule_id.as_deref())
    .bind(entry.rule_name.as_deref())
    .bind(&entry.channel_id)
    .bind(&entry.channel_name)
    .bind(&entry.channel_type)
    .bind(&entry.event_type)
    .bind(entry.trigger_scene.as_deref())
    .bind(entry.severity.as_deref())
    .bind(entry.recipients.as_deref())
    .bind(&entry.title)
    .bind(entry.content.as_deref())
    .bind(entry.link.as_deref())
    .bind(&entry.status)
    .bind(entry.error_msg.as_deref())
    .bind(entry.response_snippet.as_deref())
    .bind(entry.duration_ms)
    .bind(entry.triggered_by.as_deref())
    .bind(&entry.sent_at)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|e| format!("写通知日志失败: {}", e))
}

/// 供 notification_routes / test_channel 出口处手动写一条日志（manual_test）
pub async fn log_manual_send(
    pool: &sqlx::MySqlPool,
    channel_id: &str,
    channel_name: &str,
    channel_type: &str,
    event_type: &str,
    recipients: Option<&str>,
    title: &str,
    content: &str,
    result: &Result<(), String>,
    duration_ms: u32,
    sent_at: String,
) {
    let (status, error_msg) = match result {
        Ok(()) => ("success".to_string(), None),
        Err(e) => {
            let t: String = e.chars().take(4000).collect();
            ("failed".to_string(), Some(t))
        }
    };
    let content_snippet: String = content.chars().take(4000).collect();
    let title_s: String = title.chars().take(500).collect();
    let _ = insert_notification_log(
        pool,
        NotificationLogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            rule_id: None,
            rule_name: None,
            channel_id: channel_id.to_string(),
            channel_name: channel_name.to_string(),
            channel_type: channel_type.to_string(),
            event_type: event_type.to_string(),
            trigger_scene: None,
            severity: None,
            recipients: recipients.map(|s| s.to_string()),
            title: title_s,
            content: Some(content_snippet),
            link: None,
            status,
            error_msg,
            response_snippet: None,
            duration_ms: Some(duration_ms),
            triggered_by: Some("manual_test".to_string()),
            sent_at,
        },
    )
    .await
    .map_err(|e| tracing::warn!(target: "notification_engine", "写 manual_test 通知日志失败: {}", e));
}

// ============================================================
// 数据库查询
// ============================================================

struct ChannelInfo {
    id: String,
    name: String,
    channel_type: String,
    config_str: String,
    default_recipients: Vec<String>,
}

struct RuleInfo {
    id: String,
    name: String,
    channel_ids: Vec<String>,
    recipient_list: String,
    channels: Vec<ChannelInfo>,
}

/// 把告警 severity（canonical）映射为 Zabbix 0-5 原始数值。
/// - 输入 "0".."5" → 直接返回 0..5
/// - 输入 disaster/critical/warning/info → 映射为 5/4/2/1
/// - 其他 → None
fn severity_to_num(s: &str) -> Option<i64> {
    if let Ok(n) = s.parse::<i64>() {
        return Some(n.clamp(0, 5));
    }
    let v = match s.to_lowercase().as_str() {
        "disaster" => 5,
        "critical" => 4,
        "warning" => 2,
        "info" => 1,
        "uncategorized" | "not_classified" => 0,
        _ => return None,
    };
    Some(v)
}

/// 按 severity_op 比较 severity（告警级别）与 severity_filter（规则配置）。
/// - op = in（默认）：filter 为字符串数组，任一匹配 severity 即命中；空数组 = 匹配所有
/// - op = gte / gt / lte / lt / eq：filter 为单值（字符串或 JSON 数组首元素），数值比较
/// - op = between：filter 为数组 [min, max]，闭区间匹配
/// - filter 解析失败或值为空：视为"不筛选"，命中
fn severity_match(op: &str, filter_json: &str, severity: &str) -> bool {
    let filter: serde_json::Value = match serde_json::from_str(filter_json) {
        Ok(v) => v,
        Err(_) => return true, // 解析失败 = 不筛选
    };
    if filter.is_null() {
        return true;
    }

    let op_norm = if op.is_empty() { "in" } else { op };

    match op_norm {
        "in" => {
            // filter: ["disaster","critical"]，空数组 = 匹配所有
            let arr = match filter.as_array() {
                Some(a) => a,
                None => return true,
            };
            if arr.is_empty() {
                return true;
            }
            arr.iter().any(|v| {
                v.as_str()
                    .map(|s| s.eq_ignore_ascii_case(severity))
                    .unwrap_or(false)
            })
        }
        "gte" | "gt" | "lte" | "lt" | "eq" => {
            // filter：单值（字符串，如 "critical"）；也兼容 JSON 数组取首元素
            let filter_str: String = if let Some(arr) = filter.as_array() {
                arr.first()
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            } else if let Some(s) = filter.as_str() {
                s.to_string()
            } else {
                return true;
            };
            if filter_str.is_empty() {
                return true;
            }
            let sev_n = match severity_to_num(severity) {
                Some(n) => n,
                None => return false,
            };
            let filter_n = match severity_to_num(&filter_str) {
                Some(n) => n,
                None => return false,
            };
            match op_norm {
                "gte" => sev_n >= filter_n,
                "gt" => sev_n > filter_n,
                "lte" => sev_n <= filter_n,
                "lt" => sev_n < filter_n,
                "eq" => sev_n == filter_n,
                _ => false,
            }
        }
        "between" => {
            // filter: ["warning","critical"] 或 ["2","4"]
            let arr = match filter.as_array() {
                Some(a) if a.len() >= 2 => a,
                _ => return true,
            };
            let min_str = arr.first().and_then(|v| v.as_str()).unwrap_or("");
            let max_str = arr.get(1).and_then(|v| v.as_str()).unwrap_or("");
            if min_str.is_empty() || max_str.is_empty() {
                return true;
            }
            let sev_n = match severity_to_num(severity) {
                Some(n) => n,
                None => return false,
            };
            let min_n = match severity_to_num(min_str) {
                Some(n) => n,
                None => return false,
            };
            let max_n = match severity_to_num(max_str) {
                Some(n) => n,
                None => return false,
            };
            sev_n >= min_n && sev_n <= max_n
        }
        _ => true, // 未知 op = 不筛选
    }
}

/// 查询匹配的规则 + 关联通道
///
/// 匹配维度：
/// - event_type：规则 event_type 为空（或 NULL/''）时匹配所有事件分类；否则必须相等
/// - trigger_scene：规则 trigger_scene 为空（或 NULL/''）时匹配所有触发场景；否则必须相等
/// - severity：规则 severity_filter 为空时匹配所有级别；否则必须包含当前级别
/// - host_filter / name_keyword：见下
async fn fetch_matching_rules(
    pool: &sqlx::MySqlPool,
    event_type: &str,
    trigger_scene: &str,
    severity: Option<&str>,
    title: &str,
    host: Option<&str>,
) -> Result<Vec<RuleInfo>, String> {
    // 1) 查启用的规则，按 event_type + trigger_scene 双维度过滤
    //    event_type / trigger_scene 在规则中为空（NULL 或 ''）时表示"匹配所有"
    let rows = sqlx::query(
        "SELECT id, name, channel_ids, recipient_list, severity_filter, severity_op, host_filter, name_keyword, event_type, trigger_scene \
         FROM notification_rules \
         WHERE enabled = 1 \
           AND (event_type IS NULL OR event_type = '' OR event_type = ?) \
           AND (trigger_scene IS NULL OR trigger_scene = '' OR trigger_scene = ?)",
    )
    .bind(event_type)
    .bind(trigger_scene)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("查询通知规则失败: {}", e))?;

    let mut rules: Vec<RuleInfo> = Vec::new();
    for row in &rows {
        let id: String = row.try_get("id").unwrap_or_default();
        let name: String = row.try_get("name").unwrap_or_default();
        let channel_ids_str: String = row.try_get("channel_ids").unwrap_or_else(|_| "[]".to_string());
        let recipient_list: String = row.try_get("recipient_list").unwrap_or_default();
        let severity_filter_str: String = row.try_get("severity_filter").unwrap_or_else(|_| "null".to_string());
        let severity_op: String = row.try_get::<Option<String>, _>("severity_op").ok().flatten().unwrap_or_else(|| "in".to_string());

        // 严重程度过滤（按 severity_op 分支）
        if let Some(sev) = severity {
            if !severity_match(&severity_op, &severity_filter_str, sev) {
                continue;
            }
        }

        // host_filter 过滤：空值 = 不筛；支持逗号分隔多值 + % 通配符
        if let Ok(Some(hf)) = row.try_get::<Option<String>, _>("host_filter") {
            let hf = hf.trim();
            if !hf.is_empty() {
                let host_val = host.unwrap_or("");
                let hit = hf
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .any(|pattern| {
                        if pattern.contains('%') {
                            // 简化 LIKE：%foo% → contains；%foo → ends_with；foo% → starts_with
                            let body = pattern.replace('%', "");
                            if pattern.starts_with('%') && pattern.ends_with('%') {
                                host_val.contains(&body)
                            } else if pattern.starts_with('%') {
                                host_val.ends_with(&body)
                            } else if pattern.ends_with('%') {
                                host_val.starts_with(&body)
                            } else {
                                host_val == pattern
                            }
                        } else {
                            host_val == pattern
                        }
                    });
                if !hit {
                    continue;
                }
            }
        }

        // name_keyword 过滤：title 包含关键字（大小写不敏感）；空值 = 不筛
        if let Ok(Some(kw)) = row.try_get::<Option<String>, _>("name_keyword") {
            let kw = kw.trim();
            if !kw.is_empty() && !title.to_lowercase().contains(&kw.to_lowercase()) {
                continue;
            }
        }

        // 解析 channel_ids
        let channel_ids: Vec<String> = serde_json::from_str(&channel_ids_str).unwrap_or_default();
        if channel_ids.is_empty() {
            continue;
        }

        rules.push(RuleInfo {
            id,
            name,
            channel_ids,
            recipient_list,
            channels: Vec::new(), // 后续填充
        });
    }

    // 2) 批量查所有关联通道
    for rule in &mut rules {
        if rule.channel_ids.is_empty() {
            continue;
        }
        let placeholders = vec!["?"; rule.channel_ids.len()].join(",");
        let sql = format!(
            "SELECT id, name, channel_type, config_json FROM notification_channels WHERE id IN ({}) AND enabled = 1",
            placeholders
        );
        let mut q = sqlx::query(&sql);
        for cid in &rule.channel_ids {
            q = q.bind(cid);
        }
        let ch_rows = q.fetch_all(pool).await.map_err(|e| format!("查询通知通道失败: {}", e))?;

        for ch_row in &ch_rows {
            let id: String = ch_row.try_get("id").unwrap_or_default();
            let name: String = ch_row.try_get("name").unwrap_or_default();
            let channel_type: String = ch_row.try_get("channel_type").unwrap_or_default();
            let config_json: String = ch_row.try_get("config_json").unwrap_or_else(|_| "{}".to_string());

            // 解析通道配置，提取默认接收人
            let (_, default_recipients) = parse_channel_config(&channel_type, &config_json)
                .unwrap_or((String::new(), vec![]));

            // 重新读取原始 config_json 用于发送
            rule.channels.push(ChannelInfo {
                id,
                name,
                channel_type,
                config_str: config_json,
                default_recipients,
            });
        }
    }

    Ok(rules)
}
