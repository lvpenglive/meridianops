//! Notification Routes
//!  - GET    /api/notifications              分页列表（支持 ?unreadOnly=true）
//!  - GET    /api/notifications/unread-count  未读数量
//!  - POST   /api/notifications/:id/read      标记单条已读
//!  - POST   /api/notifications/read-all       全部已读
//!  - DELETE /api/notifications/:id            删除单条
//!  - GET    /api/notifications/stream         SSE 实时通知推送
//!  - GET    /api/notification-settings        获取当前用户通知设置
//!  - PUT    /api/notification-settings        更新通知设置
//!
//!  通知引擎 — 通道管理
//!  - GET    /api/notification/channels        通道列表
//!  - POST   /api/notification/channels        新建通道
//!  - PUT    /api/notification/channels/:id    更新通道
//!  - DELETE /api/notification/channels/:id    删除通道
//!  - POST   /api/notification/channels/:id/test  测试通道连通性
//!
//!  通知引擎 — 规则管理
//!  - GET    /api/notification/rules           规则列表
//!  - POST   /api/notification/rules           新建规则
//!  - PUT    /api/notification/rules/:id       更新规则
//!  - DELETE /api/notification/rules/:id       删除规则

use std::sync::Arc;
use std::sync::OnceLock;

use axum::{
    extract::{Path, Query, State},
    response::sse::{Event, KeepAlive, Sse},
    Json, Router,
    routing::{delete, get, post, put},
};
use futures_util::Stream;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use tokio::sync::broadcast;

use crate::auth::{self, AuthUser};
use crate::error::AppError;
use crate::routes::AppState;

// ============================================================
// SSE 全局广播通道
// ============================================================

/// 全局通知广播发送端。消息格式为 (user_id, notification_json)。
fn notification_broadcast() -> &'static broadcast::Sender<(String, Value)> {
    static TX: OnceLock<broadcast::Sender<(String, Value)>> = OnceLock::new();
    TX.get_or_init(|| {
        let (tx, _rx) = broadcast::channel(1024);
        tx
    })
}

/// 通过 SSE 向指定用户推送一条通知。
/// 供其他模块调用：当有新通知产生时调用此函数，在线的用户会通过 SSE 实时收到。
pub async fn broadcast_notification(user_id: &str, notif: &Value) {
    let tx = notification_broadcast();
    let _ = tx.send((user_id.to_string(), notif.clone()));
}

// ============================================================
// 路由
// ============================================================

#[derive(Deserialize)]
struct ListQuery {
    #[serde(default)]
    unread_only: Option<bool>,
    #[serde(default = "default_page")]
    page: u32,
    #[serde(default = "default_page_size")]
    page_size: u32,
}
fn default_page() -> u32 { 1 }
fn default_page_size() -> u32 { 20 }
fn default_true() -> bool { true }

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/notifications", get(list_notifications))
        .route("/api/notifications/unread-count", get(unread_count))
        .route("/api/notifications/stream", get(stream_notifications))
        .route("/api/notifications/read-all", post(read_all))
        .route("/api/notifications/:id/read", post(mark_read))
        .route("/api/notifications/:id", delete(delete_notification))
        .route("/api/notification-settings", get(get_notification_settings).put(update_notification_settings))
        // 通知引擎 — 通道管理
        .route("/api/notification/channels", get(list_channels).post(create_channel))
        .route("/api/notification/channels/:id", axum::routing::put(update_channel).delete(delete_channel))
        .route("/api/notification/channels/:id/test", post(test_channel))
        // 通知引擎 — 规则管理
        .route("/api/notification/rules", get(list_rules).post(create_rule))
        .route("/api/notification/rules/:id", axum::routing::put(update_rule).delete(delete_rule))
        // 通知引擎 — 发送日志
        .route("/api/notification/logs", get(list_notification_logs))
        // 通知引擎 — 清理配置 + 手动触发
        .route("/api/notification/cleaner-config", get(get_cleaner_config))
        .route("/api/notification/cleaner/run", post(run_cleaner_now))
}

// ============================================================
// 创建通知（供其他模块调用）
// ============================================================

/// 插入一条通知（供其他模块调用），同时通过 SSE 推送给在线用户。
pub async fn create_notification(
    pool: &sqlx::MySqlPool,
    user_id: &str,
    ntype: &str,
    title: &str,
    content: &str,
    link: &str,
) {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let _ = sqlx::query(
        "INSERT INTO notifications (id, user_id, type, title, content, link, created_at) VALUES (?,?,?,?,?,?,NOW())"
    )
    .bind(&id)
    .bind(user_id)
    .bind(ntype)
    .bind(title)
    .bind(content)
    .bind(link)
    .execute(pool)
    .await;

    // 同时通过 SSE 广播给在线用户
    let notif = json!({
        "id": id,
        "type": ntype,
        "title": title,
        "content": content,
        "link": link,
        "isRead": false,
        "createdAt": now,
    });
    broadcast_notification(user_id, &notif).await;
}

