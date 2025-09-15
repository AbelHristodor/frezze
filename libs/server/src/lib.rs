//! Server library - Contains common server configuration and utilities
//!
//! This library provides shared server configuration and utility types.

use serde::{Deserialize, Serialize};

/// Server configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub address: String,
    pub port: u16,
    pub database_url: String,
    pub migrations_path: String,
    pub gh_app_id: u64,
    pub gh_private_key_path: Option<String>,
    pub gh_private_key_base64: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            address: "0.0.0.0".to_string(),
            port: 3000,
            database_url: "postgresql://frezze:frezze@localhost:5432/frezze".to_string(),
            migrations_path: "migrations".to_string(),
            gh_app_id: 0,
            gh_private_key_path: None,
            gh_private_key_base64: None,
        }
    }
}

/// Common HTTP response utilities
pub mod responses {
    use axum::response::{Json, Response, IntoResponse};
    use axum::http::StatusCode;
    use serde_json::json;

    /// Create a success response
    pub fn success() -> Response {
        (StatusCode::OK, Json(json!({"status": "ok"}))).into_response()
    }

    /// Create an error response
    pub fn error(message: &str) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({
            "status": "error",
            "message": message
        }))).into_response()
    }
}

/// Common server result type
pub type ServerResult<T> = Result<T, ServerError>;

/// Server error types
#[derive(Debug, Clone)]
pub enum ServerError {
    Configuration(String),
    Database(String),
    GitHub(String),
    Http(String),
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerError::Configuration(msg) => write!(f, "Configuration error: {}", msg),
            ServerError::Database(msg) => write!(f, "Database error: {}", msg),
            ServerError::GitHub(msg) => write!(f, "GitHub error: {}", msg),
            ServerError::Http(msg) => write!(f, "HTTP error: {}", msg),
        }
    }
}

impl std::error::Error for ServerError {}