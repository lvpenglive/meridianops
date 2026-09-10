//! 告警组管理路由（独立实体，短信策略「选择组」数据源）。
//!
//! 路由：
//!   GET    /api/alert-groups/tree                全量树（无分页，避免大数量分页截断 500）  (alert_group:read)
//!   GET    /api/alert-groups                     分页列表（含 parentId / 成员数与成员姓名汇总） (alert_group:read)
//!   POST   /api/alert-groups                     新建告警组（可指定 parentId）          (alert_group:manage)
//!   PUT    /api/alert-groups/:id                 更新基本信息 / 编辑上级               (alert_group:manage)
//!   PATCH  /api/alert-groups/:id/enable          启停                                  (alert_group:manage)
//!   DELETE /api/alert-groups/:id                 删除（连同成员；有子组或策略引用时拒绝） (alert_group:manage)
//!   GET    /api/alert-groups/:id/members         查询成员（默认直属；?includeDescendants 含子孙，停用组排除） (alert_group:read)
//!   PUT    /api/alert-groups/:id/members         整体替换成员列表（按真实 user_id）     (alert_group:manage)
//!
//! 说明：MySQL 5.7 不支持递归 CTE，子树遍历在应用层完成；成员一律按真实 user_id 查询，
//! 禁止用姓名反推 ID；空组返回空列表，不报错。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::{get, patch, put},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;

use crate::auth::{self, AuthUser};
use crate::error::AppError;
use crate::routes::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/alert-groups/tree", get(tree_groups))
        .route("/api/alert-groups", get(list_groups).post(create_group))
        .route("/api/alert-groups/:id", put(update_group).delete(delete_group))
        .route("/api/alert-groups/:id/enable", patch(toggle_enable))
        .route(
            "/api/alert-groups/:id/members",
            get(list_members).put(replace_members),
        )
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListQuery {
    #[serde(default = "default_page")]
    page: u32,
    #[serde(default = "default_page_size")]
    page_size: u32,
    keyword: Option<String>,
    enabled: Option<bool>,
}

fn default_page() -> u32 {
    1
}
fn default_page_size() -> u32 {
    20
}

