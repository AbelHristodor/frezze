use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    Admin,
    Maintainer,
    Contributor,
}
impl From<&str> for Role {
    fn from(role: &str) -> Self {
        match role {
            "admin" => Role::Admin,
            "maintainer" => Role::Maintainer,
            "contributor" => Role::Contributor,
            _ => panic!("Unknown role: {role}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FreezeStatus {
    Active,
    Expired,
    Ended,
}

impl From<&str> for FreezeStatus {
    fn from(status: &str) -> Self {
        match status {
            "active" => FreezeStatus::Active,
            "expired" => FreezeStatus::Expired,
            "ended" => FreezeStatus::Ended,
            _ => panic!("Unknown freeze status: {status}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandResult {
    Success,
    Error,
    Unauthorized,
}

impl From<&str> for CommandResult {
    fn from(result: &str) -> Self {
        match result {
            "success" => CommandResult::Success,
            "error" => CommandResult::Error,
            "unauthorized" => CommandResult::Unauthorized,
            _ => panic!("Unknown command result: {result}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FreezeRecord {
    pub id: Uuid,
    pub repository: String,
    pub installation_id: i64,
    pub started_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub reason: Option<String>,
    pub initiated_by: String,
    pub ended_by: Option<String>,
    pub status: FreezeStatus, // active, expired, ended
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PermissionRecord {
    pub id: Uuid,
    pub installation_id: i64,
    pub repository: String,
    pub user_login: String,
    pub role: Role, // admin, maintainer, contributor
    pub can_freeze: bool,
    pub can_unfreeze: bool,
    pub can_emergency_override: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CommandLog {
    pub id: Uuid,
    pub installation_id: i64,
    pub repository: String,
    pub user_login: String,
    pub command: String,
    pub comment_id: i64,
    pub result: CommandResult, // success, error, unauthorized
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}
