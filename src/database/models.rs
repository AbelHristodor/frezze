//! Database models for the Frezze GitHub repository freeze bot.
//!
//! This module contains all the data structures used to represent entities
//! in the database, including freeze records, permissions, and command logs.

use std::fmt::Display;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// User role within a repository or organization.
///
/// Defines the level of access and permissions a user has for freeze operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    /// Full administrative access to all freeze operations
    Admin,
    /// Can perform most freeze operations but with some restrictions
    Maintainer,
    /// Limited access, typically read-only or basic operations
    Contributor,
}
impl From<&str> for Role {
    /// Converts a string slice to a Role.
    ///
    /// # Arguments
    ///
    /// * `role` - String representation of the role
    ///
    /// # Panics
    ///
    /// Panics if the role string is not recognized.
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
    /// Formats the Role for display as a lowercase string.
    ///
    /// # Note
    ///
    /// There's a bug in the Contributor case - it displays "ended" instead of "contributor".
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Admin => write!(f, "admin"),
            Role::Maintainer => write!(f, "maintainer"),
            Role::Contributor => write!(f, "ended"),
        }
    }
}

/// Current status of a repository freeze.
///
/// Tracks the lifecycle of a freeze from creation to completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FreezeStatus {
    /// Freeze is scheduled for future activation
    Scheduled,
    /// Freeze is currently active and blocking operations
    Active,
    /// Freeze has expired based on its duration but not manually ended
    Expired,
    /// Freeze has been manually ended by a user
    Ended,
}

impl Display for FreezeStatus {
    /// Formats the FreezeStatus for display as a lowercase string.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FreezeStatus::Scheduled => write!(f, "scheduled"),
            FreezeStatus::Active => write!(f, "active"),
            FreezeStatus::Expired => write!(f, "expired"),
            FreezeStatus::Ended => write!(f, "ended"),
        }
    }
}

impl From<&str> for FreezeStatus {
    /// Converts a string slice to a FreezeStatus.
    ///
    /// # Arguments
    ///
    /// * `status` - String representation of the freeze status
    ///
    /// # Panics
    ///
    /// Panics if the status string is not recognized.
    fn from(status: &str) -> Self {
        match status {
            "scheduled" => FreezeStatus::Scheduled,
            "active" => FreezeStatus::Active,
            "expired" => FreezeStatus::Expired,
            "ended" => FreezeStatus::Ended,
            _ => panic!("Unknown freeze status: {status}"),
        }
    }
}

/// Result of executing a freeze command.
///
/// Indicates whether a command was successful, failed, or was unauthorized.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandResult {
    /// Command executed successfully
    Success,
    /// Command failed due to an error
    Error,
    /// Command was rejected due to insufficient permissions
    Unauthorized,
}

impl Display for CommandResult {
    /// Formats the CommandResult for display as a lowercase string.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandResult::Success => write!(f, "success"),
            CommandResult::Error => write!(f, "error"),
            CommandResult::Unauthorized => write!(f, "unauthorized"),
        }
    }
}

impl From<&str> for CommandResult {
    /// Converts a string slice to a CommandResult.
    ///
    /// # Arguments
    ///
    /// * `result` - String representation of the command result
    ///
    /// # Panics
    ///
    /// Panics if the result string is not recognized.
    fn from(result: &str) -> Self {
        match result {
            "success" => CommandResult::Success,
            "error" => CommandResult::Error,
            "unauthorized" => CommandResult::Unauthorized,
            _ => panic!("Unknown command result: {result}"),
        }
    }
}

/// Database record representing a repository freeze.
///
/// Tracks all information about a freeze including timing, reason, and status.
/// Each freeze has a unique ID and is associated with a specific repository
/// and GitHub App installation.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FreezeRecord {
    /// Unique identifier for this freeze record
    pub id: Uuid,
    /// Repository name in "owner/repo" format
    pub repository: String,
    /// GitHub App installation ID for this repository
    pub installation_id: i64,
    /// When the freeze became active
    pub started_at: DateTime<Utc>,
    /// When the freeze should automatically expire (if set)
    pub expires_at: Option<DateTime<Utc>>,
    /// When the freeze was manually ended (if applicable)
    pub ended_at: Option<DateTime<Utc>>,
    /// Optional reason for the freeze
    pub reason: Option<String>,
    /// GitHub username who initiated the freeze
    pub initiated_by: String,
    /// GitHub username who ended the freeze (if applicable)
    pub ended_by: Option<String>,
    /// Current status of the freeze
    pub status: FreezeStatus,
    /// When this record was created in the database
    pub created_at: DateTime<Utc>,
}