async fn list_groups(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "alert_group:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let page = q.page.max(1) as i64;
    let page_size = q.page_size.clamp(1, 500) as i64;
    let offset = (page - 1) * page_size;

    let mut qb = sqlx::QueryBuilder::<sqlx::MySql>::new(
        "SELECT g.id, g.code, g.name, g.description, g.enabled, g.parent_id, g.created_by, g.created_at, g.updated_at, \
                (SELECT COUNT(*) FROM alert_group_members m WHERE m.group_id = g.id) AS member_count \
         FROM alert_groups g WHERE 1=1 ",
    );
    let mut cqb = sqlx::QueryBuilder::<sqlx::MySql>::new(
        "SELECT COUNT(*) AS c FROM alert_groups g WHERE 1=1 ",
    );

    if let Some(kw) = &q.keyword {
        if !kw.is_empty() {
            let like = format!("%{}%", kw);
            qb.push(" AND (g.code LIKE ");
            qb.push_bind(like.clone());
            qb.push(" OR g.name LIKE ");
            qb.push_bind(like.clone());
            qb.push(" OR g.description LIKE ");
            qb.push_bind(like.clone());
            qb.push(")");
            cqb.push(" AND (g.code LIKE ");
            cqb.push_bind(like.clone());
            cqb.push(" OR g.name LIKE ");
            cqb.push_bind(like.clone());
            cqb.push(" OR g.description LIKE ");
            cqb.push_bind(like.clone());
            cqb.push(")");
        }
    }
    if let Some(en) = q.enabled {
        qb.push(" AND g.enabled = ");
        qb.push_bind(en);
        cqb.push(" AND g.enabled = ");
        cqb.push_bind(en);
    }

    let total: i64 = cqb
        .build()
        .fetch_one(&state.db)
        .await
        .and_then(|r| r.try_get::<i64, _>("c"))
        .unwrap_or(0);

    qb.push(" ORDER BY g.created_at DESC, g.id DESC LIMIT ");
    qb.push_bind(page_size);
    qb.push(" OFFSET ");
    qb.push_bind(offset);

    let rows = qb.build().fetch_all(&state.db).await?;

    // 取本页所有组的成员姓名汇总（一次查询，避免 N+1）
    let group_ids: Vec<String> = rows
        .iter()
        .map(|r| r.try_get::<String, _>("id").unwrap_or_default())
        .collect();
    let mut names_map: HashMap<String, Vec<String>> = HashMap::new();
    if !group_ids.is_empty() {
        let placeholders = group_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT m.group_id, COALESCE(u.display_name, u.username, m.user_id) AS uname \
             FROM alert_group_members m \
             JOIN users u ON u.id = m.user_id \
             WHERE m.group_id IN ({}) \
             ORDER BY m.group_id, u.username",
            placeholders
        );
        let mut q = sqlx::query(&sql);
        for id in &group_ids {
            q = q.bind(id);
        }
        let mrows = q.fetch_all(&state.db).await?;
        for mr in mrows {
            let gid: String = mr.try_get("group_id").unwrap_or_default();
            let uname: String = mr.try_get("uname").unwrap_or_default();
            names_map.entry(gid).or_default().push(uname);
        }
    }

    let list: Vec<Value> = rows
        .iter()
        .map(|r| {
            let gid = r.try_get::<String, _>("id").unwrap_or_default();
            let names = names_map.get(&gid).cloned().unwrap_or_default();
            json!({
                "id": gid,
                "code": r.try_get::<String, _>("code").unwrap_or_default(),
                "name": r.try_get::<String, _>("name").unwrap_or_default(),
                "description": r.try_get::<Option<String>, _>("description").ok().flatten().unwrap_or_default(),
                "enabled": r.try_get::<bool, _>("enabled").unwrap_or(true),
                "parentId": r.try_get::<Option<String>, _>("parent_id").ok().flatten(),
                "createdBy": r.try_get::<String, _>("created_by").unwrap_or_default(),
                "createdAt": r.try_get::<String, _>("created_at").unwrap_or_default(),
                "updatedAt": r.try_get::<String, _>("updated_at").unwrap_or_default(),
                "memberCount": r.try_get::<i64, _>("member_count").unwrap_or(0),
                "memberNames": names,
            })
        })
        .collect();

    Ok(Json(json!({
        "code": 0,
        "data": { "list": list, "total": total, "page": page, "pageSize": page_size }
    })))
}

// ===================== 全量树接口 =====================

/// 扁平组记录（用于树构建与子树遍历）。
#[derive(Clone)]
struct FlatGroup {
    id: String,
    code: String,
    name: String,
    description: String,
    enabled: bool,
    parent_id: Option<String>,
    member_count: i64,
}

async fn tree_groups(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "alert_group:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let rows = sqlx::query(
        "SELECT g.id, g.code, g.name, g.description, g.enabled, g.parent_id, \
                (SELECT COUNT(*) FROM alert_group_members m WHERE m.group_id = g.id) AS member_count \
         FROM alert_groups g ORDER BY g.name",
    )
    .fetch_all(&state.db)
    .await?;

    let mut flats: Vec<FlatGroup> = Vec::with_capacity(rows.len());
    for r in &rows {
        flats.push(FlatGroup {
            id: r.try_get::<String, _>("id").unwrap_or_default(),
            code: r.try_get::<String, _>("code").unwrap_or_default(),
            name: r.try_get::<String, _>("name").unwrap_or_default(),
            description: r
                .try_get::<Option<String>, _>("description")
                .ok()
                .flatten()
                .unwrap_or_default(),
            enabled: r.try_get::<bool, _>("enabled").unwrap_or(true),
            parent_id: r.try_get::<Option<String>, _>("parent_id").ok().flatten(),
            member_count: r.try_get::<i64, _>("member_count").unwrap_or(0),
        });
    }

    let by_id: HashMap<String, FlatGroup> = flats
        .iter()
        .map(|f| (f.id.clone(), f.clone()))
        .collect();

    // parent -> 子节点 id 列表；父不存在（或指向不存在的组）一律当作顶级，避免孤儿节点丢失
    let mut children: HashMap<Option<String>, Vec<String>> = HashMap::new();
    for f in &flats {
        let key = match &f.parent_id {
            Some(p) if by_id.contains_key(p) => Some(p.clone()),
            _ => None,
        };
        children.entry(key).or_default().push(f.id.clone());
    }

    fn build(
        parent: &Option<String>,
        children: &HashMap<Option<String>, Vec<String>>,
        by_id: &HashMap<String, FlatGroup>,
        visited: &mut HashSet<String>,
    ) -> Vec<Value> {
        let Some(ids) = children.get(parent) else {
            return vec![];
        };
        let mut out = Vec::new();
        for id in ids {
            if !visited.insert(id.clone()) {
                continue; // 循环保护
            }
            let f = &by_id[id];
            let mut kids = build(&Some(id.clone()), children, by_id, visited);
            kids.sort_by(|a, b| {
                a.get("name")
                    .and_then(|v| v.as_str())
                    .cmp(&b.get("name").and_then(|v| v.as_str()))
            });
            out.push(json!({
                "id": f.id,
                "code": f.code,
                "name": f.name,
                "description": f.description,
                "enabled": f.enabled,
                "parentId": f.parent_id,
                "memberCount": f.member_count,
                "children": kids,
            }));
        }
        out
    }

    let mut visited = HashSet::new();
    let tree = build(&None, &children, &by_id, &mut visited);

    Ok(Json(json!({ "code": 0, "data": tree })))
}