// ============================================================
// 列表 / 未读数 / 标记已读 / 删除
// ============================================================

async fn list_notifications(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<Json<Value>, AppError> {
    let pool = &state.db;
    let me = &auth.0.uid;
    let offset = ((q.page.saturating_sub(1)) * q.page_size) as i64;
    let limit = q.page_size as i64;

    let (rows, total): (Vec<sqlx::mysql::MySqlRow>, i64) = if q.unread_only == Some(true) {
        let r = sqlx::query("SELECT id, type, title, content, link, is_read, created_at, read_at FROM notifications WHERE user_id=? AND is_read=0 ORDER BY created_at DESC LIMIT ? OFFSET ?")
            .bind(me).bind(limit).bind(offset).fetch_all(pool).await?;
        let c: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notifications WHERE user_id=? AND is_read=0")
            .bind(me).fetch_one(pool).await?;
        (r, c)
    } else {
        let r = sqlx::query("SELECT id, type, title, content, link, is_read, created_at, read_at FROM notifications WHERE user_id=? ORDER BY created_at DESC LIMIT ? OFFSET ?")
            .bind(me).bind(limit).bind(offset).fetch_all(pool).await?;
        let c: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notifications WHERE user_id=?")
            .bind(me).fetch_one(pool).await?;
        (r, c)
    };

    let list: Vec<Value> = rows.iter().map(|r| {
        json!({
            "id": r.try_get::<String, _>("id").unwrap_or_default(),
            "type": r.try_get::<String, _>("type").unwrap_or_default(),
            "title": r.try_get::<String, _>("title").unwrap_or_default(),
            "content": r.try_get::<Option<String>, _>("content").ok().flatten().unwrap_or_default(),
            "link": r.try_get::<Option<String>, _>("link").ok().flatten().unwrap_or_default(),
            "isRead": r.try_get::<bool, _>("is_read").unwrap_or(false),
            "createdAt": crate::ticket_routes::dt_str(r, "created_at"),
            "readAt": crate::ticket_routes::dt_opt(r, "read_at"),
        })
    }).collect();

    Ok(Json(json!({
        "code": 0,
        "data": { "total": total, "page": q.page, "pageSize": q.page_size, "list": list }
    })))
}

async fn unread_count(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Json<Value>, AppError> {
    let pool = &state.db;
    let c: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notifications WHERE user_id=? AND is_read=0")
        .bind(&auth.0.uid)
        .fetch_one(pool)
        .await?;
    Ok(Json(json!({ "code": 0, "data": { "count": c } })))
}

async fn mark_read(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let pool = &state.db;
    let res = sqlx::query("UPDATE notifications SET is_read=1, read_at=NOW() WHERE id=? AND user_id=?")
        .bind(&id).bind(&auth.0.uid).execute(pool).await?;
    if res.rows_affected() == 0 {
        return Err(AppError::not_found("通知不存在或无权操作"));
    }
    Ok(Json(json!({ "code": 0, "message": "ok" })))
}

async fn read_all(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Json<Value>, AppError> {
    let pool = &state.db;
    let res = sqlx::query("UPDATE notifications SET is_read=1, read_at=NOW() WHERE user_id=? AND is_read=0")
        .bind(&auth.0.uid).execute(pool).await?;
    Ok(Json(json!({ "code": 0, "message": "ok", "data": { "updated": res.rows_affected() } })))
}

async fn delete_notification(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let pool = &state.db;
    let res = sqlx::query("DELETE FROM notifications WHERE id=? AND user_id=?")
        .bind(&id).bind(&auth.0.uid).execute(pool).await?;
    if res.rows_affected() == 0 {
        return Err(AppError::not_found("通知不存在或无权操作"));
    }
    Ok(Json(json!({ "code": 0, "message": "ok" })))
}

// ============================================================
// SSE 实时通知推送
// ============================================================

/// SSE 端点：GET /api/notifications/stream
/// 客户端通过 EventSource 连接，实时接收当前用户的新通知。
/// 鉴权方式：Authorization: Bearer <token>（与其他接口一致）。
async fn stream_notifications(
    auth: AuthUser,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let user_id = auth.0.uid.clone();
    let rx = notification_broadcast().subscribe();

    let stream = futures_util::stream::unfold(rx, move |mut rx| {
        let user_id = user_id.clone();
        async move {
            loop {
                match rx.recv().await {
                    Ok((uid, notif)) if uid == user_id => {
                        let data_str = serde_json::to_string(&notif).unwrap_or_default();
                        let event = Event::default().event("notification").data(data_str);
                        return Some((Ok(event), rx));
                    }
                    Ok(_) => {
                        // 不是发给当前用户的消息，继续等待
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        // 发送端关闭，流结束
                        return None;
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        // 消息滞后（消费太慢），跳过并继续
                        continue;
                    }
                }
            }
        }
    });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(std::time::Duration::from_secs(30))
            .text(""),
    )
}

// ============================================================
// 通知设置
// ============================================================

/// 默认通知设置：所有类型都开启
fn default_notification_settings() -> Value {
    json!({
        "ticket_assigned": true,
        "ticket_approved": true,
        "ticket_rejected": true,
        "ticket_closed": true,
        "ticket_commented": true,
        "ticket_watched": true,
        "system": true,
    })
}

#[derive(Deserialize)]
struct UpdateSettingsReq {
    settings: Value,
}

/// GET /api/notification-settings - 获取当前用户通知设置
async fn get_notification_settings(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Json<Value>, AppError> {
    let pool = &state.db;
    let me = &auth.0.uid;

    let row = sqlx::query(
        "SELECT settings_json FROM user_notification_settings WHERE user_id = ? LIMIT 1"
    )
    .bind(me)
    .fetch_optional(pool)
    .await?;

    let settings = match row {
        Some(r) => {
            let settings_str: String = r.try_get("settings_json").unwrap_or_else(|_| "{}".to_string());
            serde_json::from_str::<Value>(&settings_str).unwrap_or_else(|_| default_notification_settings())
        }
        None => default_notification_settings(),
    };

    Ok(Json(json!({ "code": 0, "data": { "settings": settings } })))
}

/// PUT /api/notification-settings - 更新通知设置
async fn update_notification_settings(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(req): Json<UpdateSettingsReq>,
) -> Result<Json<Value>, AppError> {
    let pool = &state.db;
    let me = &auth.0.uid;

    let settings_json = serde_json::to_string(&req.settings).unwrap_or_else(|_| "{}".to_string());

    // 使用 INSERT ... ON DUPLICATE KEY UPDATE 进行 upsert
    sqlx::query(
        "INSERT INTO user_notification_settings (user_id, settings_json, updated_at)
         VALUES (?, ?, NOW())
         ON DUPLICATE KEY UPDATE settings_json = ?, updated_at = NOW()"
    )
    .bind(me)
    .bind(&settings_json)
    .bind(&settings_json)
    .execute(pool)
    .await?;

    Ok(Json(json!({ "code": 0, "message": "ok" })))
}

// ============================================================
// 通知引擎 — 通道管理
// ============================================================

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateChannelReq {
    name: String,
    #[serde(rename = "channelType", alias = "channel_type")]
    channel_type: String,
    config: Value,
    #[serde(default = "default_true")]
    enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListChannelsQuery {
    #[serde(default = "default_page")]
    page: u32,
    #[serde(default = "default_page_size")]
    page_size: u32,
    keyword: Option<String>,           // 名称模糊匹配
    #[serde(rename = "channelType", alias = "channel_type")]
    channel_type: Option<String>,      // email / feishu / webhook / sms_http
    enabled: Option<bool>,             // 启用 / 禁用
}

async fn list_channels(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Query(q): Query<ListChannelsQuery>,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "notification:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let page = q.page.max(1) as i64;
    let page_size = q.page_size.clamp(1, 500) as i64;
    let offset = (page - 1) * page_size;

    let mut qb = sqlx::QueryBuilder::<sqlx::MySql>::new(
        "SELECT id, name, channel_type, config_json, enabled, created_by, created_at, updated_at \
         FROM notification_channels WHERE 1=1 "
    );
    let mut cqb = sqlx::QueryBuilder::<sqlx::MySql>::new(
        "SELECT COUNT(*) AS c FROM notification_channels WHERE 1=1 "
    );

    if let Some(kw) = &q.keyword {
        if !kw.is_empty() {
            let like = format!("%{}%", kw);
            qb.push(" AND name LIKE "); qb.push_bind(like.clone());
            cqb.push(" AND name LIKE "); cqb.push_bind(like);
        }
    }
    if let Some(v) = &q.channel_type {
        if !v.is_empty() {
            qb.push(" AND channel_type = "); qb.push_bind(v);
            cqb.push(" AND channel_type = "); cqb.push_bind(v);
        }
    }
    if let Some(v) = q.enabled {
        qb.push(" AND enabled = "); qb.push_bind(v);
        cqb.push(" AND enabled = "); cqb.push_bind(v);
    }

    let total: i64 = cqb.build().fetch_one(&state.db).await
        .and_then(|r| r.try_get::<i64, _>("c"))
        .unwrap_or(0);

    qb.push(" ORDER BY created_at DESC, id DESC LIMIT ");
    qb.push_bind(page_size);
    qb.push(" OFFSET ");
    qb.push_bind(offset);

    let rows = qb.build().fetch_all(&state.db).await?;

    let list: Vec<Value> = rows.iter().map(|r| {
        let config_str: String = r.try_get("config_json").unwrap_or_else(|_| "{}".to_string());
        let config_safe = sanitize_channel_config(
            r.try_get::<String, _>("channel_type").unwrap_or_default().as_str(),
            &config_str,
        );
        json!({
            "id": r.try_get::<String, _>("id").unwrap_or_default(),
            "name": r.try_get::<String, _>("name").unwrap_or_default(),
            "channelType": r.try_get::<String, _>("channel_type").unwrap_or_default(),
            "config": config_safe,
            "enabled": r.try_get::<bool, _>("enabled").unwrap_or(true),
            "createdBy": r.try_get::<String, _>("created_by").unwrap_or_default(),
            "createdAt": crate::ticket_routes::dt_str(r, "created_at"),
            "updatedAt": crate::ticket_routes::dt_str(r, "updated_at"),
        })
    }).collect();

    Ok(Json(json!({
        "code": 0,
        "data": {
            "list": list,
            "total": total,
            "page": page,
            "pageSize": page_size,
        }
    })))
}

async fn create_channel(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(req): Json<CreateChannelReq>,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "notification:manage")?;
    crate::license_routes::require_active_license(&state.db).await?;

    if !["email", "feishu", "webhook", "sms_http"].contains(&req.channel_type.as_str()) {
        return Err(AppError::bad("不支持的通道类型，仅支持 email/feishu/webhook"));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let config_json = serde_json::to_string(&req.config).unwrap_or_else(|_| "{}".to_string());

    sqlx::query(
        "INSERT INTO notification_channels (id, name, channel_type, config_json, enabled, created_by, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&req.channel_type)
    .bind(&config_json)
    .bind(req.enabled)
    .bind(&auth.0.sub)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "code": 0, "data": { "id": id } })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateChannelReq {
    name: Option<String>,
    #[serde(rename = "channelType", alias = "channel_type")]
    channel_type: Option<String>,
    config: Option<Value>,
    #[serde(default = "default_true")]
    enabled: bool,
}

async fn update_channel(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateChannelReq>,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "notification:manage")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let now = chrono::Utc::now().to_rfc3339();
    let mut q = sqlx::QueryBuilder::<sqlx::MySql>::new("UPDATE notification_channels SET updated_at = ");
    q.push_bind(now);

    if let Some(name) = &req.name {
        q.push(", name = "); q.push_bind(name);
    }
    if let Some(ct) = &req.channel_type {
        if !["email", "feishu", "webhook", "sms_http"].contains(&ct.as_str()) {
            return Err(AppError::bad("不支持的通道类型"));
        }
        q.push(", channel_type = "); q.push_bind(ct);
    }
    if let Some(config) = &req.config {
        let config_json = serde_json::to_string(config).unwrap_or_else(|_| "{}".to_string());
        q.push(", config_json = "); q.push_bind(config_json);
    }
    q.push(", enabled = "); q.push_bind(req.enabled);
    q.push(" WHERE id = "); q.push_bind(&id);

    let result = q.build().execute(&state.db).await?;
    if result.rows_affected() == 0 {
        return Err(AppError::not_found("通道不存在"));
    }
    Ok(Json(json!({ "code": 0, "message": "ok" })))
}

async fn delete_channel(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "notification:manage")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let result = sqlx::query("DELETE FROM notification_channels WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::not_found("通道不存在"));
    }
    Ok(Json(json!({ "code": 0, "message": "ok" })))
}

/// 测试通道连通性 — 发送一条测试消息
async fn test_channel(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "notification:manage")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let row = sqlx::query(
        "SELECT name, channel_type, config_json FROM notification_channels WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::not_found("通道不存在"))?;

    let name: String = row.try_get("name").unwrap_or_default();
    let channel_type: String = row.try_get("channel_type").unwrap_or_default();
    let config_json: String = row.try_get("config_json").unwrap_or_else(|_| "{}".to_string());

    let title = "MeridianOps 通知通道测试";
    let content = format!("通道「{}」测试消息：如果你收到了这条消息，说明通道配置正确。", name);

    let started = std::time::Instant::now();
    let (result, recipients) = match channel_type.as_str() {
        "email" => {
            let config: Value = serde_json::from_str(&config_json).unwrap_or(json!({}));
            let recipients = config["defaultRecipients"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect::<Vec<_>>())
                .unwrap_or_default();
            let r = crate::notification_engine::send_email(&config_json, &recipients, title, &content).await;
            (r, Some(recipients.join(",")))
        }
        "feishu" => (crate::notification_engine::send_feishu(&config_json, title, &content).await, None),
        "webhook" => (crate::notification_engine::send_webhook(&config_json, title, &content, "test").await, None),
        "sms_http" => {
            // 测试用一个占位手机号 13800138000
            let test_mobile = "13800138000".to_string();
            let (success, failures) = crate::notification_engine::send_sms_http(
                &config_json,
                &[test_mobile.clone()],
                title,
                &content,
                "127.0.0.1",
                "test",
                "test",
            ).await;
            if success.is_empty() {
                let err = failures.first()
                    .map(|(m, e)| format!("{}: {}", m, e))
                    .unwrap_or_else(|| "未知错误".to_string());
                (Err(err), Some(test_mobile))
            } else {
                (Ok(()), Some(test_mobile))
            }
        }
        other => (Err(format!("不支持的通道类型: {}", other)), None),
    };
    let duration_ms = started.elapsed().as_millis() as u32;
    let sent_at = chrono::Utc::now().to_rfc3339();

    // 写日志（不阻塞响应，即使日志表还没建也不影响测试返回）
    let channel_id = id.clone();
    let name_clone = name.clone();
    let ct_clone = channel_type.clone();
    let recps_snapshot = recipients.clone();
    let title_owned = title.to_string();
    let content_owned = content.clone();
    let result_copy = result.clone().map_err(|e| e.to_string());
    let pool = state.db.clone();
    tokio::spawn(async move {
        crate::notification_engine::log_manual_send(
            &pool,
            &channel_id,
            &name_clone,
            &ct_clone,
            "test",
            recps_snapshot.as_deref(),
            &title_owned,
            &content_owned,
            &result_copy,
            duration_ms,
            sent_at,
        )
        .await;
    });

    match result {
        Ok(()) => Ok(Json(json!({ "code": 0, "message": "测试消息发送成功", "durationMs": duration_ms }))),
        Err(e) => Ok(Json(json!({ "code": 500, "message": format!("发送失败: {}", e), "durationMs": duration_ms }))),
    }
}

// ============================================================
// 通知引擎 — 规则管理
// ============================================================

#[derive(Deserialize)]
struct CreateRuleReq {
    name: String,
    event_type: Option<String>,
    #[serde(default)]
    trigger_scene: Option<String>,
    #[serde(default)]
    severity_filter: Option<Value>,
    #[serde(default)]
    severity_op: Option<String>,
    #[serde(default)]
    host_filter: Option<String>,
    #[serde(default)]
    name_keyword: Option<String>,
    channel_ids: Value,
    #[serde(default)]
    recipient_list: Option<String>,
    #[serde(default = "default_true")]
    enabled: bool,
}

#[derive(Debug, Deserialize)]
struct ListRulesQuery {
    #[serde(default = "default_page")]
    page: u32,
    #[serde(default = "default_page_size")]
    page_size: u32,
    keyword: Option<String>,           // 规则名称模糊匹配
    event_type: Option<String>,        // host / software / database / ...（来自字典）
    trigger_scene: Option<String>,     // alert_firing / ticket_assigned / ...
    enabled: Option<bool>,             // 启用 / 禁用
}

async fn list_rules(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Query(q): Query<ListRulesQuery>,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "notification:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let page = q.page.max(1) as i64;
    let page_size = q.page_size.clamp(1, 500) as i64;
    let offset = (page - 1) * page_size;

    let mut qb = sqlx::QueryBuilder::<sqlx::MySql>::new(
        "SELECT id, name, event_type, trigger_scene, severity_filter, severity_op, host_filter, name_keyword, channel_ids, recipient_list, enabled, created_by, created_at, updated_at \
         FROM notification_rules WHERE 1=1 "
    );
    let mut cqb = sqlx::QueryBuilder::<sqlx::MySql>::new(
        "SELECT COUNT(*) AS c FROM notification_rules WHERE 1=1 "
    );

    if let Some(kw) = &q.keyword {
        if !kw.is_empty() {
            let like = format!("%{}%", kw);
            qb.push(" AND name LIKE "); qb.push_bind(like.clone());
            cqb.push(" AND name LIKE "); cqb.push_bind(like);
        }
    }
    if let Some(v) = &q.event_type {
        if !v.is_empty() {
            qb.push(" AND event_type = "); qb.push_bind(v);
            cqb.push(" AND event_type = "); cqb.push_bind(v);
        }
    }
    if let Some(v) = &q.trigger_scene {
        if !v.is_empty() {
            qb.push(" AND trigger_scene = "); qb.push_bind(v);
            cqb.push(" AND trigger_scene = "); cqb.push_bind(v);
        }
    }
    if let Some(v) = q.enabled {
        qb.push(" AND enabled = "); qb.push_bind(v);
        cqb.push(" AND enabled = "); cqb.push_bind(v);
    }

    let total: i64 = cqb.build().fetch_one(&state.db).await
        .and_then(|r| r.try_get::<i64, _>("c"))
        .unwrap_or(0);

    qb.push(" ORDER BY created_at DESC, id DESC LIMIT ");
    qb.push_bind(page_size);
    qb.push(" OFFSET ");
    qb.push_bind(offset);

    let rows = qb.build().fetch_all(&state.db).await?;

    let list: Vec<Value> = rows.iter().map(|r| {
        let sf_str: String = r.try_get("severity_filter").unwrap_or_else(|_| "null".to_string());
        let ci_str: String = r.try_get("channel_ids").unwrap_or_else(|_| "[]".to_string());
        json!({
            "id": r.try_get::<String, _>("id").unwrap_or_default(),
            "name": r.try_get::<String, _>("name").unwrap_or_default(),
            "eventType": r.try_get::<Option<String>, _>("event_type").ok().flatten().unwrap_or_default(),
            "triggerScene": r.try_get::<Option<String>, _>("trigger_scene").ok().flatten().unwrap_or_default(),
            "severityFilter": serde_json::from_str::<Value>(&sf_str).unwrap_or(Value::Null),
            "severityOp": r.try_get::<Option<String>, _>("severity_op").ok().flatten().unwrap_or_else(|| "in".to_string()),
            "hostFilter": r.try_get::<Option<String>, _>("host_filter").ok().flatten().unwrap_or_default(),
            "nameKeyword": r.try_get::<Option<String>, _>("name_keyword").ok().flatten().unwrap_or_default(),
            "channelIds": serde_json::from_str::<Value>(&ci_str).unwrap_or(json!([])),
            "recipientList": r.try_get::<Option<String>, _>("recipient_list").ok().flatten().unwrap_or_default(),
            "enabled": r.try_get::<bool, _>("enabled").unwrap_or(true),
            "createdBy": r.try_get::<String, _>("created_by").unwrap_or_default(),
            "createdAt": crate::ticket_routes::dt_str(r, "created_at"),
            "updatedAt": crate::ticket_routes::dt_str(r, "updated_at"),
        })
    }).collect();

    Ok(Json(json!({
        "code": 0,
        "data": {
            "list": list,
            "total": total,
            "page": page,
            "pageSize": page_size,
        }
    })))
}

async fn create_rule(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(req): Json<CreateRuleReq>,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "notification:manage")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let event_type = req.event_type.as_deref().unwrap_or("").trim().to_string();
    let trigger_scene = req.trigger_scene.as_deref().unwrap_or("").trim().to_string();
    let severity_json = req.severity_filter
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "null".to_string()))
        .unwrap_or_else(|| "null".to_string());
    let channel_ids_json = serde_json::to_string(&req.channel_ids).unwrap_or_else(|_| "[]".to_string());
    let recipients = req.recipient_list.unwrap_or_default();
    let severity_op = req.severity_op.as_deref().unwrap_or("in").trim().to_string();

    sqlx::query(
        "INSERT INTO notification_rules (id, name, event_type, trigger_scene, severity_filter, severity_op, host_filter, name_keyword, channel_ids, recipient_list, enabled, created_by, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&event_type)
    .bind(&trigger_scene)
    .bind(&severity_json)
    .bind(&severity_op)
    .bind(&req.host_filter)
    .bind(&req.name_keyword)
    .bind(&channel_ids_json)
    .bind(&recipients)
    .bind(req.enabled)
    .bind(&auth.0.sub)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "code": 0, "data": { "id": id } })))
}