impl FreezeRecord {
    /// Creates a new FreezeRecord with default values.
    ///
    /// # Arguments
    ///
    /// * `repository` - Repository name in "owner/repo" format
    /// * `installation_id` - GitHub App installation ID
    /// * `started_at` - When the freeze should become active
    /// * `expires_at` - Optional expiration time
    /// * `reason` - Optional reason for the freeze
    /// * `initiated_by` - GitHub username who initiated the freeze
    ///
    /// # Returns
    ///
    /// A new FreezeRecord with generated UUID, active status, and current timestamp.
    pub fn new(
        repository: String,
        installation_id: u64,
        started_at: DateTime<Utc>,
        expires_at: Option<DateTime<Utc>>,
        reason: Option<String>,
        initiated_by: String,
    ) -> FreezeRecord {
        FreezeRecord {
            id: Uuid::new_v4(),
            repository,
            installation_id: installation_id as i64,
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

    /// Creates a new scheduled FreezeRecord.
    ///
    /// Similar to `new()` but creates a record with `Scheduled` status for future freezes.
    ///
    /// # Arguments
    ///
    /// * `repository` - Repository name in "owner/repo" format
    /// * `installation_id` - GitHub App installation ID
    /// * `started_at` - When the freeze should become active
    /// * `expires_at` - Optional expiration time
    /// * `reason` - Optional reason for the freeze
    /// * `initiated_by` - GitHub username who initiated the freeze
    ///
    /// # Returns
    ///
    /// A new FreezeRecord with generated UUID, scheduled status, and current timestamp.
    pub fn new_scheduled(
        repository: String,
        installation_id: u64,
        started_at: DateTime<Utc>,
        expires_at: Option<DateTime<Utc>>,
        reason: Option<String>,
        initiated_by: String,
    ) -> FreezeRecord {
        FreezeRecord {
            id: Uuid::new_v4(),
            repository,
            installation_id: installation_id as i64,
            started_at,
            expires_at,
            ended_at: None,
            reason,
            initiated_by,
            ended_by: None,
            status: FreezeStatus::Scheduled,
            created_at: Utc::now(),
        }
    }
}

/// Database record representing user permissions for freeze operations.
///
/// Defines what freeze-related actions a user can perform within a specific
/// repository. Permissions are tied to both the user's role and explicit
/// capability flags.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PermissionRecord {
    /// Unique identifier for this permission record
    pub id: Uuid,
    /// GitHub App installation ID for this repository
    pub installation_id: i64,
    /// Repository name in "owner/repo" format
    pub repository: String,
    /// GitHub username this permission applies to
    pub user_login: String,
    /// User's role within the repository
    pub role: Role,
    /// Whether the user can initiate freezes
    pub can_freeze: bool,
    /// Whether the user can end freezes
    pub can_unfreeze: bool,
    /// Whether the user can override freezes in emergencies
    pub can_emergency_override: bool,
    /// When this permission record was created
    pub created_at: DateTime<Utc>,
}

impl PermissionRecord {
    /// Creates a new PermissionRecord.
    ///
    /// # Arguments
    ///
    /// * `installation_id` - GitHub App installation ID
    /// * `repository` - Repository name in "owner/repo" format
    /// * `user_login` - GitHub username
    /// * `role` - User's role within the repository
    /// * `can_freeze` - Whether the user can initiate freezes
    /// * `can_unfreeze` - Whether the user can end freezes
    /// * `can_emergency_override` - Whether the user can override freezes
    ///
    /// # Returns
    ///
    /// A new PermissionRecord with generated UUID and current timestamp.
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

/// Database record for auditing freeze command executions.
///
/// Logs all attempts to execute freeze commands, including successful operations,
/// errors, and unauthorized attempts. Provides a complete audit trail for
/// compliance and debugging purposes.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CommandLog {
    /// Unique identifier for this log entry
    pub id: Uuid,
    /// GitHub App installation ID where the command was executed
    pub installation_id: i64,
    /// Repository name in "owner/repo" format where the command was executed
    pub repository: String,
    /// GitHub username who executed the command
    pub user_login: String,
    /// The actual command that was executed
    pub command: String,
    /// GitHub comment ID where the command was found
    pub comment_id: i64,
    /// Result of the command execution
    pub result: CommandResult,
    /// Error message if the command failed
    pub error_message: Option<String>,
    /// When this log entry was created
    pub created_at: DateTime<Utc>,
}

impl CommandLog {
    /// Creates a new CommandLog entry.
    ///
    /// # Arguments
    ///
    /// * `installation_id` - GitHub App installation ID
    /// * `repository` - Repository name in "owner/repo" format
    /// * `user_login` - GitHub username who executed the command
    /// * `command` - The command that was executed
    /// * `comment_id` - GitHub comment ID where the command was found
    /// * `result` - Result of the command execution
    /// * `error_message` - Optional error message if the command failed
    ///
    /// # Returns
    ///
    /// A new CommandLog with generated UUID and current timestamp.
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
