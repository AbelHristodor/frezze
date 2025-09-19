//! Permission checking and authorization module.
//!
//! This module provides functionality to check user permissions for executing
//! freeze commands based on their roles and repository-specific permissions.

use std::sync::Arc;

use crate::{
    database::{Database, models::{PermissionRecord, Role}},
    freezer::commands::Command,
    config::UserPermissionsConfig,
};
use anyhow::Result;
use tracing::{debug, warn};

/// Service for checking user permissions for command execution.
#[derive(Debug, Clone)]
pub struct PermissionService {
    database: Arc<Database>,
    user_config: Option<Arc<UserPermissionsConfig>>,
}

/// Result of a permission check.
#[derive(Debug, Clone, PartialEq)]
pub enum PermissionResult {
    /// Permission granted
    Allowed,
    /// Permission denied with reason
    Denied(String),
}

impl PermissionService {
    /// Creates a new permission service.
    pub fn new(database: Arc<Database>) -> Self {
        Self { 
            database,
            user_config: None,
        }
    }

    /// Creates a new permission service with user configuration.
    pub fn with_config(database: Arc<Database>, user_config: Option<Arc<UserPermissionsConfig>>) -> Self {
        Self { 
            database,
            user_config,
        }
    }

    /// Checks if a user has permission to execute a command.
    ///
    /// # Arguments
    ///
    /// * `installation_id` - GitHub App installation ID
    /// * `repository` - Repository name in "owner/repo" format
    /// * `user_login` - GitHub username
    /// * `command` - Command to check permission for
    ///
    /// # Returns
    ///
    /// Returns `PermissionResult::Allowed` if permission is granted,
    /// or `PermissionResult::Denied` with a reason if denied.
    pub async fn check_permission(
        &self,
        installation_id: i64,
        repository: &str,
        user_login: &str,
        command: &Command,
    ) -> Result<PermissionResult> {
        debug!(
            "Checking permission for user {} to execute {:?} on repository {}",
            user_login, command, repository
        );

        // First, try to get permissions from configuration file
        if let Some(ref config) = self.user_config {
            if let Some(user_perms) = config.get_user_permissions(installation_id, repository, user_login) {
                debug!("Found user permissions in configuration file for user {}", user_login);
                
                // Create a temporary permission record for checking
                let permission_record = PermissionRecord {
                    id: uuid::Uuid::new_v4(),
                    installation_id,
                    repository: repository.to_string(),
                    user_login: user_login.to_string(),
                    role: user_perms.to_role()?,
                    can_freeze: user_perms.can_freeze,
                    can_unfreeze: user_perms.can_unfreeze,
                    can_emergency_override: user_perms.can_emergency_override,
                    created_at: chrono::Utc::now(),
                };

                return Ok(self.check_command_permission(&permission_record, command));
            }
            
            debug!("User {} not found in configuration file, falling back to database", user_login);
        }

        // Fall back to database lookup
        let permission_record = match PermissionRecord::get_by_user_and_repo(
            self.database.pool(),
            installation_id,
            repository,
            user_login,
        ).await? {
            Some(record) => record,
            None => {
                warn!(
                    "No permission record found for user {} in repository {}",
                    user_login, repository
                );
                return Ok(PermissionResult::Denied(
                    "No permissions configured for this user in this repository".to_string()
                ));
            }
        };

        Ok(self.check_command_permission(&permission_record, command))
    }

    /// Checks if a permission record allows a specific command.
    fn check_command_permission(&self, permission_record: &PermissionRecord, command: &Command) -> PermissionResult {
        let result = match command {
            Command::Freeze(_) => {
                if self.can_freeze(permission_record) {
                    PermissionResult::Allowed
                } else {
                    PermissionResult::Denied(format!(
                        "User role '{}' does not have freeze permissions",
                        permission_record.role
                    ))
                }
            }
            Command::FreezeAll(_) => {
                if self.can_freeze_all(permission_record) {
                    PermissionResult::Allowed
                } else {
                    PermissionResult::Denied(format!(
                        "User role '{}' does not have freeze-all permissions",
                        permission_record.role
                    ))
                }
            }
            Command::Unfreeze => {
                if self.can_unfreeze(permission_record) {
                    PermissionResult::Allowed
                } else {
                    PermissionResult::Denied(format!(
                        "User role '{}' does not have unfreeze permissions",
                        permission_record.role
                    ))
                }
            }
            Command::UnfreezeAll => {
                if self.can_unfreeze_all(permission_record) {
                    PermissionResult::Allowed
                } else {
                    PermissionResult::Denied(format!(
                        "User role '{}' does not have unfreeze-all permissions",
                        permission_record.role
                    ))
                }
            }
            Command::Status(_) => {
                if self.can_view_status(permission_record) {
                    PermissionResult::Allowed
                } else {
                    PermissionResult::Denied(format!(
                        "User role '{}' does not have status viewing permissions",
                        permission_record.role
                    ))
                }
            }
            Command::ScheduleFreeze(_) => {
                if self.can_schedule_freeze(permission_record) {
                    PermissionResult::Allowed
                } else {
                    PermissionResult::Denied(format!(
                        "User role '{}' does not have schedule freeze permissions",
                        permission_record.role
                    ))
                }
            }
        };

        debug!(
            "Permission check result for user {} on command {:?}: {:?}",
            permission_record.user_login, command, result
        );

        result
    }

