//! 知识库路由。
//!
//! 路由：
//!   GET    /api/knowledge                    分页查询知识条目（支持搜索/分类/标签筛选） (knowledge:read)
//!   GET    /api/knowledge/:id                获取知识详情（view_count +1）            (knowledge:read)
//!   POST   /api/knowledge                    创建知识条目                             (knowledge:create)
//!   PUT    /api/knowledge/:id                更新知识条目（自动保存版本历史）          (knowledge:update)
//!   DELETE /api/knowledge/:id                删除知识条目                             (knowledge:delete)
//!   GET    /api/knowledge/search             全文检索                                 (knowledge:read)
//!   GET    /api/knowledge/categories         列出所有分类及条目数                     (knowledge:read)
//!   GET    /api/knowledge/tags               列出所有标签及条目数                     (knowledge:read)
//!   GET    /api/knowledge/:id/versions       查看版本历史                             (knowledge:read)
//!   POST   /api/knowledge/:id/helpful        标记有帮助（helpful_count +1）           (knowledge:read)
//!
//! AI 扩展预留：embedding / embedding_model / embedding_updated_at 字段已建表但当前不写入，
//! Phase 7 可在创建/更新时自动生成向量嵌入，用于语义检索和告警推荐。

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::OnceLock;

use axum::extract::{ConnectInfo, Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use jieba_rs::Jieba as JiebaRs;
use serde::Deserialize;
use sqlx::Row;
use uuid::Uuid;

use crate::audit;
use crate::auth;
use crate::db::{self, DbPool};
use crate::error::AppError;
use crate::routes::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/knowledge", get(list_knowledge).post(create_knowledge))
        .route(
            "/api/knowledge/search",
            get(search_knowledge),
        )
        .route("/api/knowledge/categories", get(list_categories))
        .route("/api/knowledge/tags", get(list_tags))
        .route(
            "/api/knowledge/:id",
            get(get_knowledge).put(update_knowledge).delete(delete_knowledge),
        )
        .route("/api/knowledge/:id/versions", get(list_versions))
        .route("/api/knowledge/:id/helpful", post(mark_helpful))
}

// ---- 请求/响应结构 ----

#[derive(Debug, Deserialize)]
struct ListQuery {
    #[serde(default)]
    page: u32,
    #[serde(default = "default_page_size")]
    page_size: u32,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    tag: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    q: Option<String>,
}

fn default_page_size() -> u32 {
    10
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    q: String,
    #[serde(default = "default_page_size")]
    page_size: u32,
}

#[derive(Debug, Deserialize)]
struct CreateRequest {
    title: String,
    category: String,
    tags: Vec<String>,
    content: String,
    summary: Option<String>,
    #[serde(default = "default_status")]
    status: String,
}

fn default_status() -> String {
    "published".to_string()
}

#[derive(Debug, Deserialize)]
struct UpdateRequest {
    title: String,
    category: String,
    tags: Vec<String>,
    content: String,
    summary: Option<String>,
    #[serde(default = "default_status")]
    status: String,
}

/// jieba 分词器全局实例（首次调用时初始化，约 100ms 加载词典）。
static JIEBA: OnceLock<JiebaRs> = OnceLock::new();

fn jieba() -> &'static JiebaRs {
    JIEBA.get_or_init(JiebaRs::new)
}

/// 中文分词：用 jieba 切词，空格连接。
/// 英文/数字保持原样（jieba 不切英文）。
fn segment_chinese(text: &str) -> String {
    jieba().cut(text, true).join(" ")
}

/// Markdown 转纯文本：去除标记符号，保留文字内容，再用 jieba 分词。
/// 用于全文检索和未来 embedding 生成。
fn markdown_to_text(md: &str) -> String {
    let plain = md.lines()
        .map(|line| {
            let trimmed = line.trim();
            // 去除标题标记
            let s = trimmed.trim_start_matches('#').trim();
            // 去除列表标记
            let s = s.trim_start_matches("- ").trim_start_matches("* ").trim_start_matches("1. ");
            // 去除代码块标记
            let s = s.trim_start_matches("```");
            // 去除行内代码反引号
            let s = s.replace('`', "");
            // 去除链接 [text](url) → text
            let s = regex_replace_link(&s);
            s
        })
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();
    // jieba 中文分词，空格分隔词元
    segment_chinese(&plain)
}

