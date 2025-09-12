use std::fmt::Display;

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

impl Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Admin => write!(f, "admin"),
            Role::Maintainer => write!(f, "maintainer"),
            Role::Contributor => write!(f, "ended"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FreezeStatus {
    Active,
    Expired,
    Ended,
}

impl Display for FreezeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FreezeStatus::Active => write!(f, "active"),
            FreezeStatus::Expired => write!(f, "expired"),
            FreezeStatus::Ended => write!(f, "ended"),
        }
    }
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

impl Display for CommandResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandResult::Success => write!(f, "success"),
            CommandResult::Error => write!(f, "error"),
            CommandResult::Unauthorized => write!(f, "unauthorized"),
        }
    }
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

impl FreezeRecord {
    pub fn new(
        repository: String,
        installation_id: i64,
        started_at: DateTime<Utc>,
        expires_at: Option<DateTime<Utc>>,
        reason: Option<String>,
        initiated_by: String,
    ) -> FreezeRecord {
        FreezeRecord {
            id: Uuid::new_v4(),
            repository,
            installation_id,
            started_at,
            expires_at,
            ended_at: None,
            reason,
            initiated_by,
            ended_by: None,
            status: FreezeStatus::Active, // default to active
            created_at: Utc::now(),
        }
    }
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

impl PermissionRecord {
    pub fn new(
        installation_id: i64,
        repository: String,
        user_login: String,
        role: Role,
        can_freeze: bool,
        can_unfreeze: bool,
        can_emergency_override: bool,
    ) -> PermissionRecord {
        PermissionRecord {
            id: Uuid::new_v4(),
            installation_id,
            repository,
            user_login,
            role,
            can_freeze,
            can_unfreeze,
            can_emergency_override,
            created_at: Utc::now(),
        }
    }
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

impl CommandLog {
    pub fn new(
        installation_id: i64,
        repository: String,
        user_login: String,
        command: String,
        comment_id: i64,
        result: CommandResult,
        error_message: Option<String>,
    ) -> CommandLog {
        CommandLog {
            id: Uuid::new_v4(),
            installation_id,
            repository,
            user_login,
            command,
            comment_id,
            result,
            error_message,
            created_at: Utc::now(),
        }
    }
}
