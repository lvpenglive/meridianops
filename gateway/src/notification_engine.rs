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

/// 短信平台（HTTP）配置 — 对接外部短信网关，支持 JSON/XML 报文
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SmsHttpConfig {
    /// 短信平台 URL，如 http://sms-gw/api/send
    url: String,
    /// HTTP 方法，默认 POST
    method: Option<String>,
    /// 报文格式：json | xml
    content_type: Option<String>,
    /// 自定义请求头
    headers: Option<Value>,
    /// 签名（可选，注入到 ${signName} 变量）
    sign_name: Option<String>,
    /// 模板 ID（可选，注入到 ${templateId} 变量）
    template_id: Option<String>,
    /// 报文模板，支持变量替换：${mobile} ${title} ${content} ${host}
    /// ${severity} ${eventType} ${timestamp} ${signName} ${templateId}
    body_template: String,
    /// 成功响应匹配模式（可选）：若提供，则在响应文本中查找此子串表示成功
    success_pattern: Option<String>,
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
        "sms_http" => {
            let url = cfg["url"].as_str().unwrap_or("").to_string();
            let method = cfg.get("method").and_then(|v| v.as_str()).map(|s| s.to_string());
            let content_type = cfg.get("contentType").and_then(|v| v.as_str()).map(|s| s.to_string());
            let headers = cfg.get("headers").cloned();
            let sign_name = cfg.get("signName").and_then(|v| v.as_str()).map(|s| s.to_string());
            let template_id = cfg.get("templateId").and_then(|v| v.as_str()).map(|s| s.to_string());
            let body_template = cfg["bodyTemplate"].as_str().unwrap_or("").to_string();
            let success_pattern = cfg.get("successPattern").and_then(|v| v.as_str()).map(|s| s.to_string());
            let sms_cfg = SmsHttpConfig {
                url, method, content_type, headers, sign_name, template_id,
                body_template, success_pattern,
            };
            Ok((serde_json::to_string(&sms_cfg).unwrap_or_default(), vec![]))
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

/// 发送短信平台 HTTP 报文（pub 供 test_channel 调用）
///
/// 对每个手机号做一次独立 HTTP 调用，支持 JSON / XML 两种报文格式。
/// 返回 (成功手机号列表, 失败手机号 + 错误信息列表)。
pub async fn send_sms_http(
    config_str: &str,
    mobiles: &[String],
    title: &str,
    content: &str,
    host: &str,
    severity: &str,
    event_type: &str,
) -> (Vec<String>, Vec<(String, String)>) {
    let cfg: SmsHttpConfig = match serde_json::from_str(config_str) {
        Ok(c) => c,
        Err(e) => {
            let err = format!("短信配置解析失败: {}", e);
            return (
                vec![],
                mobiles.iter().map(|m| (m.clone(), err.clone())).collect(),
            );
        }
    };

    if cfg.url.is_empty() {
        let err = "短信平台 URL 未配置".to_string();
        return (
            vec![],
            mobiles.iter().map(|m| (m.clone(), err.clone())).collect(),
        );
    }

    let method = cfg.method.as_deref().unwrap_or("POST").to_uppercase();
    let content_type = cfg.content_type.as_deref().unwrap_or("json").to_lowercase();
    let sign_name = cfg.sign_name.clone().unwrap_or_default();
    let template_id = cfg.template_id.clone().unwrap_or_default();
    let timestamp = chrono::Utc::now().to_rfc3339();

    let client = reqwest::Client::new();
    let mut success_mobiles: Vec<String> = Vec::new();
    let mut failures: Vec<(String, String)> = Vec::new();

    for mobile in mobiles {
        // 变量替换
        let body = cfg.body_template
            .replace("${mobile}", mobile)
            .replace("${title}", title)
            .replace("${content}", content)
            .replace("${host}", host)
            .replace("${severity}", severity)
            .replace("${eventType}", event_type)
            .replace("${timestamp}", &timestamp)
            .replace("${signName}", &sign_name)
            .replace("${templateId}", &template_id);

        let req_builder = match method.as_str() {
            "GET" => client.get(&cfg.url),
            _ => client.post(&cfg.url),
        };

        // 按 contentType 设置 body 和 Content-Type
        let req_builder = if content_type == "xml" {
            req_builder
                .header("Content-Type", "application/xml; charset=utf-8")
                .body(body)
        } else {
            // json：尝试解析为 Value 再用 .json()，失败则当文本发
            match serde_json::from_str::<Value>(&body) {
                Ok(v) => req_builder.json(&v),
                Err(_) => req_builder
                    .header("Content-Type", "application/json; charset=utf-8")
                    .body(body),
            }
        };

        // 注入自定义 headers
        let req_builder = if let Some(headers) = &cfg.headers {
            if let Some(h) = headers.as_object() {
                let mut rb = req_builder;
                for (k, v) in h {
                    if let Some(s) = v.as_str() {
                        rb = rb.header(k, s);
                    }
                }
                rb
            } else {
                req_builder
            }
        } else {
            req_builder
        };

        match req_builder
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                let body_text = resp.text().await.unwrap_or_default();
                if !status.is_success() {
                    failures.push((
                        mobile.clone(),
                        format!("HTTP {} : {}", status, body_text.chars().take(500).collect::<String>()),
                    ));
                    continue;
                }
                // success_pattern 校验
                if let Some(pat) = &cfg.success_pattern {
                    if pat.is_empty() || !body_text.contains(pat.as_str()) {
                        failures.push((
                            mobile.clone(),
                            format!("响应未匹配成功标识 [{}]: {}", pat, body_text.chars().take(500).collect::<String>()),
                        ));
                        continue;
                    }
                }
                success_mobiles.push(mobile.clone());
            }
            Err(e) => {
                failures.push((mobile.clone(), format!("请求失败: {}", e)));
            }
        }
    }

    (success_mobiles, failures)
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

    // 2) 通知策略：按勾选渠道分发（站内信 / 短信 / 邮件 / 飞书 / Webhook）
    dispatch_sms_strategies(
        state,
        event_type,
        trigger_scene,
        severity,
        title,
        content,
        link,
        host,
        user_ids,
    )
    .await;

    // 告警触发已由通知策略覆盖，避免与旧 notification_rules 重复发送。
    // 认领 / 恢复 / 工单 / 作业 / 日志突增仍走规则，兼容现网已配飞书规则。
    if trigger_scene == "alert_firing" {
        return;
    }

    let rules = match fetch_matching_rules(&state.db, event_type, trigger_scene, severity, title, host).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(target: "notification_engine", "查询通知规则失败: {}", e);
            return;
        }
    };

    if rules.is_empty() {
        return; // 无匹配通知规则，跳过（不影响短信策略）
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
                    alert_id: None,
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
    pub alert_id: Option<String>,
    pub status: String,
    pub error_msg: Option<String>,
    pub response_snippet: Option<String>,
    pub duration_ms: Option<u32>,
    pub triggered_by: Option<String>,
    pub sent_at: String,
}