/// 简单替换 [text](url) → text
fn regex_replace_link(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '[' {
            let mut text = String::new();
            let mut found_close = false;
            let mut rest = String::new();
            for rc in chars.by_ref() {
                if rc == ']' {
                    found_close = true;
                    break;
                }
                if rc == '(' {
                    // 不合法，回退
                    text.push('(');
                }
                text.push(rc);
            }
            if found_close {
                // 跳过 (url)
                if chars.peek() == Some(&'(') {
                    chars.next();
                    for rc in chars.by_ref() {
                        if rc == ')' {
                            break;
                        }
                    }
                }
                result.push_str(&text);
            } else {
                result.push('[');
                result.push_str(&text);
                result.push_str(&rest);
            }
        } else {
            result.push(c);
        }
    }
    result
}

// ---- Handlers ----

async fn list_knowledge(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Query(params): Query<ListQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "knowledge:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let page = params.page.max(1);
    let page_size = params.page_size.min(100).max(1);
    let offset = (page - 1) * page_size;

    // 构建动态查询
    let mut where_clauses: Vec<String> = vec![];
    let mut bind_values: Vec<String> = vec![];

    if let Some(cat) = &params.category {
        if cat != "all" {
            where_clauses.push("category = ?".to_string());
            bind_values.push(cat.clone());
        }
    }
    if let Some(st) = &params.status {
        where_clauses.push("status = ?".to_string());
        bind_values.push(st.clone());
    } else {
        where_clauses.push("status = 'published'".to_string());
    }
    if let Some(q) = &params.q {
        if !q.is_empty() {
            where_clauses.push("(title LIKE ? OR content_text LIKE ?)".to_string());
            bind_values.push(format!("%{}%", q));
            bind_values.push(format!("%{}%", q));
        }
    }
    if let Some(tag) = &params.tag {
        if !tag.is_empty() {
            where_clauses.push("JSON_CONTAINS(tags, JSON_QUOTE(?))".to_string());
            bind_values.push(tag.clone());
        }
    }

    let where_sql = if where_clauses.is_empty() {
        "1=1".to_string()
    } else {
        where_clauses.join(" AND ")
    };

    // 查询总数
    let count_sql = format!("SELECT COUNT(*) as cnt FROM knowledge_items WHERE {}", where_sql);
    let mut count_q = sqlx::query_scalar::<_, i64>(&count_sql);
    for v in &bind_values {
        count_q = count_q.bind(v);
    }
    let total: i64 = count_q.fetch_one(&state.db).await?;

    // 查询列表
    let list_sql = format!(
        "SELECT id, title, category, tags, summary, status, view_count, helpful_count, version, \
         created_by_name, created_at, updated_at \
         FROM knowledge_items WHERE {} ORDER BY updated_at DESC LIMIT ? OFFSET ?",
        where_sql
    );
    let mut list_q = sqlx::query(&list_sql);
    for v in &bind_values {
        list_q = list_q.bind(v);
    }
    list_q = list_q.bind(page_size).bind(offset);
    let rows = list_q.fetch_all(&state.db).await?;

    let items: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            let tags_str: String = row.try_get("tags").unwrap_or_else(|_| "[]".to_string());
            let tags: serde_json::Value = serde_json::from_str(&tags_str).unwrap_or(serde_json::json!([]));
            serde_json::json!({
                "id": row.try_get::<String, _>("id").unwrap_or_default(),
                "title": row.try_get::<String, _>("title").unwrap_or_default(),
                "category": row.try_get::<String, _>("category").unwrap_or_default(),
                "tags": tags,
                "summary": row.try_get::<Option<String>, _>("summary").unwrap_or(None),
                "status": row.try_get::<String, _>("status").unwrap_or_default(),
                "viewCount": row.try_get::<i64, _>("view_count").unwrap_or(0),
                "helpfulCount": row.try_get::<i64, _>("helpful_count").unwrap_or(0),
                "version": row.try_get::<i64, _>("version").unwrap_or(1),
                "createdByName": row.try_get::<Option<String>, _>("created_by_name").unwrap_or(None),
                "createdAt": row.try_get::<String, _>("created_at").unwrap_or_default(),
                "updatedAt": row.try_get::<String, _>("updated_at").unwrap_or_default(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": {
            "items": items,
            "total": total,
            "page": page,
            "pageSize": page_size,
        }
    })))
}

