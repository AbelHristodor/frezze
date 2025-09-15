//! Freeze library - Contains freeze-related constants and common functionality
//!
//! This library provides shared constants and utilities for freeze operations.

/// Default freeze duration when none is specified
pub const DEFAULT_FREEZE_DURATION: chrono::Duration = chrono::Duration::hours(2);

/// Trait for objects that can represent a repository
pub trait RepositoryLike {
    fn full_name(&self) -> String;
    fn owner(&self) -> &str;
    fn name(&self) -> &str;
}

/// Common result type for freeze operations
pub type FreezeResult<T> = Result<T, FreezeError>;

/// Error types for freeze operations
#[derive(Debug, Clone)]
pub enum FreezeError {
    DatabaseError(String),
    GitHubError(String),
    ValidationError(String),
    NotFound(String),
}

impl std::fmt::Display for FreezeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FreezeError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            FreezeError::GitHubError(msg) => write!(f, "GitHub error: {}", msg),
            FreezeError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            FreezeError::NotFound(msg) => write!(f, "Not found: {}", msg),
        }
    }
}

impl std::error::Error for FreezeError {}