fn extract_alert_id_from_link(link: &str) -> Option<String> {
    let idx = link.find("id=")?;
    let rest = &link[idx + 3..];
    let id = rest.split('&').next()?.trim();
    if id.len() >= 8 && id.len() <= 64 {
        Some(id.to_string())
    } else {
        None
    }
}

async fn insert_notification_log(
    pool: &sqlx::MySqlPool,
    entry: NotificationLogEntry,
) -> Result<(), String> {
    let alert_id = entry
        .alert_id
        .clone()
        .or_else(|| entry.link.as_deref().and_then(extract_alert_id_from_link));
    sqlx::query(
        "INSERT INTO notification_logs \
         (id, rule_id, rule_name, channel_id, channel_name, channel_type, event_type, trigger_scene, severity, \
          recipients, title, content, link, alert_id, status, error_msg, response_snippet, duration_ms, triggered_by, sent_at) \
         VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
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
    .bind(alert_id.as_deref())
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
            alert_id: None,
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

// ============================================================
// 告警短信策略（对齐老系统「短信策略」）
// 匹配维度：事件类型（一级 + 二级子类）→ 触发级别（等于/大于等于…）→ 设备IP → 事件名称
// 命中后给「已选人员」发站内信 + 邮件（若该用户配置了邮箱）。
// ============================================================

/// 短信策略行（匹配用）
struct SmsStrategyRow {
    recipient_user_ids: Vec<String>,
    event_type: String,
    event_sub_type: String,
    trigger_op: String,
    severity_filter: String,
    host_filter: String,
    name_keyword: String,
}

/// 事件二级子类 → 关键字列表（用于从告警标题推断子类，大小写不敏感）
fn sub_type_keywords(sub_type: &str) -> &'static [&'static str] {
    match sub_type.to_lowercase().as_str() {
        "db2"        => &["db2"],
        "oracle"     => &["oracle"],
        "sequoiadb"  => &["sequoia", "sequoiadb", "巨杉"],
        "informix"   => &["informix"],
        "sybase"     => &["sybase"],
        "sqlserver"  => &["sqlserver", "sql server", "mssql"],
        "gbase"      => &["gbase"],
        "was"        => &["websphere", "was"],
        "cics"       => &["cics"],
        "mq"         => &["mq", "ibmmq", "rabbitmq", "activemq"],
        "tomcat"     => &["tomcat"],
        _            => &[],
    }
}

/// 从告警标题/事件名推断二级子类（返回字典 value，无匹配返回 None）
fn infer_sub_type(title: &str, event_type: &str) -> Option<String> {
    let t = title.to_lowercase();
    // 仅对 database / middleware 大类推断子类，避免误命中
    let candidates: &[&str] = match event_type {
        "database" => &["db2", "oracle", "sequoiadb", "informix", "sybase", "sqlserver", "gbase"],
        "middleware" => &["was", "cics", "mq", "tomcat"],
        _ => return None,
    };
    for c in candidates {
        for kw in sub_type_keywords(c) {
            if t.contains(kw) {
                return Some(c.to_string());
            }
        }
    }
    None
}

/// 短信策略级别 → 数值（老系统六级递进：信息=0 < 一级=1 < … < 五级=5）
/// 与告警侧 severity_for_rule_match 输出的 0-5 数字尺度对齐比较。
fn sms_severity_to_num(s: &str) -> Option<i64> {
    match s.trim().to_lowercase().as_str() {
        "info" => Some(0),
        "1" => Some(1),
        "2" => Some(2),
        "3" => Some(3),
        "4" => Some(4),
        "5" => Some(5),
        _ => None,
    }
}

/// 按 trigger_op 比较告警级别与策略级别（复用 severity_to_num 的 0-5 数值语义）
fn sms_severity_match(op: &str, filter: &str, severity: &str) -> bool {
    let f = filter.trim();
    if f.is_empty() {
        return true; // 空 = 全部级别
    }
    // 告警侧级别：0-5 数字（severity_for_rule_match 输出），兼容 info 等历史值
    let sev_n = match severity_to_num(severity) {
        Some(n) => n,
        None => return false,
    };
    let filter_n = match sms_severity_to_num(f) {
        Some(n) => n,
        None => return false,
    };
    match op {
        "eq"  => sev_n == filter_n,
        "gte" => sev_n >= filter_n,
        "gt"  => sev_n > filter_n,
        "lte" => sev_n <= filter_n,
        "lt"  => sev_n < filter_n,
        _     => sev_n == filter_n,
    }
}

/// host_filter 匹配（逗号多值 + % 通配，与规则 host_filter 逻辑一致）
fn sms_host_match(host_filter: &str, host: &str) -> bool {
    let hf = host_filter.trim();
    if hf.is_empty() {
        return true;
    }
    let host_val = host.trim();
    hf.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .any(|pattern| {
            if pattern.contains('%') {
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
        })
}

/// 查询并匹配启用的通知策略，按勾选渠道分发。
/// `owner_user_ids` 为告警关联资产责任人；仅当命中策略打开 notify_owner 时并入收件人。
async fn dispatch_sms_strategies(
    state: &Arc<AppState>,
    event_type: &str,
    trigger_scene: &str,
    severity: Option<&str>,
    title: &str,
    content: &str,
    link: &str,
    host: Option<&str>,
    owner_user_ids: &[String],
) {
    tracing::info!(
        target: "notification_engine",
        "dispatch_sms_strategies called: event_type={}, scene={}, severity={:?}, title={}, host={:?}, owners={}",
        event_type, trigger_scene, severity, title, host, owner_user_ids.len()
    );
    let rows = match sqlx::query(
        "SELECT recipient_user_ids, notify_owner, channel_kinds, extra_channel_ids, trigger_scene, \
                event_type, event_sub_type, trigger_op, severity_filter, host_filter, name_keyword \
         FROM alert_sms_strategies WHERE enabled = 1",
    )
    .fetch_all(&state.db)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(target: "notification_engine", "查询通知策略失败: {}", e);
            return;
        }
    };

    let host_val = host.unwrap_or("");
    let sev_val = severity.unwrap_or("");
    let title_lower = title.to_lowercase();
    let actual_sub_type = infer_sub_type(title, event_type).unwrap_or_default();

    let mut inbox_users: Vec<String> = Vec::new();
    let mut sms_users: Vec<String> = Vec::new();
    let mut email_users: Vec<String> = Vec::new();
    let mut feishu_ids: Vec<String> = Vec::new();
    let mut webhook_ids: Vec<String> = Vec::new();
    let mut feishu_need_default = false;
    let mut webhook_need_default = false;
    let mut matched = 0usize;

    for row in &rows {
        let rids_val: serde_json::Value = row
            .try_get::<serde_json::Value, _>("recipient_user_ids")
            .unwrap_or(serde_json::Value::Array(vec![]));
        let ids: Vec<String> = serde_json::from_value(rids_val).unwrap_or_default();
        let notify_owner: bool = row.try_get("notify_owner").unwrap_or(false);
        if ids.is_empty() && !notify_owner {
            continue;
        }

        let s_event: String = row.try_get("event_type").unwrap_or_default();
        let s_sub: String = row.try_get("event_sub_type").unwrap_or_default();
        let s_op: String = row.try_get("trigger_op").unwrap_or_else(|_| "eq".to_string());
        let s_sev: String = row.try_get("severity_filter").unwrap_or_default();
        let s_host: String = row.try_get("host_filter").unwrap_or_default();
        let s_name: String = row.try_get("name_keyword").unwrap_or_default();
        let s_scene: String = row
            .try_get::<Option<String>, _>("trigger_scene")
            .ok()
            .flatten()
            .unwrap_or_default();

        if !s_scene.is_empty() && s_scene != trigger_scene {
            continue;
        }
        if !s_event.is_empty() && s_event != event_type {
            continue;
        }
        if !s_sub.is_empty() {
            if actual_sub_type.is_empty() || s_sub != actual_sub_type {
                continue;
            }
        }
        if !sms_severity_match(&s_op, &s_sev, sev_val) {
            continue;
        }
        if !sms_host_match(&s_host, host_val) {
            continue;
        }
        if !s_name.trim().is_empty() && !title_lower.contains(&s_name.trim().to_lowercase()) {
            continue;
        }

        let mut users = ids;
        if notify_owner {
            for uid in owner_user_ids {
                if !uid.is_empty() && !users.contains(uid) {
                    users.push(uid.clone());
                }
            }
        }
        if users.is_empty() {
            continue;
        }

        let kinds = normalize_strategy_kinds(parse_json_str_list(row, "channel_kinds"));
        let extras = parse_json_str_list(row, "extra_channel_ids");
        matched += 1;

        if kind_on(&kinds, "inbox") {
            merge_unique(&mut inbox_users, &users);
        }
        if kind_on(&kinds, "sms") {
            merge_unique(&mut sms_users, &users);
        }
        if kind_on(&kinds, "email") {
            merge_unique(&mut email_users, &users);
        }
        if kind_on(&kinds, "feishu") {
            if extras.is_empty() {
                feishu_need_default = true;
            } else {
                merge_unique(&mut feishu_ids, &extras);
            }
        }
        if kind_on(&kinds, "webhook") {
            if extras.is_empty() {
                webhook_need_default = true;
            } else {
                merge_unique(&mut webhook_ids, &extras);
            }
        }
    }

    if matched == 0 {
        tracing::info!(target: "notification_engine", "dispatch_sms_strategies: no strategy matched, rows={}", rows.len());
        return;
    }
    tracing::info!(
        target: "notification_engine",
        "dispatch_sms_strategies: matched {} strategies inbox={} sms={} email={} feishu={} webhook={}",
        matched, inbox_users.len(), sms_users.len(), email_users.len(), feishu_ids.len() + feishu_need_default as usize, webhook_ids.len() + webhook_need_default as usize
    );

    for uid in &inbox_users {
        crate::notification_routes::create_notification(
            &state.db,
            uid,
            "sms_strategy",
            title,
            content,
            link,
        )
        .await;
    }

    if !sms_users.is_empty() {
        send_strategy_sms(state, &sms_users, event_type, trigger_scene, severity, title, content, link, host).await;
    }
    if !email_users.is_empty() {
        send_strategy_email(state, &email_users, event_type, trigger_scene, severity, title, content, link).await;
    }

    let extra_channels = load_enabled_channels_by_ids(&state.db, &{
        let mut ids = feishu_ids.clone();
        ids.extend(webhook_ids.iter().cloned());
        ids
    })
    .await;

    let mut feishu_chs: Vec<ChannelSnap> = extra_channels
        .iter()
        .filter(|c| c.channel_type == "feishu" && feishu_ids.contains(&c.id))
        .cloned()
        .collect();
    if feishu_need_default {
        if let Some(ch) = first_enabled_channel(&state.db, "feishu").await {
            if !feishu_chs.iter().any(|c| c.id == ch.id) {
                feishu_chs.push(ch);
            }
        } else if feishu_chs.is_empty() {
            log_strategy_skip(state, event_type, trigger_scene, severity, title, content, link, "feishu", "未配置启用的飞书通道").await;
        }
    }
    for ch in &feishu_chs {
        send_strategy_named_channel(state, ch, "feishu", event_type, trigger_scene, severity, title, content, link).await;
    }

    let mut webhook_chs: Vec<ChannelSnap> = extra_channels
        .iter()
        .filter(|c| c.channel_type == "webhook" && webhook_ids.contains(&c.id))
        .cloned()
        .collect();
    if webhook_need_default {
        if let Some(ch) = first_enabled_channel(&state.db, "webhook").await {
            if !webhook_chs.iter().any(|c| c.id == ch.id) {
                webhook_chs.push(ch);
            }
        } else if webhook_chs.is_empty() {
            log_strategy_skip(state, event_type, trigger_scene, severity, title, content, link, "webhook", "未配置启用的 Webhook 通道").await;
        }
    }
    for ch in &webhook_chs {
        send_strategy_named_channel(state, ch, "webhook", event_type, trigger_scene, severity, title, content, link).await;
    }
}

