use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;

use crate::config::GatewayConfig;

pub struct AppState {
    pub config: Arc<GatewayConfig>,
    pub client: Client,
    pub db: sqlx::MySqlPool,
    pub jwt_secret: String,
    pub jwt_ttl_hours: u64,
}

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/health", get(health_check))
        .merge(crate::auth_routes::routes())
        .merge(crate::audit_routes::routes())
        .merge(crate::role_routes::routes())
        .merge(crate::dept_routes::routes())
        .merge(crate::system_routes::routes())
        .merge(crate::dashboard_routes::routes())
        .merge(crate::report_routes::routes())
        .merge(crate::cmdb_routes::routes())
        .merge(crate::token_routes::routes())
        .merge(crate::knowledge_routes::routes())
        .merge(crate::dict_routes::routes())
        .merge(crate::license_routes::routes())
        .merge(crate::job_routes::routes())
        .merge(crate::credential_routes::routes())
        .route("/api/systems", get(list_systems))
        .route("/api/systems/:id", get(get_system))
        .route("/api/proxy/*rest", proxy_any_method())
        .route("/api/aggregate/overview", get(aggregate_overview))
        .route("/api/aggregate/alerts", get(aggregate_alerts))
        .layer(tower_http::cors::CorsLayer::permissive())
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(state)
}

fn json_response(data: serde_json::Value) -> Response {
    Json(data).into_response()
}

async fn health_check() -> &'static str {
    "ok"
}

async fn list_systems(State(state): State<Arc<AppState>>) -> Response {
    json_response(serde_json::json!({
        "code": 0,
        "data": state.config.systems,
    }))
}

async fn get_system(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Response {
    match state.config.system_by_id(&id) {
        Some(sys) => json_response(serde_json::json!({
            "code": 0,
            "data": sys,
        })),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "code": 404,
                "message": format!("System '{}' not found", id),
            })),
        )
            .into_response(),
    }
}

async fn proxy_request(
    State(state): State<Arc<AppState>>,
    Path(rest): Path<String>,
    method: Method,
    headers: HeaderMap,
    Query(params): Query<Option<serde_json::Value>>,
    body: axum::body::Body,
) -> Response {
    // rest 形如 "id/remaining/path"，拆出首段为系统 id，其余为代理路径。
    // matchit 0.7.x 不允许 ":id/*path"（参数后接 catch-all），故用单个 catch-all 手动拆分。
    let (id, path) = match rest.find('/') {
        Some(idx) => (rest[..idx].to_string(), rest[idx + 1..].to_string()),
        None => (rest, String::new()),
    };

    let system = match state.config.system_by_id(&id) {
        Some(s) => s.clone(),
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "code": 404, "message": "System not found" })),
            )
                .into_response();
        }
    };

    let target_url = format!("{}/{}", system.base_url.trim_end_matches('/'), path);

    let mut req = state.client.request(method.clone(), &target_url);

    if let Some(token) = &system.auth_token {
        req = req.header("X-AxleOps-Token", token);
        req = req.header("Authorization", format!("Bearer {}", token));
    }

    if let Some(username) = &system.auth_username {
        if let Some(password) = &system.auth_password {
            req = req.basic_auth(username, Some(password));
        }
    }

    for (key, value) in headers.iter() {
        let k = key.as_str();
        if k != "host" && k != "content-length" && k != "connection" {
            req = req.header(key.clone(), value);
        }
    }

    if let Some(params) = params {
        req = req.query(&[("params", params.to_string())]);
    }

    let body_bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .unwrap_or_default();

    if !body_bytes.is_empty() {
        req = req.body(body_bytes);
    }

    match req.send().await {
        Ok(resp) => {
            let status = resp.status();
            let resp_headers = resp.headers().clone();
            let body = resp.bytes().await.unwrap_or_default();

            let mut builder = Response::builder().status(status);

            for (key, value) in resp_headers.iter() {
                let k = key.as_str();
                if k != "server" && k != "date" && k != "connection" {
                    builder = builder.header(key.clone(), value);
                }
            }

            builder
                .header("x-proxied-by", "meridianops-gateway")
                .body(body.to_vec().into())
                .unwrap_or_else(|_| {
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(axum::body::Body::empty())
                        .unwrap()
                })
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({
                "code": 502,
                "message": format!("Proxy request failed: {}", e),
            })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
struct OverviewParams {
    #[serde(default = "default_limit")]
    limit: u32,
}

fn default_limit() -> u32 {
    10
}

async fn aggregate_overview(
    State(state): State<Arc<AppState>>,
    Query(_params): Query<OverviewParams>,
) -> Response {
    let systems = &state.config.systems;

    json_response(serde_json::json!({
        "code": 0,
        "data": {
            "systems": systems.iter().map(|s| {
                serde_json::json!({
                    "id": s.id,
                    "name": s.name,
                    "status": s.status,
                    "version": s.version,
                })
            }).collect::<Vec<_>>(),
            "aggregated_at": chrono::Utc::now().to_rfc3339(),
        }
    }))
}

async fn aggregate_alerts(
    State(_state): State<Arc<AppState>>,
    Query(params): Query<OverviewParams>,
) -> Response {
    let limit = params.limit.min(50);
    json_response(serde_json::json!({
        "code": 0,
        "data": {
            "alerts": [],
            "total": 0,
            "limit": limit,
            "note": "Alert aggregation requires Eventide integration",
        }
    }))
}

fn proxy_any_method() -> axum::routing::MethodRouter<Arc<AppState>> {
    axum::routing::MethodRouter::new()
        .get(proxy_request)
        .post(proxy_request)
        .put(proxy_request)
        .delete(proxy_request)
        .patch(proxy_request)
        .options(proxy_request)
        .head(proxy_request)
}
