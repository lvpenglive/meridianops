use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use std::fmt;

/// 统一错误类型。错误响应体格式：`{ "code": <http-ish>, "message": "..." }`
/// 成功响应保持现有 `{ "code": 0, "data": ... }` 风格（在 routes.rs / auth_routes.rs 内手动构造）。
#[derive(Debug)]
pub struct AppError {
    pub status: StatusCode,
    pub code: i32,
    pub message: String,
}

impl AppError {
    pub fn unauthorized(msg: &str) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code: 401,
            message: msg.to_string(),
        }
    }
    pub fn forbidden(msg: &str) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            code: 403,
            message: msg.to_string(),
        }
    }
    pub fn bad(msg: &str) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: 400,
            message: msg.to_string(),
        }
    }
    pub fn not_found(msg: &str) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: 404,
            message: msg.to_string(),
        }
    }
    pub fn internal(e: impl fmt::Display) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: 500,
            message: e.to_string(),
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = serde_json::json!({ "code": self.code, "message": self.message });
        (self.status, Json(body)).into_response()
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        Self::internal(e)
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => Self::bad("资源不存在"),
            _ => Self::internal(e),
        }
    }
}