fn parse_json_str_list(row: &sqlx::mysql::MySqlRow, col: &str) -> Vec<String> {
    row.try_get::<serde_json::Value, _>(col)
        .ok()
        .and_then(|v| match v {
            serde_json::Value::Array(arr) => Some(
                arr.into_iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect(),
            ),
            serde_json::Value::String(s) => serde_json::from_str(&s).ok(),
            _ => None,
        })
        .unwrap_or_default()
}

fn normalize_strategy_kinds(raw: Vec<String>) -> Vec<String> {
    let allowed = ["inbox", "sms", "email", "feishu", "webhook"];
    let mut out = Vec::new();
    for k in raw {
        let k = k.trim().to_lowercase();
        if allowed.contains(&k.as_str()) && !out.contains(&k) {
            out.push(k);
        }
    }
    if out.is_empty() {
        vec!["inbox".into(), "sms".into(), "email".into()]
    } else {
        out
    }
}

fn kind_on(kinds: &[String], k: &str) -> bool {
    kinds.iter().any(|x| x == k)
}

fn merge_unique(dst: &mut Vec<String>, src: &[String]) {
    for s in src {
        if !s.is_empty() && !dst.contains(s) {
            dst.push(s.clone());
        }
    }
}

#[derive(Clone)]
struct ChannelSnap {
    id: String,
    name: String,
    channel_type: String,
    config_str: String,
}

