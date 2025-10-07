//! Permission checking and authorization module.
//!
//! This module provides functionality to check user permissions for executing
//! freeze commands based on their roles and repository-specific permissions loaded
//! from YAML configuration files.
//!
//! The permission system operates on a role-based access control model with three roles:
//! - **Admin**: Full access to all freeze operations
//! - **Maintainer**: Access to freeze/unfreeze operations based on permission flags
//! - **Contributor**: Read-only access (status commands only)
//!
//! # Examples
//!
//! ```rust
//! use std::sync::Arc;
//! use frezze::permissions::{PermissionService, PermissionResult};
//! use frezze::config::UserPermissionsConfig;
//! use frezze::freezer::commands::Command;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let config = UserPermissionsConfig::load_from_file("permissions.yaml")?;
//! let service = PermissionService::new(Arc::new(config));
//!
//! let result = service.check_permission(
//!     12345,
//!     "owner/repo",
//!     "username",
//!     &Command::Freeze(Default::default()),
//! ).await?;
//!
//! match result {
//!     PermissionResult::Allowed => println!("Permission granted"),
//!     PermissionResult::Denied(reason) => println!("Permission denied: {}", reason),
//! }
//! # Ok(())
//! # }
//! ```

use std::sync::Arc;

use crate::{
    config::{UserPermissions, UserPermissionsConfig},
    database::models::Role,
    freezer::commands::Command,
};
use anyhow::Result;
use tracing::{debug, warn};

/// Service for checking user permissions for command execution.
///
/// This service uses YAML configuration as the single source of truth for user permissions.
/// It evaluates permissions based on a hierarchical system: repository-specific permissions
/// take precedence over global permissions, which take precedence over default permissions.
#[derive(Debug, Clone)]
pub struct PermissionService {
    /// User permissions configuration loaded from YAML
    user_config: Arc<UserPermissionsConfig>,
}

/// Result of a permission check operation.
///
/// This enum represents the outcome of checking whether a user has permission
/// to execute a specific command.
#[derive(Debug, Clone, PartialEq)]
pub enum PermissionResult {
    /// Permission is granted - the user can execute the command
    Allowed,
    /// Permission is denied with a specific reason
    Denied(String),
}

impl PermissionService {
    /// Creates a new permission service with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `user_config` - The user permissions configuration loaded from YAML
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::sync::Arc;
    /// use frezze::permissions::PermissionService;
    /// use frezze::config::UserPermissionsConfig;
    ///
    /// # fn example() -> anyhow::Result<()> {
    /// let config = UserPermissionsConfig::load_from_file("permissions.yaml")?;
    /// let service = PermissionService::new(Arc::new(config));
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(user_config: Arc<UserPermissionsConfig>) -> Self {
        Self { user_config }
    }

    /// Check if user has admin role
    fn is_admin(&self, role: &Role) -> bool {
        matches!(role, Role::Admin)
    }

    /// Checks if a user has permission to execute a specific command.
    ///
    /// This method evaluates permissions using the hierarchical system:
    /// 1. Repository-specific user permissions (highest priority)
    /// 2. Global user permissions for the installation
    /// 3. Default permissions for the installation
    /// 4. Denied (if no configuration found)
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
    /// Returns `Ok(PermissionResult::Allowed)` if permission is granted,
    /// `Ok(PermissionResult::Denied(reason))` if denied, or an error if
    /// the permission check fails due to configuration issues.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use std::sync::Arc;
    /// # use frezze::permissions::{PermissionService, PermissionResult};
    /// # use frezze::config::UserPermissionsConfig;
    /// # use frezze::freezer::commands::Command;
    /// # async fn example() -> anyhow::Result<()> {
    /// let config = UserPermissionsConfig::load_from_file("permissions.yaml")?;
    /// let service = PermissionService::new(Arc::new(config));
    ///
    /// let result = service.check_permission(
    ///     12345,
    ///     "owner/repo",
    ///     "admin_user",
    ///     &Command::Freeze(Default::default()),
    /// ).await?;
    ///
    /// assert_eq!(result, PermissionResult::Allowed);
    /// # Ok(())
    /// # }
    /// ```
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

        // Get user permissions from configuration
        let user_permissions = match self.user_config.get_user_permissions(
            installation_id,
            repository,
            user_login,
        ) {
            Some(perms) => perms,
            None => {
                warn!(
                    "No permission configuration found for user {} in repository {} (installation {})",
                    user_login, repository, installation_id
                );
                return Ok(PermissionResult::Denied(
                    "No permissions configured for this user in this repository".to_string(),
                ));
            }
        };

