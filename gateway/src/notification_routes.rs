//! Notification Routes
//!  - GET    /api/notifications              分页列表（支持 ?unreadOnly=true）
//!  - GET    /api/notifications/unread-count  未读数量
//!  - POST   /api/notifications/:id/read      标记单条已读
//!  - POST   /api/notifications/read-all       全部已读
//!  - DELETE /api/notifications/:id            删除单条
//!  - GET    /api/notifications/stream         SSE 实时通知推送
//!  - GET    /api/notification-settings        获取当前用户通知设置
//!  - PUT    /api/notification-settings        更新通知设置

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

use crate::auth::AuthUser;
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

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/notifications", get(list_notifications))
        .route("/api/notifications/unread-count", get(unread_count))
        .route("/api/notifications/stream", get(stream_notifications))
        .route("/api/notifications/read-all", post(read_all))
        .route("/api/notifications/:id/read", post(mark_read))
        .route("/api/notifications/:id", delete(delete_notification))
        .route("/api/notification-settings", get(get_notification_settings).put(update_notification_settings))
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