async fn first_enabled_channel(pool: &sqlx::MySqlPool, ty: &str) -> Option<ChannelSnap> {
    let row = sqlx::query(
        "SELECT id, name, channel_type, config_json FROM notification_channels \
         WHERE channel_type = ? AND enabled = 1 ORDER BY created_at ASC LIMIT 1",
    )
    .bind(ty)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()?;
    Some(ChannelSnap {
        id: row.try_get("id").unwrap_or_default(),
        name: row.try_get("name").unwrap_or_default(),
        channel_type: row.try_get("channel_type").unwrap_or_else(|_| ty.to_string()),
        config_str: row.try_get("config_json").unwrap_or_else(|_| "{}".to_string()),
    })
}

async fn load_enabled_channels_by_ids(pool: &sqlx::MySqlPool, ids: &[String]) -> Vec<ChannelSnap> {
    if ids.is_empty() {
        return vec![];
    }
    let placeholders = vec!["?"; ids.len()].join(",");
    let sql = format!(
        "SELECT id, name, channel_type, config_json FROM notification_channels \
         WHERE enabled = 1 AND id IN ({})",
        placeholders
    );
    let mut q = sqlx::query(&sql);
    for id in ids {
        q = q.bind(id);
    }
    let rows = match q.fetch_all(pool).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(target: "notification_engine", "按 ID 查询通知通道失败: {}", e);
            return vec![];
        }
    };
    rows.iter()
        .map(|r| ChannelSnap {
            id: r.try_get("id").unwrap_or_default(),
            name: r.try_get("name").unwrap_or_default(),
            channel_type: r.try_get("channel_type").unwrap_or_default(),
            config_str: r.try_get("config_json").unwrap_or_else(|_| "{}".to_string()),
        })
        .collect()
}