// ===================== 层级辅助 =====================

/// 仅加载 id / parent_id / enabled，用于循环检测与子树遍历（应用层，兼容 MySQL 5.7）。
async fn load_all_groups(db: &sqlx::MySqlPool) -> Result<Vec<GroupIdRow>, AppError> {
    let rows = sqlx::query("SELECT id, parent_id, enabled FROM alert_groups")
        .fetch_all(db)
        .await?;
    let mut out = Vec::with_capacity(rows.len());
    for r in &rows {
        out.push(GroupIdRow {
            id: r.try_get::<String, _>("id").unwrap_or_default(),
            parent_id: r.try_get::<Option<String>, _>("parent_id").ok().flatten(),
            enabled: r.try_get::<bool, _>("enabled").unwrap_or(true),
        });
    }
    Ok(out)
}

struct GroupIdRow {
    id: String,
    parent_id: Option<String>,
    enabled: bool,
}

/// 从 root 出发收集其自身 + 所有子孙（应用层 DFS），含循环保护。
fn collect_subtree(root: &str, all: &[GroupIdRow]) -> Vec<String> {
    let mut children: HashMap<Option<String>, Vec<String>> = HashMap::new();
    for g in all {
        children.entry(g.parent_id.clone()).or_default().push(g.id.clone());
    }
    let mut result = Vec::new();
    let mut stack = vec![root.to_string()];
    let mut visited = HashSet::new();
    while let Some(cur) = stack.pop() {
        if !visited.insert(cur.clone()) {
            continue;
        }
        result.push(cur.clone());
        if let Some(kids) = children.get(&Some(cur.clone())) {
            for k in kids {
                stack.push(k.clone());
            }
        }
    }
    result
}

/// candidate 是否为 ancestor 的子孙（用于防止把组移动到自身子树之下）。
fn is_descendant_of(candidate: &str, ancestor: &str, all: &[GroupIdRow]) -> bool {
    let subtree = collect_subtree(ancestor, all);
    subtree.iter().any(|x| x == candidate)
}

/// 解析并校验 parent_id：空/None 表示顶级；存在性校验；若给定 self_id 则防自身与后代循环。
async fn resolve_parent(
    db: &sqlx::MySqlPool,
    parent_id: &Option<String>,
    self_id: Option<&str>,
) -> Result<Option<String>, AppError> {
    let pid = match parent_id {
        None => return Ok(None),
        Some(p) if p.trim().is_empty() => return Ok(None),
        Some(p) => p.trim().to_string(),
    };

    let exists: Option<String> =
        sqlx::query_scalar("SELECT id FROM alert_groups WHERE id = ?")
            .bind(&pid)
            .fetch_optional(db)
            .await?;
    if exists.is_none() {
        return Err(AppError::bad("上级组不存在"));
    }
    if let Some(sid) = self_id {
        if &pid == sid {
            return Err(AppError::bad("不能将组设为自身的上级"));
        }
        let all = load_all_groups(db).await?;
        if is_descendant_of(&pid, sid, &all) {
            return Err(AppError::bad("不能将组移动到其子组之下（会形成循环）"));
        }
    }
    Ok(Some(pid))
}