#[derive(Deserialize)]
struct UpdateRuleReq {
    name: Option<String>,
    event_type: Option<String>,
    #[serde(default)]
    trigger_scene: Option<String>,
    #[serde(default)]
    severity_filter: Option<Value>,
    #[serde(default)]
    severity_op: Option<String>,
    #[serde(default)]
    host_filter: Option<String>,
    #[serde(default)]
    name_keyword: Option<String>,
    #[serde(default)]
    channel_ids: Option<Value>,
    #[serde(default)]
    recipient_list: Option<String>,
    #[serde(default = "default_true")]
    enabled: bool,
}

async fn update_rule(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateRuleReq>,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "notification:manage")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let now = chrono::Utc::now().to_rfc3339();
    let mut q = sqlx::QueryBuilder::<sqlx::MySql>::new("UPDATE notification_rules SET updated_at = ? ");
    q.push_bind(now);

    if let Some(name) = &req.name {
        q.push(", name = "); q.push_bind(name);
    }
    if let Some(et) = &req.event_type {
        q.push(", event_type = "); q.push_bind(et);
    }
    if let Some(ts) = &req.trigger_scene {
        q.push(", trigger_scene = "); q.push_bind(ts);
    }
    if let Some(sf) = &req.severity_filter {
        let sf_str = serde_json::to_string(sf).unwrap_or_else(|_| "null".to_string());
        q.push(", severity_filter = "); q.push_bind(sf_str);
    }
    if let Some(sop) = &req.severity_op {
        q.push(", severity_op = "); q.push_bind(sop);
    }
    if let Some(hf) = &req.host_filter {
        q.push(", host_filter = "); q.push_bind(hf);
    }
    if let Some(nk) = &req.name_keyword {
        q.push(", name_keyword = "); q.push_bind(nk);
    }
    if let Some(ci) = &req.channel_ids {
        let ci_str = serde_json::to_string(ci).unwrap_or_else(|_| "[]".to_string());
        q.push(", channel_ids = "); q.push_bind(ci_str);
    }
    if let Some(rl) = &req.recipient_list {
        q.push(", recipient_list = "); q.push_bind(rl);
    }
    q.push(", enabled = "); q.push_bind(req.enabled);
    q.push(" WHERE id = "); q.push_bind(&id);

    let result = q.build().execute(&state.db).await?;
    if result.rows_affected() == 0 {
        return Err(AppError::not_found("规则不存在"));
    }
    Ok(Json(json!({ "code": 0, "message": "ok" })))
}