async fn send_strategy_sms(
    state: &Arc<AppState>,
    user_ids: &[String],
    event_type: &str,
    trigger_scene: &str,
    severity: Option<&str>,
    title: &str,
    content: &str,
    link: &str,
    host: Option<&str>,
) {
    let Some(ch) = first_enabled_channel(&state.db, "sms_http").await else {
        log_strategy_skip(state, event_type, trigger_scene, severity, title, content, link, "sms_http", "未配置启用的短信 HTTP 通道").await;
        return;
    };
    let mobiles = match fetch_user_mobiles(&state.db, user_ids).await {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(target: "notification_engine", "查询命中人员手机号失败: {}", e);
            vec![]
        }
    };
    if mobiles.is_empty() {
        log_strategy_skip(state, event_type, trigger_scene, severity, title, content, link, "sms_http", "命中人员无手机号，短信未发送").await;
        return;
    }
    let started = std::time::Instant::now();
    let (success_mobiles, failures) = send_sms_http(
        &ch.config_str,
        &mobiles,
        title,
        content,
        host.unwrap_or(""),
        severity.unwrap_or(""),
        event_type,
    )
    .await;
    let duration_ms = started.elapsed().as_millis() as u32;
    let error_msg: Option<String> = if failures.is_empty() {
        None
    } else {
        Some(
            failures
                .iter()
                .map(|(m, e)| format!("{}: {}", m, e))
                .collect::<Vec<_>>()
                .join("; ")
                .chars()
                .take(4000)
                .collect(),
        )
    };
    let status = if failures.is_empty() {
        "success"
    } else if success_mobiles.is_empty() {
        "failed"
    } else {
        "partial"
    };
    let _ = insert_notification_log(
        &state.db,
        NotificationLogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            rule_id: None,
            rule_name: Some("通知策略".to_string()),
            channel_id: ch.id,
            channel_name: ch.name,
            channel_type: "sms_http".to_string(),
            event_type: event_type.to_string(),
            trigger_scene: Some(trigger_scene.to_string()),
            severity: severity.map(|s| s.to_string()),
            recipients: Some(mobiles.join(",")),
            title: title.chars().take(500).collect(),
            content: Some(content.chars().take(4000).collect()),
            link: if link.is_empty() { None } else { Some(link.chars().take(500).collect()) },
            alert_id: None,
            status: status.to_string(),
            error_msg,
            response_snippet: Some(format!("成功 {} / 失败 {}", success_mobiles.len(), failures.len())),
            duration_ms: Some(duration_ms),
            triggered_by: Some("sms_strategy".to_string()),
            sent_at: chrono::Utc::now().to_rfc3339(),
        },
    )
    .await;
}