// ===================== 新建 / 更新 =====================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GroupReq {
    code: String,
    name: String,
    description: Option<String>,
    #[serde(default = "default_enabled")]
    enabled: bool,
    parent_id: Option<String>,
}

fn default_enabled() -> bool {
    true
}

async fn create_group(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(req): Json<GroupReq>,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "alert_group:manage")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let code = req.code.trim().to_string();
    let name = req.name.trim().to_string();
    if code.is_empty() || name.is_empty() {
        return Err(AppError::bad("编码和名称不能为空"));
    }

    let parent_id = resolve_parent(&state.db, &req.parent_id, None).await?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO alert_groups (id, code, name, description, parent_id, enabled, created_by, created_at, updated_at) \
         VALUES (?,?,?,?,?,?,?,?,?)",
    )
    .bind(&id)
    .bind(&code)
    .bind(&name)
    .bind(req.description.as_deref())
    .bind(&parent_id)
    .bind(req.enabled)
    .bind(&auth.0.sub)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("Duplicate") {
            AppError::bad(&format!("编码「{}」已存在", code))
        } else {
            AppError::from(e)
        }
    })?;

    Ok(Json(json!({ "code": 0, "data": { "id": id } })))
}

async fn update_group(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<GroupReq>,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "alert_group:manage")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let code = req.code.trim().to_string();
    let name = req.name.trim().to_string();
    if code.is_empty() || name.is_empty() {
        return Err(AppError::bad("编码和名称不能为空"));
    }
    let parent_id = resolve_parent(&state.db, &req.parent_id, Some(&id)).await?;
    let now = chrono::Utc::now().to_rfc3339();

    let result = sqlx::query(
        "UPDATE alert_groups SET code = ?, name = ?, description = ?, parent_id = ?, enabled = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&code)
    .bind(&name)
    .bind(req.description.as_deref())
    .bind(&parent_id)
    .bind(req.enabled)
    .bind(&now)
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("Duplicate") {
            AppError::bad(&format!("编码「{}」已存在", code))
        } else {
            AppError::from(e)
        }
    })?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("告警组不存在"));
    }
    Ok(Json(json!({ "code": 0, "message": "ok" })))
}

#[derive(Debug, Deserialize)]
struct ToggleReq {
    enabled: bool,
}

async fn toggle_enable(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<ToggleReq>,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "alert_group:manage")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let result = sqlx::query("UPDATE alert_groups SET enabled = ?, updated_at = ? WHERE id = ?")
        .bind(req.enabled)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(&id)
        .execute(&state.db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::not_found("告警组不存在"));
    }
    Ok(Json(json!({ "code": 0, "message": "ok" })))
}

async fn delete_group(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "alert_group:manage")?;
    crate::license_routes::require_active_license(&state.db).await?;

    // 有子组则拒绝
    let child_count: Option<i64> =
        sqlx::query_scalar("SELECT COUNT(*) FROM alert_groups WHERE parent_id = ?")
            .bind(&id)
            .fetch_optional(&state.db)
            .await?;
    if child_count.unwrap_or(0) > 0 {
        return Err(AppError::bad("该组下存在子组，请先移除或转移子组后再删除"));
    }

    // 被短信策略引用则拒绝
    let ref_count: Option<i64> =
        sqlx::query_scalar("SELECT COUNT(*) FROM alert_sms_strategies WHERE alert_group_id = ?")
            .bind(&id)
            .fetch_optional(&state.db)
            .await?;
    if ref_count.unwrap_or(0) > 0 {
        return Err(AppError::bad("该组被短信策略引用，无法删除"));
    }

    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM alert_group_members WHERE group_id = ?")
        .bind(&id)
        .execute(&mut *tx)
        .await?;
    let result = sqlx::query("DELETE FROM alert_groups WHERE id = ?")
        .bind(&id)
        .execute(&mut *tx)
        .await?;
    if result.rows_affected() == 0 {
        tx.rollback().await.ok();
        return Err(AppError::not_found("告警组不存在"));
    }
    tx.commit().await?;

    Ok(Json(json!({ "code": 0, "message": "ok" })))
}