async fn get_knowledge(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "knowledge:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let row = sqlx::query(
        "SELECT id, title, category, tags, content, summary, status, view_count, helpful_count, \
         version, created_by, created_by_name, created_at, updated_at \
         FROM knowledge_items WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::not_found("知识条目不存在"))?;

    let tags_str: String = row.try_get("tags").unwrap_or_else(|_| "[]".to_string());
    let tags: serde_json::Value = serde_json::from_str(&tags_str).unwrap_or(serde_json::json!([]));

    // 浏览数 +1
    let _ = sqlx::query("UPDATE knowledge_items SET view_count = view_count + 1 WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await;

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": {
            "id": row.try_get::<String, _>("id").unwrap_or_default(),
            "title": row.try_get::<String, _>("title").unwrap_or_default(),
            "category": row.try_get::<String, _>("category").unwrap_or_default(),
            "tags": tags,
            "content": row.try_get::<String, _>("content").unwrap_or_default(),
            "summary": row.try_get::<Option<String>, _>("summary").unwrap_or(None),
            "status": row.try_get::<String, _>("status").unwrap_or_default(),
            "viewCount": row.try_get::<i64, _>("view_count").unwrap_or(0) + 1,
            "helpfulCount": row.try_get::<i64, _>("helpful_count").unwrap_or(0),
            "version": row.try_get::<i64, _>("version").unwrap_or(1),
            "createdBy": row.try_get::<String, _>("created_by").unwrap_or_default(),
            "createdByName": row.try_get::<Option<String>, _>("created_by_name").unwrap_or(None),
            "createdAt": row.try_get::<String, _>("created_at").unwrap_or_default(),
            "updatedAt": row.try_get::<String, _>("updated_at").unwrap_or_default(),
        }
    })))
}

async fn create_knowledge(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    Json(req): Json<CreateRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "knowledge:create")?;
    crate::license_routes::require_active_license(&state.db).await?;

    if req.title.trim().is_empty() {
        return Err(AppError::bad("标题不能为空"));
    }
    if req.content.trim().is_empty() {
        return Err(AppError::bad("内容不能为空"));
    }

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let tags_json = serde_json::to_string(&req.tags).unwrap_or_else(|_| "[]".to_string());
    let content_text = markdown_to_text(&req.content);
    let summary = req.summary.unwrap_or_else(|| {
        // 自动生成摘要：取前 200 字符
        let t = &content_text;
        if t.len() > 200 {
            format!("{}...", &t[..200])
        } else {
            t.clone()
        }
    });

    sqlx::query(
        "INSERT INTO knowledge_items \
         (id, title, category, tags, content, content_text, summary, status, version, \
         created_by, created_by_name, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, 1, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&req.title)
    .bind(&req.category)
    .bind(&tags_json)
    .bind(&req.content)
    .bind(&content_text)
    .bind(&summary)
    .bind(&req.status)
    .bind(&auth.0.uid)
    .bind(&auth.0.sub)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await?;

    let detail = serde_json::json!({
        "title": req.title,
        "category": req.category,
    });
    let ip = audit::extract_ip(&headers, Some(addr));
    audit::log_async(
        &state.db,
        &auth,
        "create_knowledge",
        "knowledge_items",
        &id,
        Some(&detail),
        &ip,
        "success",
    )
    .await;

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": { "id": id }
    })))
}

async fn update_knowledge(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<UpdateRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "knowledge:update")?;
    crate::license_routes::require_active_license(&state.db).await?;

    if req.title.trim().is_empty() {
        return Err(AppError::bad("标题不能为空"));
    }
    if req.content.trim().is_empty() {
        return Err(AppError::bad("内容不能为空"));
    }

    // 查询当前版本
    let current = sqlx::query("SELECT version, title, content, tags FROM knowledge_items WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::not_found("知识条目不存在"))?;

    let old_version: i64 = current.try_get("version").unwrap_or(1);
    let old_title: String = current.try_get("title").unwrap_or_default();
    let old_content: String = current.try_get("content").unwrap_or_default();
    let old_tags: String = current.try_get("tags").unwrap_or_else(|_| "[]".to_string());
    let new_version = old_version + 1;
    let now = chrono::Utc::now().to_rfc3339();
    let tags_json = serde_json::to_string(&req.tags).unwrap_or_else(|_| "[]".to_string());
    let content_text = markdown_to_text(&req.content);

    // 保存旧版本到版本历史
    let ver_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO knowledge_versions (id, knowledge_id, version, title, content, tags, edited_by, edited_by_name, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&ver_id)
    .bind(&id)
    .bind(old_version)
    .bind(&old_title)
    .bind(&old_content)
    .bind(&old_tags)
    .bind(&auth.0.uid)
    .bind(&auth.0.sub)
    .bind(&now)
    .execute(&state.db)
    .await?;

    // 更新知识条目
    sqlx::query(
        "UPDATE knowledge_items SET title = ?, category = ?, tags = ?, content = ?, content_text = ?, \
         summary = COALESCE(?, summary), status = ?, version = ?, updated_at = ?, \
         embedding = NULL, embedding_model = NULL, embedding_updated_at = NULL \
         WHERE id = ?",
    )
    .bind(&req.title)
    .bind(&req.category)
    .bind(&tags_json)
    .bind(&req.content)
    .bind(&content_text)
    .bind(&req.summary)
    .bind(&req.status)
    .bind(new_version)
    .bind(&now)
    .bind(&id)
    .execute(&state.db)
    .await?;

    let detail = serde_json::json!({
        "title": req.title,
        "version": new_version,
    });
    let ip = audit::extract_ip(&headers, Some(addr));
    audit::log_async(
        &state.db,
        &auth,
        "update_knowledge",
        "knowledge_items",
        &id,
        Some(&detail),
        &ip,
        "success",
    )
    .await;

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": { "id": id, "version": new_version }
    })))
}