async fn send_strategy_email(
    state: &Arc<AppState>,
    user_ids: &[String],
    event_type: &str,
    trigger_scene: &str,
    severity: Option<&str>,
    title: &str,
    content: &str,
    link: &str,
) {
    let emails = match fetch_user_emails(&state.db, user_ids).await {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!(target: "notification_engine", "查询命中人员邮箱失败: {}", e);
            return;
        }
    };
    if emails.is_empty() {
        return;
    }
    let Some(ch) = first_enabled_channel(&state.db, "email").await else {
        return;
    };
    let started = std::time::Instant::now();
    let result = send_email(&ch.config_str, &emails, title, content).await;
    let duration_ms = started.elapsed().as_millis() as u32;
    let (status, error_msg) = match &result {
        Ok(()) => ("success".to_string(), None),
        Err(e) => ("failed".to_string(), Some(e.chars().take(4000).collect())),
    };
    let _ = insert_notification_log(
        &state.db,
        NotificationLogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            rule_id: None,
            rule_name: Some("通知策略".to_string()),
            channel_id: ch.id,
            channel_name: ch.name,
            channel_type: "email".to_string(),
            event_type: event_type.to_string(),
            trigger_scene: Some(trigger_scene.to_string()),
            severity: severity.map(|s| s.to_string()),
            recipients: Some(emails.join(",")),
            title: title.chars().take(500).collect(),
            content: Some(content.chars().take(4000).collect()),
            link: if link.is_empty() { None } else { Some(link.chars().take(500).collect()) },
            alert_id: None,
            status,
            error_msg,
            response_snippet: None,
            duration_ms: Some(duration_ms),
            triggered_by: Some("sms_strategy".to_string()),
            sent_at: chrono::Utc::now().to_rfc3339(),
        },
    )
    .await;
}