async fn delete_rule(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "notification:manage")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let result = sqlx::query("DELETE FROM notification_rules WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::not_found("规则不存在"));
    }
    Ok(Json(json!({ "code": 0, "message": "ok" })))
}

/// 通道配置脱敏（不返回明文密码/secret/token）
fn sanitize_channel_config(channel_type: &str, config_str: &str) -> Value {
    let mut cfg: Value = serde_json::from_str(config_str).unwrap_or(json!({}));
    match channel_type {
        "email" => {
            if let Some(obj) = cfg.as_object_mut() {
                if obj.contains_key("password") {
                    obj.insert("password".to_string(), Value::String("******".to_string()));
                }
            }
        }
        "feishu" => {
            if let Some(obj) = cfg.as_object_mut() {
                if obj.contains_key("secret") {
                    obj.insert("secret".to_string(), Value::String("******".to_string()));
                }
            }
        }
        _ => {}
    }
    cfg
}

// ============================================================
// 通知引擎 — 发送日志
// ============================================================

fn default_log_page() -> u32 { 1 }
fn default_log_page_size() -> u32 { 50 }

#[derive(Debug, Deserialize)]
struct ListLogsQuery {
    #[serde(default = "default_log_page")]
    page: u32,
    #[serde(default = "default_log_page_size")]
    page_size: u32,
    status: Option<String>,              // success / failed
    channel_type: Option<String>,        // email / feishu / webhook
    event_type: Option<String>,          // alert_firing / test / ...
    channel_id: Option<String>,          // 按通道过滤
    triggered_by: Option<String>,        // rule / manual_test
    keyword: Option<String>,             // 模糊匹配 title / recipients / error_msg
    sent_since: Option<String>,          // >= YYYY-MM-DD 或 RFC3339
    sent_until: Option<String>,          // <= 同上
}

