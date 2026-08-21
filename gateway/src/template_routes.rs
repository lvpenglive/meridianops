//! Workflow Template routes
//!  - GET    /api/workflow-templates                  列表
//!  - GET    /api/workflow-templates/:id              详情
//!  - POST   /api/workflow-templates                  新建
//!  - PUT    /api/workflow-templates/:id              更新（definition 变更时版本递增）
//!  - DELETE /api/workflow-templates/:id              软删除（仅 custom）
//!  - POST   /api/workflow-templates/:id/enable       启停
//!  - POST   /api/workflow-templates/compile          预览编译结果

use std::sync::Arc;
use axum::{extract::{Path, Query, State}, Json, Router, routing::{get, post, delete, put}};
use serde::{Deserialize};
use serde_json::{json, Value};
use sqlx::Row;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::routes::AppState;
use crate::workflow_engine::{compile_definition, LfDefinition};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/workflow-templates", post(create_template).get(list_templates))
        .route("/api/workflow-templates/compile", post(compile_template_preview))
        .route("/api/workflow-templates/:id", get(get_template).put(update_template).delete(delete_template))
        .route("/api/workflow-templates/:id/enable", post(enable_template))
}

#[derive(Debug, Deserialize)]
pub struct TmplListQ {
    pub ticketType: Option<String>,
    pub enabled: Option<i8>,
    pub scope: Option<String>,
    pub keyword: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTmplReq {
    pub name: String,
    #[serde(default)] pub displayName: Option<String>,
    pub ticketType: String,
    #[serde(default)] pub category: Option<String>,
    pub definition: Value,
    #[serde(default)] pub description: Option<String>,
    #[serde(default = "one")] pub version: i32,
    #[serde(default = "tru")] pub enabled: bool,
    #[serde(default = "custom_scope")] pub scope: String,
}
fn one() -> i32 { 1 }
fn tru() -> bool { true }
fn custom_scope() -> String { "custom".into() }

#[derive(Debug, Deserialize)]
pub struct UpdateTmplReq {
    pub name: Option<String>,
    pub displayName: Option<String>,
    pub ticketType: Option<String>,
    pub category: Option<String>,
    pub definition: Option<Value>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub scope: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EnableReq { pub enabled: bool }
#[derive(Debug, Deserialize)]
pub struct CompileReq { pub definition: Value }

async fn def_compile_errors(def_val: &Value) -> Vec<String> {
    let lf = match serde_json::from_value::<LfDefinition>(def_val.clone()) {
        Ok(v) => v, Err(e) => return vec![format!("definition 解析失败: {}", e)],
    };
    let (_, errs) = compile_definition(&lf);
    errs
}

fn bind_val<'q>(q: sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>, b: &'q Value) -> sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments> {
    match b {
        Value::String(s) => q.bind(s),
        Value::Number(n) => if let Some(i) = n.as_i64() { q.bind(i) } else if let Some(f) = n.as_f64() { q.bind(f) } else { q.bind(n.to_string()) },
        Value::Bool(x) => q.bind(x),
        Value::Null => q.bind(None::<String>),
        _ => q.bind(b.to_string()),
    }
}

pub async fn list_templates(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Query(q): Query<TmplListQ>,
) -> Result<Json<Value>, AppError> {
    crate::auth::require_permission(&auth, "workflow:read")
        .or_else(|_| crate::auth::require_permission(&auth, "workflow:admin"))?;
    let pool = &state.db;
    let mut wc = vec!["t.deleted_at IS NULL".to_string()];
    let mut binds = Vec::<Value>::new();
    if let Some(v) = q.ticketType { if !v.is_empty() { wc.push("t.ticket_type=?".into()); binds.push(json!(v)); } }
    if let Some(v) = q.enabled { wc.push("t.enabled=?".into()); binds.push(json!(v)); }
    if let Some(v) = q.scope { if !v.is_empty() { wc.push("t.scope=?".into()); binds.push(json!(v)); } }
    if let Some(kw) = q.keyword { if !kw.is_empty() {
        wc.push("(t.name LIKE ? OR t.display_name LIKE ? OR t.description LIKE ?)".into());
        let p = format!("%{}%", kw);
        binds.push(json!(p.clone())); binds.push(json!(p.clone())); binds.push(json!(p));
    }}
    let wsql = format!(" WHERE {}", wc.join(" AND "));
    let sql = format!("SELECT t.id, t.name, t.display_name, t.ticket_type, t.category, t.version, t.enabled,
        t.scope, t.description, t.created_by, t.created_at, t.updated_at,
        u.display_name AS creator_name
        FROM workflow_templates t LEFT JOIN users u ON u.id = t.created_by
        {} ORDER BY CASE WHEN t.scope='builtin' THEN 0 ELSE 1 END, t.ticket_type ASC, t.name ASC, t.version DESC", wsql);
    let mut qb = sqlx::query(&sql);
    for b in &binds { qb = bind_val(qb, b); }
    let rows = qb.fetch_all(pool).await.unwrap_or_default();
    let items: Vec<Value> = rows.iter().map(|r| json!({
        "id": r.try_get::<String,_>("id").unwrap_or_default(),
        "name": r.try_get::<String,_>("name").unwrap_or_default(),
        "displayName": r.try_get::<Option<String>,_>("display_name").unwrap_or(None),
        "ticketType": r.try_get::<String,_>("ticket_type").unwrap_or_default(),
        "category": r.try_get::<Option<String>,_>("category").unwrap_or(None),
        "version": r.try_get::<i32,_>("version").unwrap_or(1),
        "enabled": r.try_get::<bool,_>("enabled").unwrap_or(false),
        "scope": r.try_get::<String,_>("scope").unwrap_or("custom".to_string()),
        "description": r.try_get::<Option<String>,_>("description").unwrap_or(None),
        "createdBy": r.try_get::<Option<String>,_>("created_by").unwrap_or(None),
        "creatorName": r.try_get::<Option<String>,_>("creator_name").unwrap_or(None),
        "createdAt": r.try_get::<String,_>("created_at").unwrap_or_default(),
        "updatedAt": r.try_get::<String,_>("updated_at").unwrap_or_default(),
    })).collect();
    Ok(Json(json!({"code":0,"message":"ok","data":items})))
}

pub async fn get_template(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    crate::auth::require_permission(&auth, "workflow:read")
        .or_else(|_| crate::auth::require_permission(&auth, "workflow:admin"))?;
    let pool = &state.db;
    let row = sqlx::query(
        "SELECT t.id, t.name, t.display_name, t.ticket_type, t.category, t.definition, t.version,
            t.enabled, t.scope, t.description, t.created_by, t.created_at, t.updated_at,
            u.display_name AS creator_name
         FROM workflow_templates t LEFT JOIN users u ON u.id = t.created_by
         WHERE t.id=? AND t.deleted_at IS NULL LIMIT 1"
    ).bind(&id).fetch_optional(pool).await?;
    let r = row.ok_or(AppError::not_found("模板不存在"))?;
    let definition = r.try_get::<Option<Value>,_>("definition").unwrap_or(Some(Value::Null)).unwrap_or(Value::Null);
    let errs = def_compile_errors(&definition).await;
    let tmpl = json!({
        "id": r.try_get::<String,_>("id").unwrap_or_default(),
        "name": r.try_get::<String,_>("name").unwrap_or_default(),
        "displayName": r.try_get::<Option<String>,_>("display_name").unwrap_or(None),
        "ticketType": r.try_get::<String,_>("ticket_type").unwrap_or_default(),
        "category": r.try_get::<Option<String>,_>("category").unwrap_or(None),
        "definition": definition,
        "version": r.try_get::<i32,_>("version").unwrap_or(1),
        "enabled": r.try_get::<bool,_>("enabled").unwrap_or(false),
        "scope": r.try_get::<String,_>("scope").unwrap_or("custom".to_string()),
        "description": r.try_get::<Option<String>,_>("description").unwrap_or(None),
        "createdBy": r.try_get::<Option<String>,_>("created_by").unwrap_or(None),
        "creatorName": r.try_get::<Option<String>,_>("creator_name").unwrap_or(None),
        "createdAt": r.try_get::<String,_>("created_at").unwrap_or_default(),
        "updatedAt": r.try_get::<String,_>("updated_at").unwrap_or_default(),
    });
    Ok(Json(json!({"code":0,"message":"ok","data":{"template": tmpl, "compileErrors": errs}})))
}

pub async fn create_template(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(req): Json<CreateTmplReq>,
) -> Result<Json<Value>, AppError> {
    crate::auth::require_permission(&auth, "workflow:admin")?;
    if req.name.trim().is_empty() { return Err(AppError::bad("name 不能为空")); }
    let errs = def_compile_errors(&req.definition).await;
    if !errs.is_empty() { return Err(AppError::bad(&format!("definition 不合法: {:?}", errs))); }
    let pool = &state.db;
    let id = Uuid::new_v4().to_string();
    let def_str = serde_json::to_string(&req.definition).unwrap_or_else(|_| "{}".into());
    let disp = req.displayName.clone().unwrap_or_else(|| req.name.trim().to_string());
    sqlx::query(
        "INSERT INTO workflow_templates (id, name, display_name, ticket_type, category,
            definition, version, enabled, scope, description, created_by, created_at, updated_at)
         VALUES (?,?,?,?,?,?,?,?,?,?,?,NOW(),NOW())"
    ).bind(&id).bind(&req.name).bind(&disp).bind(&req.ticketType).bind(req.category.as_deref())
     .bind(&def_str).bind(req.version).bind(req.enabled).bind(&req.scope)
     .bind(req.description.as_deref()).bind(&auth.0.uid).execute(pool).await?;
    Ok(Json(json!({"code":0,"message":"ok","data":{"id":id}})))
}

pub async fn update_template(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateTmplReq>,
) -> Result<Json<Value>, AppError> {
    crate::auth::require_permission(&auth, "workflow:admin")?;
    let pool = &state.db;
    let old = sqlx::query(
        "SELECT id, name, ticket_type, category, definition, version, enabled, scope, description, display_name
         FROM workflow_templates WHERE id=? AND deleted_at IS NULL LIMIT 1"
    ).bind(&id).fetch_optional(pool).await?;
    let old_r = old.ok_or(AppError::not_found("模板不存在"))?;
    let o_name = old_r.try_get::<String,_>("name").unwrap_or_default();
    let o_type = old_r.try_get::<String,_>("ticket_type").unwrap_or_default();
    let o_cat = old_r.try_get::<Option<String>,_>("category").ok().flatten();
    let o_def = old_r.try_get::<Option<Value>,_>("definition").ok().flatten().unwrap_or(Value::Null);
    let o_ver = old_r.try_get::<i32,_>("version").unwrap_or(1);
    let o_en = old_r.try_get::<bool,_>("enabled").unwrap_or(true);
    let o_scope = old_r.try_get::<String,_>("scope").unwrap_or("custom".to_string());
    let o_desc = old_r.try_get::<Option<String>,_>("description").ok().flatten();
    let o_disp = old_r.try_get::<Option<String>,_>("display_name").ok().flatten();

    let name = if req.name.is_none() { o_name } else { req.name.unwrap() };
    let ticket_type = if req.ticketType.is_none() { o_type } else { req.ticketType.unwrap() };
    let category = req.category.or(o_cat);
    let description = req.description.or(o_desc);
    let scope = if req.scope.is_none() { o_scope } else { req.scope.unwrap() };
    let enabled = req.enabled.unwrap_or(o_en);
    let display = req.displayName.or(o_disp);
    let (def, ver) = if let Some(d) = req.definition {
        if d != o_def {
            let errs = def_compile_errors(&d).await;
            if !errs.is_empty() { return Err(AppError::bad(&format!("definition 不合法: {:?}", errs))); }
            (d, o_ver + 1)
        } else { (d, o_ver) }
    } else { (o_def, o_ver) };
    let def_str = serde_json::to_string(&def).unwrap_or_else(|_| "{}".into());
    sqlx::query(
        "UPDATE workflow_templates SET name=?, display_name=COALESCE(?, display_name),
            ticket_type=?, category=?, definition=?, version=?, enabled=?, scope=?,
            description=?, updated_at=NOW()
         WHERE id=?"
    ).bind(&name).bind(display.as_deref()).bind(&ticket_type).bind(category.as_deref())
     .bind(&def_str).bind(ver).bind(enabled).bind(&scope).bind(description.as_deref()).bind(&id)
     .execute(pool).await?;
    Ok(Json(json!({"code":0,"message":"ok","data":{"version":ver}})))
}

pub async fn delete_template(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    crate::auth::require_permission(&auth, "workflow:admin")?;
    let pool = &state.db;
    let scope = sqlx::query("SELECT scope FROM workflow_templates WHERE id=? LIMIT 1")
        .bind(&id).fetch_optional(pool).await?;
    if let Some(r) = scope {
        let s = r.try_get::<Option<String>,_>("scope").ok().flatten().unwrap_or_default();
        if s == "builtin" { return Err(AppError::bad("内置模板不允许删除")); }
    } else { return Err(AppError::not_found("模板不存在")); }
    sqlx::query("UPDATE workflow_templates SET deleted_at=NOW(), enabled=0, updated_at=NOW() WHERE id=? AND deleted_at IS NULL")
        .bind(&id).execute(pool).await?;
    Ok(Json(json!({"code":0,"message":"ok"})))
}

pub async fn enable_template(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<EnableReq>,
) -> Result<Json<Value>, AppError> {
    crate::auth::require_permission(&auth, "workflow:admin")?;
    let pool = &state.db;
    if req.enabled {
        let def = sqlx::query_scalar::<_, Option<Value>>("SELECT definition FROM workflow_templates WHERE id=? LIMIT 1")
            .bind(&id).fetch_optional(pool).await.unwrap_or(None).flatten();
        if let Some(d) = def {
            let errs = def_compile_errors(&d).await;
            if !errs.is_empty() { return Err(AppError::bad(&format!("definition 不合法: {:?}", errs))); }
        }
    }
    sqlx::query("UPDATE workflow_templates SET enabled=?, updated_at=NOW() WHERE id=? AND deleted_at IS NULL")
        .bind(req.enabled).bind(&id).execute(pool).await?;
    Ok(Json(json!({"code":0,"message":"ok"})))
}

pub async fn compile_template_preview(
    auth: AuthUser,
    Json(req): Json<CompileReq>,
) -> Result<Json<Value>, AppError> {
    crate::auth::require_permission(&auth, "workflow:read")
        .or_else(|_| crate::auth::require_permission(&auth, "workflow:admin"))?;
    let lf = match serde_json::from_value::<LfDefinition>(req.definition) {
        Ok(v) => v, Err(e) => return Ok(Json(json!({"code":0,"message":"ok","data":Value::Null,"errors":[format!("{}",e)]}))),
    };
    let (nodes, errors) = compile_definition(&lf);
    let edges_info: Vec<Value> = lf.edges.iter().map(|e| json!({
        "id": e.id, "source": e.source, "target": e.target, "condition": e.properties.get("condition")
    })).collect();
    Ok(Json(json!({
        "code":0,"message":"ok",
        "data":{ "nodes": nodes, "edges": edges_info },
        "errors": errors
    })))
}