async fn send_strategy_named_channel(
    state: &Arc<AppState>,
    ch: &ChannelSnap,
    expected_type: &str,
    event_type: &str,
    trigger_scene: &str,
    severity: Option<&str>,
    title: &str,
    content: &str,
    link: &str,
) {
    if ch.channel_type != expected_type {
        return;
    }
    let started = std::time::Instant::now();
    let result = match ch.channel_type.as_str() {
        "feishu" => send_feishu(&ch.config_str, title, content).await,
        "webhook" => send_webhook(&ch.config_str, title, content, event_type).await,
        _ => return,
    };
    let duration_ms = started.elapsed().as_millis() as u32;
    let (status, error_msg) = match &result {
        Ok(()) => ("success".to_string(), None),
        Err(e) => ("failed".to_string(), Some(e.chars().take(4000).collect())),
    };
    let _ = insert_notification_log(
        &state.db,
        NotificationLogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            rule_id: None,
            rule_name: Some("通知策略".to_string()),
            channel_id: ch.id.clone(),
            channel_name: ch.name.clone(),
            channel_type: ch.channel_type.clone(),
            event_type: event_type.to_string(),
            trigger_scene: Some(trigger_scene.to_string()),
            severity: severity.map(|s| s.to_string()),
            recipients: None,
            title: title.chars().take(500).collect(),
            content: Some(content.chars().take(4000).collect()),
            link: if link.is_empty() { None } else { Some(link.chars().take(500).collect()) },
            alert_id: None,
            status,
            error_msg,
            response_snippet: None,
            duration_ms: Some(duration_ms),
            triggered_by: Some("sms_strategy".to_string()),
            sent_at: chrono::Utc::now().to_rfc3339(),
        },
    )
    .await;
}