async fn list_notification_logs(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Query(q): Query<ListLogsQuery>,
) -> Result<Json<Value>, AppError> {
    // 兼容：没有专门 notification_log:read 权限时，只要有 notification:read 也允许（老用户无新权限点）
    let can_log = auth.has_permission("notification_log:read");
    let can_read = auth.has_permission("notification:read");
    if !can_log && !can_read {
        return Err(AppError::forbidden("缺少查看通知发送日志权限"));
    }
    crate::license_routes::require_active_license(&state.db).await?;

    let page = q.page.max(1) as i64;
    let page_size = q.page_size.clamp(1, 500) as i64;
    let offset = (page - 1) * page_size;

    // 使用 QueryBuilder 动态拼接 WHERE
    let mut qb = sqlx::QueryBuilder::<sqlx::MySql>::new(
        "SELECT id, rule_id, rule_name, channel_id, channel_name, channel_type, event_type, severity, \
                recipients, title, content, link, status, error_msg, response_snippet, duration_ms, triggered_by, sent_at \
         FROM notification_logs WHERE 1=1 "
    );

    if let Some(v) = &q.status {
        qb.push(" AND status = "); qb.push_bind(v);
    }
    if let Some(v) = &q.channel_type {
        qb.push(" AND channel_type = "); qb.push_bind(v);
    }
    if let Some(v) = &q.event_type {
        qb.push(" AND event_type = "); qb.push_bind(v);
    }
    if let Some(v) = &q.channel_id {
        qb.push(" AND channel_id = "); qb.push_bind(v);
    }
    if let Some(v) = &q.triggered_by {
        qb.push(" AND triggered_by = "); qb.push_bind(v);
    }
    if let Some(v) = &q.sent_since {
        qb.push(" AND sent_at >= "); qb.push_bind(v);
    }
    if let Some(v) = &q.sent_until {
        qb.push(" AND sent_at <= "); qb.push_bind(v);
    }
    if let Some(kw) = &q.keyword {
        if !kw.is_empty() {
            let like1 = format!("%{}%", kw);
            let like2 = format!("%{}%", kw);
            let like3 = format!("%{}%", kw);
            qb.push(" AND (title LIKE ? OR recipients LIKE ? OR error_msg LIKE ?)");
            qb.push_bind(like1);
            qb.push_bind(like2);
            qb.push_bind(like3);
        }
    }

    // 计数
    let mut count_qb = sqlx::QueryBuilder::<sqlx::MySql>::new("SELECT COUNT(*) AS c FROM notification_logs WHERE 1=1 ");
    if let Some(v) = &q.status {
        count_qb.push(" AND status = "); count_qb.push_bind(v);
    }
    if let Some(v) = &q.channel_type {
        count_qb.push(" AND channel_type = "); count_qb.push_bind(v);
    }
    if let Some(v) = &q.event_type {
        count_qb.push(" AND event_type = "); count_qb.push_bind(v);
    }
    if let Some(v) = &q.channel_id {
        count_qb.push(" AND channel_id = "); count_qb.push_bind(v);
    }
    if let Some(v) = &q.triggered_by {
        count_qb.push(" AND triggered_by = "); count_qb.push_bind(v);
    }
    if let Some(v) = &q.sent_since {
        count_qb.push(" AND sent_at >= "); count_qb.push_bind(v);
    }
    if let Some(v) = &q.sent_until {
        count_qb.push(" AND sent_at <= "); count_qb.push_bind(v);
    }
    if let Some(kw) = &q.keyword {
        if !kw.is_empty() {
            let like1 = format!("%{}%", kw);
            let like2 = format!("%{}%", kw);
            let like3 = format!("%{}%", kw);
            count_qb.push(" AND (title LIKE ? OR recipients LIKE ? OR error_msg LIKE ?)");
            count_qb.push_bind(like1);
            count_qb.push_bind(like2);
            count_qb.push_bind(like3);
        }
    }
    let total: i64 = count_qb
        .build()
        .fetch_one(&state.db)
        .await
        .and_then(|row| row.try_get::<i64, _>("c"))
        .unwrap_or(0);

    // 列表（排序：最新优先）
    qb.push(" ORDER BY sent_at DESC, id DESC LIMIT ");
    qb.push_bind(page_size);
    qb.push(" OFFSET ");
    qb.push_bind(offset);
    let rows = qb.build().fetch_all(&state.db).await?;

    let list: Vec<Value> = rows.iter().map(|r| {
        json!({
            "id": r.try_get::<String, _>("id").unwrap_or_default(),
            "ruleId": r.try_get::<Option<String>, _>("rule_id").ok().flatten(),
            "ruleName": r.try_get::<Option<String>, _>("rule_name").ok().flatten(),
            "channelId": r.try_get::<String, _>("channel_id").unwrap_or_default(),
            "channelName": r.try_get::<String, _>("channel_name").unwrap_or_default(),
            "channelType": r.try_get::<String, _>("channel_type").unwrap_or_default(),
            "eventType": r.try_get::<String, _>("event_type").unwrap_or_default(),
            "severity": r.try_get::<Option<String>, _>("severity").ok().flatten(),
            "recipients": r.try_get::<Option<String>, _>("recipients").ok().flatten(),
            "title": r.try_get::<String, _>("title").unwrap_or_default(),
            "content": r.try_get::<Option<String>, _>("content").ok().flatten(),
            "link": r.try_get::<Option<String>, _>("link").ok().flatten(),
            "status": r.try_get::<String, _>("status").unwrap_or_default(),
            "errorMsg": r.try_get::<Option<String>, _>("error_msg").ok().flatten(),
            "responseSnippet": r.try_get::<Option<String>, _>("response_snippet").ok().flatten(),
            "durationMs": r.try_get::<Option<u32>, _>("duration_ms").ok().flatten(),
            "triggeredBy": r.try_get::<Option<String>, _>("triggered_by").ok().flatten(),
            "sentAt": r.try_get::<String, _>("sent_at").unwrap_or_default(),
        })
    }).collect();

    Ok(Json(json!({
        "code": 0,
        "data": {
            "list": list,
            "total": total,
            "page": page,
            "pageSize": page_size,
        }
    })))
}

