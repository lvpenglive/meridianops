//! 系统组件连通状态：探测本进程及配置中的依赖。
//!
//! GET /api/system/components  （system:read）
//! 地址只读返回（MySQL 密码脱敏）。不在页面上提供修改。

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use crate::auth;
use crate::error::AppError;
use crate::routes::AppState;

const PROBE_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ComponentStatus {
    id: String,
    name: String,
    kind: String,
    address: String,
    status: String,
    latency_ms: u64,
    message: String,
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/api/system/components", get(list_components))
}

async fn list_components(
    State(state): State<Arc<AppState>>,
    auth: auth::AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    auth::require_permission(&auth, "system:read")?;
    crate::license_routes::require_active_license(&state.db).await?;

    let clickhouse_probe = join_url(&state.config.logs.clickhouse_url, "/ping");
    let clickhouse_display = state.config.logs.clickhouse_url.clone();
    let loki_probe = join_url(&state.config.logs.loki_url, "/ready");
    let loki_display = state.config.logs.loki_url.clone();

    let (mysql, clickhouse, loki) = tokio::join!(
        probe_mysql(&state),
        probe_http_named(
            &state,
            "clickhouse",
            "ClickHouse",
            "log-store",
            &clickhouse_probe,
            &clickhouse_display,
        ),
        probe_http_named(
            &state,
            "loki",
            "Loki",
            "log-index",
            &loki_probe,
            &loki_display,
        ),
    );

    let mut items = vec![gateway_self(&state), mysql, clickhouse, loki];

    let mut handles = Vec::new();
    for sys in &state.config.systems {
        let st = state.clone();
        let id = sys.id.clone();
        let name = sys.name.clone();
        let kind = sys.system_type.clone();
        let addr = sys.base_url.clone();
        handles.push(tokio::spawn(async move {
            probe_http_named(&st, &id, &name, &kind, &addr, &addr).await
        }));
    }
    for handle in handles {
        if let Ok(item) = handle.await {
            items.push(item);
        }
    }

    let ok = items.iter().filter(|i| i.status == "ok").count();
    let failed = items.iter().filter(|i| i.status == "error").count();

    Ok(Json(serde_json::json!({
        "code": 0,
        "data": {
            "ok": ok,
            "failed": failed,
            "total": items.len(),
            "items": items,
        }
    })))
}

fn gateway_self(state: &AppState) -> ComponentStatus {
    ComponentStatus {
        id: "gateway".into(),
        name: "API 网关".into(),
        kind: "self".into(),
        address: state.config.server.bind.clone(),
        status: "ok".into(),
        latency_ms: 0,
        message: "本进程".into(),
    }
}

async fn probe_mysql(state: &AppState) -> ComponentStatus {
    let address = mask_mysql_url(&state.config.database.url);
    let start = Instant::now();
    match sqlx::query("SELECT 1").execute(&state.db).await {
        Ok(_) => ComponentStatus {
            id: "mysql".into(),
            name: "MySQL".into(),
            kind: "database".into(),
            address,
            status: "ok".into(),
            latency_ms: elapsed_ms(start),
            message: "SELECT 1 成功".into(),
        },
        Err(e) => ComponentStatus {
            id: "mysql".into(),
            name: "MySQL".into(),
            kind: "database".into(),
            address,
            status: "error".into(),
            latency_ms: elapsed_ms(start),
            message: sanitize_error(&e.to_string()),
        },
    }
}

async fn probe_http_named(
    state: &AppState,
    id: &str,
    name: &str,
    kind: &str,
    probe_url: &str,
    display_url: &str,
) -> ComponentStatus {
    let start = Instant::now();
    match state
        .client
        .get(probe_url)
        .timeout(PROBE_TIMEOUT)
        .send()
        .await
    {
        Ok(resp) => {
            let code = resp.status().as_u16();
            let ok = code < 500;
            ComponentStatus {
                id: id.to_string(),
                name: name.to_string(),
                kind: kind.to_string(),
                address: display_url.to_string(),
                status: if ok { "ok".into() } else { "error".into() },
                latency_ms: elapsed_ms(start),
                message: format!("HTTP {code}"),
            }
        }
        Err(e) => ComponentStatus {
            id: id.to_string(),
            name: name.to_string(),
            kind: kind.to_string(),
            address: display_url.to_string(),
            status: "error".into(),
            latency_ms: elapsed_ms(start),
            message: sanitize_error(&e.to_string()),
        },
    }
}

fn join_url(base: &str, path: &str) -> String {
    format!("{}{}", base.trim_end_matches('/'), path)
}

fn elapsed_ms(start: Instant) -> u64 {
    start.elapsed().as_millis() as u64
}

fn mask_mysql_url(url: &str) -> String {
    if let Some(scheme_end) = url.find("://") {
        let rest = &url[scheme_end + 3..];
        if let Some(at) = rest.find('@') {
            let userinfo = &rest[..at];
            let host = &rest[at + 1..];
            let user = userinfo.split(':').next().unwrap_or("*");
            return format!("{}://{}:***@{}", &url[..scheme_end], user, host);
        }
    }
    url.to_string()
}

fn sanitize_error(msg: &str) -> String {
    let mut out = msg.to_string();
    if let Some(start) = out.find("://") {
        if let Some(rel_at) = out[start + 3..].find('@') {
            let userinfo_start = start + 3;
            let at = userinfo_start + rel_at;
            if let Some(colon) = out[userinfo_start..at].find(':') {
                let pwd_start = userinfo_start + colon + 1;
                out.replace_range(pwd_start..at, "***");
            }
        }
    }
    out.chars().take(240).collect()
}