async fn log_strategy_skip(
    state: &Arc<AppState>,
    event_type: &str,
    trigger_scene: &str,
    severity: Option<&str>,
    title: &str,
    content: &str,
    link: &str,
    channel_type: &str,
    reason: &str,
) {
    let _ = insert_notification_log(
        &state.db,
        NotificationLogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            rule_id: None,
            rule_name: Some("通知策略".to_string()),
            channel_id: "".to_string(),
            channel_name: "通知策略".to_string(),
            channel_type: channel_type.to_string(),
            event_type: event_type.to_string(),
            trigger_scene: Some(trigger_scene.to_string()),
            severity: severity.map(|s| s.to_string()),
            recipients: None,
            title: title.chars().take(500).collect(),
            content: Some(content.chars().take(4000).collect()),
            link: if link.is_empty() { None } else { Some(link.chars().take(500).collect()) },
            alert_id: None,
            status: "skipped".to_string(),
            error_msg: Some(reason.to_string()),
            response_snippet: None,
            duration_ms: None,
            triggered_by: Some("sms_strategy".to_string()),
            sent_at: chrono::Utc::now().to_rfc3339(),
        },
    )
    .await;
}

/// 批量查询用户的手机号（id → mobile），仅保留非空手机号（去重）
async fn fetch_user_mobiles(
    pool: &sqlx::MySqlPool,
    user_ids: &[String],
) -> Result<Vec<String>, String> {
    if user_ids.is_empty() {
        return Ok(vec![]);
    }
    let placeholders = vec!["?"; user_ids.len()].join(",");
    let sql = format!(
        "SELECT id, mobile FROM users WHERE employment_status = 'active' AND id IN ({})",
        placeholders
    );
    let mut q = sqlx::query(&sql);
    for uid in user_ids {
        q = q.bind(uid);
    }
    let rows = q.fetch_all(pool).await.map_err(|e| e.to_string())?;
    let mut mobiles: Vec<String> = Vec::new();
    for r in &rows {
        let mobile: String = r.try_get("mobile").unwrap_or_default();
        let m = mobile.trim().to_string();
        if !m.is_empty() && !mobiles.contains(&m) {
            mobiles.push(m);
        }
    }
    Ok(mobiles)
}

/// 批量查询用户的 email（id → email），仅保留非空邮箱
async fn fetch_user_emails(
    pool: &sqlx::MySqlPool,
    user_ids: &[String],
) -> Result<Vec<String>, String> {
    if user_ids.is_empty() {
        return Ok(vec![]);
    }
    let placeholders = vec!["?"; user_ids.len()].join(",");
    let sql = format!("SELECT id, email FROM users WHERE id IN ({})", placeholders);
    let mut q = sqlx::query(&sql);
    for uid in user_ids {
        q = q.bind(uid);
    }
    let rows = q.fetch_all(pool).await.map_err(|e| e.to_string())?;
    let mut emails: Vec<String> = Vec::new();
    for r in &rows {
        let email: String = r.try_get("email").unwrap_or_default();
        let e = email.trim().to_string();
        if !e.is_empty() && !emails.contains(&e) {
            emails.push(e);
        }
    }
    Ok(emails)
}