async fn delete_knowledge(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "knowledge:delete")?;
    crate::license_routes::require_active_license(&state.db).await?;

    // 删除版本历史
    sqlx::query("DELETE FROM knowledge_versions WHERE knowledge_id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;

    sqlx::query("DELETE FROM knowledge_items WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;

    let detail = serde_json::json!({ "id": id });
    let ip = audit::extract_ip(&headers, Some(addr));
    audit::log_async(
        &state.db,
        &auth,
        "delete_knowledge",
        "knowledge_items",
        &id,
        Some(&detail),
        &ip,
        "success",
    )
    .await;

    Ok(Json(serde_json::json!({ "code": 0, "data": null })))
}

async fn search_knowledge(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Query(params): Query<SearchQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "knowledge:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let q = params.q.trim();
    if q.is_empty() {
        return Ok(Json(serde_json::json!({
            "code": 0,
            "data": { "items": [], "total": 0 }
        })));
    }

    let page_size = params.page_size.min(50).max(1);

    // title 用 ngram FULLTEXT，content_text 用 default parser FULLTEXT（jieba 预分词）
    // 搜索查询也需 jieba 分词，使 default parser 能按词匹配
    let segmented_q = segment_chinese(q);
    // content_text LIKE 兜底：分词后空格替换为 % 通配符，
    // 使 '%密码%错误%' 能匹配分词后的 "密码 错误"（解决 innodb_ft_min_token_size=3 导致 2 字词不索引的问题）
    let like_content_q = format!("%{}%", segmented_q.replace(' ', "%"));

    // 使用 MySQL FULLTEXT 检索（需 MySQL 5.7+），fallback 到 LIKE
    let rows = sqlx::query(
        "SELECT id, title, category, tags, summary, view_count, helpful_count, created_at, updated_at \
         FROM knowledge_items \
         WHERE status = 'published' \
         AND (MATCH(title) AGAINST(? IN BOOLEAN MODE) OR MATCH(content_text) AGAINST(? IN BOOLEAN MODE) OR title LIKE ? OR content_text LIKE ?) \
         ORDER BY CASE WHEN MATCH(title) AGAINST(? IN BOOLEAN MODE) OR MATCH(content_text) AGAINST(? IN BOOLEAN MODE) THEN 0 ELSE 1 END, \
                  updated_at DESC \
         LIMIT ?",
    )
    .bind(q)              // title MATCH（ngram 自动切分）
    .bind(&segmented_q)   // content_text MATCH（jieba 分词后按空格匹配）
    .bind(format!("%{}%", q))       // title LIKE 兜底（title 未分词，用原始查询）
    .bind(&like_content_q)          // content_text LIKE 兜底（分词后空格→%通配）
    .bind(q)              // ORDER BY title MATCH
    .bind(&segmented_q)   // ORDER BY content_text MATCH
    .bind(page_size)
    .fetch_all(&state.db)
    .await?;

    let items: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            let tags_str: String = row.try_get("tags").unwrap_or_else(|_| "[]".to_string());
            let tags: serde_json::Value =
                serde_json::from_str(&tags_str).unwrap_or(serde_json::json!([]));
            serde_json::json!({
                "id": row.try_get::<String, _>("id").unwrap_or_default(),
                "title": row.try_get::<String, _>("title").unwrap_or_default(),
                "category": row.try_get::<String, _>("category").unwrap_or_default(),
                "tags": tags,
                "summary": row.try_get::<Option<String>, _>("summary").unwrap_or(None),
                "viewCount": row.try_get::<i64, _>("view_count").unwrap_or(0),
                "helpfulCount": row.try_get::<i64, _>("helpful_count").unwrap_or(0),
                "createdAt": row.try_get::<String, _>("created_at").unwrap_or_default(),
                "updatedAt": row.try_get::<String, _>("updated_at").unwrap_or_default(),
            })
        })
        .collect();

    let total = items.len() as u64;

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": {
            "items": items,
            "total": total,
            "query": q,
        }
    })))
}