        debug!(
            "Found permissions for user {}: {:?}",
            user_login, user_permissions
        );

        // Check command permission based on user role and capabilities
        let result = self.check_command_permission(&user_permissions, command)?;

        debug!(
            "Permission check result for user {} on command {:?}: {:?}",
            user_login, command, result
        );

        Ok(result)
    }

    /// Checks if the given user permissions allow execution of a specific command.
    ///
    /// # Arguments
    ///
    /// * `user_permissions` - The user's permission configuration
    /// * `command` - The command to check permission for
    ///
    /// # Returns
    ///
    /// Returns the permission check result based on the user's role and command type.
    fn check_command_permission(
        &self,
        user_permissions: &UserPermissions,
        command: &Command,
    ) -> Result<PermissionResult> {
        let role = user_permissions.to_role()?;

        let result = match command {
            Command::Freeze(_) => {
                if self.can_freeze(&role, user_permissions) {
                    PermissionResult::Allowed
                } else {
                    PermissionResult::Denied(format!(
                        "User role '{}' does not have freeze permissions",
                        role
                    ))
                }
            }
            Command::FreezeAll(_) => {
                if self.can_freeze_all(&role, user_permissions) {
                    PermissionResult::Allowed
                } else {
                    PermissionResult::Denied(format!(
                        "User role '{}' does not have freeze-all permissions",
                        role
                    ))
                }
            }
            Command::Unfreeze(_) => {
                if self.can_unfreeze(&role, user_permissions) {
                    PermissionResult::Allowed
                } else {
                    PermissionResult::Denied(format!(
                        "User role '{}' does not have unfreeze permissions",
                        role
                    ))
                }
            }
            Command::UnfreezeAll => {
                if self.can_unfreeze_all(&role, user_permissions) {
                    PermissionResult::Allowed
                } else {
                    PermissionResult::Denied(format!(
                        "User role '{}' does not have unfreeze-all permissions",
                        role
                    ))
                }
            }
            Command::Status(_) => {
                if self.can_view_status(&role) {
                    PermissionResult::Allowed
                } else {
                    PermissionResult::Denied(format!(
                        "User role '{}' does not have status viewing permissions",
                        role
                    ))
                }
            }
            Command::ScheduleFreeze(_) => {
                if self.can_schedule_freeze(&role, user_permissions) {
                    PermissionResult::Allowed
                } else {
                    PermissionResult::Denied(format!(
                        "User role '{}' does not have schedule freeze permissions",
                        role
                    ))
                }
            }
            Command::UnlockPr(_) => {
                if self.is_admin(&role) {
                    PermissionResult::Allowed
                } else {
                    PermissionResult::Denied(format!(
                        "User role '{}' does not have unlock pr permissions",
                        role
                    ))
                }
            }
        };

        Ok(result)
    }

    /// Checks if the user can freeze individual repositories.
    ///
    /// # Arguments
    ///
    /// * `role` - The user's role
    /// * `permissions` - The user's specific permissions
    ///
    /// # Returns
    ///
    /// `true` if the user can freeze repositories, `false` otherwise.
    fn can_freeze(&self, role: &Role, permissions: &UserPermissions) -> bool {
        match role {
            Role::Admin => true,
            Role::Maintainer => permissions.can_freeze,
            Role::Contributor => false,
        }
    }

    /// Checks if the user can freeze all repositories.
    ///
    /// # Arguments
    ///
    /// * `role` - The user's role
    /// * `permissions` - The user's specific permissions
    ///
    /// # Returns
    ///
    /// `true` if the user can freeze all repositories, `false` otherwise.
    fn can_freeze_all(&self, role: &Role, permissions: &UserPermissions) -> bool {
        match role {
            Role::Admin => true,
            Role::Maintainer => permissions.can_freeze,
            Role::Contributor => false,
        }
    }

    /// Checks if the user can unfreeze individual repositories.
    ///
    /// # Arguments
    ///
    /// * `role` - The user's role
    /// * `permissions` - The user's specific permissions
    ///
    /// # Returns
    ///
    /// `true` if the user can unfreeze repositories, `false` otherwise.
    fn can_unfreeze(&self, role: &Role, permissions: &UserPermissions) -> bool {
        match role {
            Role::Admin => true,
            Role::Maintainer => permissions.can_unfreeze,
            Role::Contributor => false,
        }
    }

    /// Checks if the user can unfreeze all repositories.
    ///
    /// # Arguments
    ///
    /// * `role` - The user's role
    /// * `permissions` - The user's specific permissions
    ///
    /// # Returns
    ///
    /// `true` if the user can unfreeze all repositories, `false` otherwise.
    fn can_unfreeze_all(&self, role: &Role, permissions: &UserPermissions) -> bool {
        match role {
            Role::Admin => true,
            Role::Maintainer => permissions.can_unfreeze,
            Role::Contributor => false,
        }
    }

    /// Checks if the user can view status.
    ///
    /// All roles can view status information.
    ///
    /// # Arguments
    ///
    /// * `role` - The user's role (unused but kept for consistency)
    ///
    /// # Returns
    ///
    /// Always returns `true` as all users can view status.
    fn can_view_status(&self, _role: &Role) -> bool {
        // All roles can view status
        true
    }

    /// Checks if the user can schedule freezes.
    ///
    /// # Arguments
    ///
    /// * `role` - The user's role
    /// * `permissions` - The user's specific permissions
    ///
    /// # Returns
    ///
    /// `true` if the user can schedule freezes, `false` otherwise.
    fn can_schedule_freeze(&self, role: &Role, permissions: &UserPermissions) -> bool {
        match role {
            Role::Admin => true,
            Role::Maintainer => permissions.can_freeze,
            Role::Contributor => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{self, UserPermissions, UserPermissionsConfig};
    use tempfile::NamedTempFile;

    fn create_test_permissions(
        role: &str,
        can_freeze: bool,
        can_unfreeze: bool,
    ) -> UserPermissions {
        UserPermissions {
            role: role.to_string(),
            can_freeze,
            can_unfreeze,
            can_emergency_override: false,
        }
    }

    #[test]
    fn test_admin_permissions() {
        let service = create_test_service();
        let permissions = create_test_permissions("admin", false, false);

        // Admin should have all permissions regardless of flags
        assert!(service.can_freeze(&Role::Admin, &permissions));
        assert!(service.can_freeze_all(&Role::Admin, &permissions));
        assert!(service.can_unfreeze(&Role::Admin, &permissions));
        assert!(service.can_unfreeze_all(&Role::Admin, &permissions));
        assert!(service.can_view_status(&Role::Admin));
        assert!(service.can_schedule_freeze(&Role::Admin, &permissions));
    }

    #[test]
    fn test_maintainer_permissions() {
        let service = create_test_service();

        // Maintainer with freeze permissions
        let permissions_with_freeze = create_test_permissions("maintainer", true, true);
        assert!(service.can_freeze(&Role::Maintainer, &permissions_with_freeze));
        assert!(service.can_freeze_all(&Role::Maintainer, &permissions_with_freeze));
        assert!(service.can_unfreeze(&Role::Maintainer, &permissions_with_freeze));
        assert!(service.can_unfreeze_all(&Role::Maintainer, &permissions_with_freeze));
        assert!(service.can_view_status(&Role::Maintainer));
        assert!(service.can_schedule_freeze(&Role::Maintainer, &permissions_with_freeze));

        // Maintainer without freeze permissions
        let permissions_without_freeze = create_test_permissions("maintainer", false, false);
        assert!(!service.can_freeze(&Role::Maintainer, &permissions_without_freeze));
        assert!(!service.can_freeze_all(&Role::Maintainer, &permissions_without_freeze));
        assert!(!service.can_unfreeze(&Role::Maintainer, &permissions_without_freeze));
        assert!(!service.can_unfreeze_all(&Role::Maintainer, &permissions_without_freeze));
        assert!(service.can_view_status(&Role::Maintainer));
        assert!(!service.can_schedule_freeze(&Role::Maintainer, &permissions_without_freeze));
    }

    #[test]
    fn test_contributor_permissions() {
        let service = create_test_service();
        let permissions = create_test_permissions("contributor", true, true);

        // Contributor should only have status permissions
        assert!(!service.can_freeze(&Role::Contributor, &permissions));
        assert!(!service.can_freeze_all(&Role::Contributor, &permissions));
        assert!(!service.can_unfreeze(&Role::Contributor, &permissions));
        assert!(!service.can_unfreeze_all(&Role::Contributor, &permissions));
        assert!(service.can_view_status(&Role::Contributor));
        assert!(!service.can_schedule_freeze(&Role::Contributor, &permissions));
    }

    fn create_test_service() -> PermissionService {
        let temp_file = NamedTempFile::new().unwrap();
        config::create_example_config(temp_file.path()).unwrap();
        let config = UserPermissionsConfig::load_from_file(temp_file.path()).unwrap();
        PermissionService::new(Arc::new(config))
    }
}