// ============================================================
// 通知发送日志清理：配置查看 + 手动触发
// ============================================================

#[derive(Debug, Deserialize)]
struct RunCleanerReq {
    /// 可选：自定义本次保留天数（不填则用配置值）
    retention_days: Option<u32>,
    /// 可选：自定义本次批大小
    batch_size: Option<u32>,
}

async fn get_cleaner_config(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Json<Value>, AppError> {
    let can_log = auth.has_permission("notification_log:read");
    let can_read = auth.has_permission("notification:read");
    if !can_log && !can_read {
        return Err(AppError::forbidden("缺少查看通知发送日志权限"));
    }
    let cfg = &state.config.notification_cleaner;
    Ok(Json(json!({
        "code": 0,
        "data": {
            "enabled": cfg.enabled,
            "retentionDays": cfg.retention_days,
            "intervalSecs": cfg.interval_secs,
            "batchSize": cfg.batch_size,
        }
    })))
}

async fn run_cleaner_now(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(req): Json<RunCleanerReq>,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "notification:manage")?;
    crate::license_routes::require_active_license(&state.db).await?;

    // 构造本次清理配置：用户指定优先，否则继承全局
    let effective_cfg = crate::config::NotificationCleanerConfig {
        enabled: true,
        retention_days: req.retention_days.unwrap_or(state.config.notification_cleaner.retention_days),
        interval_secs: state.config.notification_cleaner.interval_secs,
        batch_size: req.batch_size.unwrap_or(state.config.notification_cleaner.batch_size),
    };

    let result = crate::notification_cleaner::run_cleanup_once(&state.db, &effective_cfg)
        .await
        .map_err(|e| AppError::internal(&format!("清理执行失败: {}", e)))?;

    Ok(Json(json!({
        "code": 0,
        "message": format!("已删除 {} 行（{} 批，耗时 {}ms）", result.deleted, result.batches, result.duration_ms),
        "data": {
            "retentionDays": result.retention_days,
            "cutoff": result.cutoff,
            "deleted": result.deleted,
            "batches": result.batches,
            "durationMs": result.duration_ms,
        }
    })))
}