async fn list_categories(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "knowledge:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let rows = sqlx::query(
        "SELECT category, COUNT(*) as cnt \
         FROM knowledge_items WHERE status = 'published' \
         GROUP BY category ORDER BY cnt DESC",
    )
    .fetch_all(&state.db)
    .await?;

    let cats: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "category": row.try_get::<String, _>("category").unwrap_or_default(),
                "count": row.try_get::<i64, _>("cnt").unwrap_or(0),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "code": 0, "data": cats })))
}

async fn list_tags(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "knowledge:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let rows = sqlx::query("SELECT tags FROM knowledge_items WHERE status = 'published'")
        .fetch_all(&state.db)
        .await?;

    let mut tag_counts: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    for row in &rows {
        let tags_str: String = row.try_get("tags").unwrap_or_else(|_| "[]".to_string());
        if let Ok(tags) = serde_json::from_str::<Vec<String>>(&tags_str) {
            for tag in tags {
                *tag_counts.entry(tag).or_insert(0) += 1;
            }
        }
    }

    let mut tags: Vec<(String, u64)> = tag_counts.into_iter().collect();
    tags.sort_by(|a, b| b.1.cmp(&a.1));

    let data: Vec<serde_json::Value> = tags
        .into_iter()
        .map(|(tag, count)| {
            serde_json::json!({ "tag": tag, "count": count })
        })
        .collect();

    Ok(Json(serde_json::json!({ "code": 0, "data": data })))
}

async fn list_versions(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "knowledge:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let rows = sqlx::query(
        "SELECT id, version, title, tags, edited_by_name, created_at \
         FROM knowledge_versions WHERE knowledge_id = ? \
         ORDER BY version DESC",
    )
    .bind(&id)
    .fetch_all(&state.db)
    .await?;

    let versions: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            let tags_str: String = row.try_get("tags").unwrap_or_else(|_| "[]".to_string());
            let tags: serde_json::Value =
                serde_json::from_str(&tags_str).unwrap_or(serde_json::json!([]));
            serde_json::json!({
                "id": row.try_get::<String, _>("id").unwrap_or_default(),
                "version": row.try_get::<i64, _>("version").unwrap_or(0),
                "title": row.try_get::<String, _>("title").unwrap_or_default(),
                "tags": tags,
                "editedByName": row.try_get::<Option<String>, _>("edited_by_name").unwrap_or(None),
                "createdAt": row.try_get::<String, _>("created_at").unwrap_or_default(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "code": 0, "data": versions })))
}

async fn mark_helpful(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "knowledge:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    sqlx::query("UPDATE knowledge_items SET helpful_count = helpful_count + 1 WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await?;

    let count: i64 = sqlx::query_scalar("SELECT helpful_count FROM knowledge_items WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": { "helpfulCount": count }
    })))
}

/// 一次性任务：用 jieba 重新分词所有 knowledge_items.content_text。
/// 通过 system_settings 标记防重复执行。
pub async fn resegment_knowledge_content(pool: DbPool) -> anyhow::Result<()> {
    // 1. 检查标记，已执行则跳过
    if let Some(v) = db::get_setting(&pool, "knowledge_jieba_segmented").await? {
        if v == "1" {
            return Ok(());
        }
    }
    tracing::info!("resegmenting knowledge content_text with jieba...");

    // 2. 读取所有 id + content
    let rows = sqlx::query("SELECT id, content FROM knowledge_items")
        .fetch_all(&pool)
        .await?;

    // 3. 逐条重新分词并更新
    for row in &rows {
        let id: String = row.try_get("id")?;
        let content: String = row.try_get("content")?;
        let new_text = markdown_to_text(&content);
        sqlx::query("UPDATE knowledge_items SET content_text = ? WHERE id = ?")
            .bind(&new_text)
            .bind(&id)
            .execute(&pool)
            .await?;
    }

    // 4. 写标记防重复
    db::upsert_settings(
        &pool,
        &[(
            "knowledge_jieba_segmented".to_string(),
            "1".to_string(),
            "system".to_string(),
        )],
    )
    .await?;
    tracing::info!(
        "knowledge content_text resegmented ({} rows)",
        rows.len()
    );
    Ok(())
}