// ===================== 成员 =====================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MembersQuery {
    #[serde(default)]
    include_descendants: bool,
}

async fn list_members(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<String>,
    Query(q): Query<MembersQuery>,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "alert_group:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    // 收集需要查询成员的目标组 id：
    //   - 默认：仅直属（用于成员维护），空组返回空。
    //   - includeDescendants：含全部子孙；应用层遍历，并明确排除停用组（含根组本身若停用）。
    let group_ids: Vec<String> = if q.include_descendants {
        let all = load_all_groups(&state.db).await?;
        let mut ids = collect_subtree(&id, &all);
        // 明确停用组排除规则：移除 enabled=false 的组（含根组若停用）
        ids.retain(|gid| {
            all.iter()
                .find(|x| &x.id == gid)
                .map(|x| x.enabled)
                .unwrap_or(false)
        });
        ids
    } else {
        vec![id.clone()]
    };

    if group_ids.is_empty() {
        return Ok(Json(json!({ "code": 0, "data": [] })));
    }

    // 按真实 user_id 查询并去重（跨组重复成员只出现一次），禁止姓名反推 ID。
    let placeholders = vec!["?"; group_ids.len()].join(",");
    let sql = format!(
        "SELECT DISTINCT m.user_id AS user_id, \
                COALESCE(u.display_name, u.username) AS display_name, u.username, u.email, u.enabled \
         FROM alert_group_members m \
         JOIN users u ON u.id = m.user_id \
         WHERE m.group_id IN ({}) \
         ORDER BY u.username",
        placeholders
    );
    let mut q = sqlx::query(&sql);
    for gid in &group_ids {
        q = q.bind(gid);
    }
    let rows = q.fetch_all(&state.db).await?;

    let list: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "id": r.try_get::<String, _>("user_id").unwrap_or_default(),
                "displayName": r.try_get::<Option<String>, _>("display_name").ok().flatten(),
                "username": r.try_get::<Option<String>, _>("username").ok().flatten(),
                "email": r.try_get::<Option<String>, _>("email").ok().flatten(),
                "enabled": r.try_get::<bool, _>("enabled").unwrap_or(true),
            })
        })
        .collect();

    Ok(Json(json!({ "code": 0, "data": list })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MembersReq {
    user_ids: Vec<String>,
}

async fn replace_members(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<MembersReq>,
) -> Result<Json<Value>, AppError> {
    auth::require_permission(&auth, "alert_group:manage")?;
    crate::license_routes::require_active_license(&state.db).await?;

    // 校验组存在
    let exists: Option<String> = sqlx::query_scalar("SELECT id FROM alert_groups WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db)
        .await?;
    if exists.is_none() {
        return Err(AppError::not_found("告警组不存在"));
    }

    // 成员必须真实存在
    let mut valid_ids: Vec<String> = Vec::with_capacity(req.user_ids.len());
    for uid in &req.user_ids {
        let u: Option<String> = sqlx::query_scalar("SELECT id FROM users WHERE id = ?")
            .bind(uid)
            .fetch_optional(&state.db)
            .await?;
        if u.is_some() {
            valid_ids.push(uid.clone());
        }
    }

    let now = chrono::Utc::now().to_rfc3339();
    let mut tx = state.db.begin().await?;
    sqlx::query("DELETE FROM alert_group_members WHERE group_id = ?")
        .bind(&id)
        .execute(&mut *tx)
        .await?;

    for uid in &valid_ids {
        sqlx::query(
            "INSERT IGNORE INTO alert_group_members (group_id, user_id, created_at) VALUES (?,?,?)",
        )
        .bind(&id)
        .bind(uid)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query("UPDATE alert_groups SET updated_at = ? WHERE id = ?")
        .bind(&now)
        .bind(&id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(Json(json!({ "code": 0, "message": "ok", "data": { "count": valid_ids.len() } })))
}
