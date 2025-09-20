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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Admin => write!(f, "admin"),
            Role::Maintainer => write!(f, "maintainer"),
            Role::Contributor => write!(f, "contributor"),
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