    /// Checks if user can freeze individual repositories.
    fn can_freeze(&self, permission: &PermissionRecord) -> bool {
        match permission.role {
            Role::Admin => true,
            Role::Maintainer => permission.can_freeze,
            Role::Contributor => false,
        }
    }

    /// Checks if user can freeze all repositories.
    fn can_freeze_all(&self, permission: &PermissionRecord) -> bool {
        match permission.role {
            Role::Admin => true,
            Role::Maintainer => permission.can_freeze,
            Role::Contributor => false,
        }
    }

    /// Checks if user can unfreeze individual repositories.
    fn can_unfreeze(&self, permission: &PermissionRecord) -> bool {
        match permission.role {
            Role::Admin => true,
            Role::Maintainer => permission.can_unfreeze,
            Role::Contributor => false,
        }
    }

    /// Checks if user can unfreeze all repositories.
    fn can_unfreeze_all(&self, permission: &PermissionRecord) -> bool {
        match permission.role {
            Role::Admin => true,
            Role::Maintainer => permission.can_unfreeze,
            Role::Contributor => false,
        }
    }

    /// Checks if user can view status.
    fn can_view_status(&self, _permission: &PermissionRecord) -> bool {
        // All roles can view status
        true
    }

    /// Checks if user can schedule freezes.
    fn can_schedule_freeze(&self, permission: &PermissionRecord) -> bool {
        match permission.role {
            Role::Admin => true,
            Role::Maintainer => permission.can_freeze,
            Role::Contributor => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::models::PermissionRecord;
    use chrono::Utc;
    use uuid::Uuid;

    fn create_test_permission(role: Role, can_freeze: bool, can_unfreeze: bool) -> PermissionRecord {
        PermissionRecord {
            id: Uuid::new_v4(),
            installation_id: 123,
            repository: "owner/repo".to_string(),
            user_login: "testuser".to_string(),
            role,
            can_freeze,
            can_unfreeze,
            can_emergency_override: false,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_admin_permissions() {
        let service = PermissionService::new(Arc::new(Database::new_mock()));
        let permission = create_test_permission(Role::Admin, false, false);

        // Admin should have all permissions regardless of flags
        assert!(service.can_freeze(&permission));
        assert!(service.can_freeze_all(&permission));
        assert!(service.can_unfreeze(&permission));
        assert!(service.can_unfreeze_all(&permission));
        assert!(service.can_view_status(&permission));
        assert!(service.can_schedule_freeze(&permission));
    }

    #[test]
    fn test_maintainer_permissions() {
        let service = PermissionService::new(Arc::new(Database::new_mock()));
        
        // Maintainer with freeze permissions
        let permission_with_freeze = create_test_permission(Role::Maintainer, true, true);
        assert!(service.can_freeze(&permission_with_freeze));
        assert!(service.can_freeze_all(&permission_with_freeze));
        assert!(service.can_unfreeze(&permission_with_freeze));
        assert!(service.can_unfreeze_all(&permission_with_freeze));
        assert!(service.can_view_status(&permission_with_freeze));
        assert!(service.can_schedule_freeze(&permission_with_freeze));

        // Maintainer without freeze permissions
        let permission_without_freeze = create_test_permission(Role::Maintainer, false, false);
        assert!(!service.can_freeze(&permission_without_freeze));
        assert!(!service.can_freeze_all(&permission_without_freeze));
        assert!(!service.can_unfreeze(&permission_without_freeze));
        assert!(!service.can_unfreeze_all(&permission_without_freeze));
        assert!(service.can_view_status(&permission_without_freeze));
        assert!(!service.can_schedule_freeze(&permission_without_freeze));
    }

    #[test]
    fn test_contributor_permissions() {
        let service = PermissionService::new(Arc::new(Database::new_mock()));
        let permission = create_test_permission(Role::Contributor, true, true);

        // Contributor should only have status permissions
        assert!(!service.can_freeze(&permission));
        assert!(!service.can_freeze_all(&permission));
        assert!(!service.can_unfreeze(&permission));
        assert!(!service.can_unfreeze_all(&permission));
        assert!(service.can_view_status(&permission));
        assert!(!service.can_schedule_freeze(&permission));
    }
}