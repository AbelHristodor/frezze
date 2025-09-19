//! Configuration module for user permissions and roles.
//!
//! This module provides functionality to load and manage user permissions from
//! YAML configuration files. This allows administrators to define which users
//! have access to which commands without modifying the database directly.

use std::{collections::HashMap, path::Path};

use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use tracing::{info, warn};

use crate::database::models::Role;

/// Configuration for user permissions loaded from YAML file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPermissionsConfig {
    /// Map of installation ID to installation-specific permissions
    pub installations: HashMap<String, InstallationConfig>,
}

/// Configuration for a specific GitHub App installation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationConfig {
    /// Installation ID (as string for YAML compatibility)
    pub installation_id: String,
    /// Default permissions for users not explicitly listed
    #[serde(default)]
    pub default_permissions: Option<UserPermissions>,
    /// Repository-specific permissions
    #[serde(default)]
    pub repositories: HashMap<String, RepositoryConfig>,
    /// Global users that apply to all repositories in this installation
    #[serde(default)]
    pub global_users: HashMap<String, UserPermissions>,
}

/// Configuration for a specific repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryConfig {
    /// Repository name in "owner/repo" format
    pub repository: String,
    /// Users with specific permissions for this repository
    pub users: HashMap<String, UserPermissions>,
}

/// User permissions configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPermissions {
    /// User's role (admin, maintainer, contributor)
    pub role: String,
    /// Whether user can freeze repositories
    #[serde(default)]
    pub can_freeze: bool,
    /// Whether user can unfreeze repositories
    #[serde(default)]
    pub can_unfreeze: bool,
    /// Whether user can override freezes in emergencies
    #[serde(default)]
    pub can_emergency_override: bool,
}

impl UserPermissions {
    /// Creates default admin permissions.
    pub fn admin() -> Self {
        Self {
            role: "admin".to_string(),
            can_freeze: true,
            can_unfreeze: true,
            can_emergency_override: true,
        }
    }

    /// Creates default maintainer permissions.
    pub fn maintainer() -> Self {
        Self {
            role: "maintainer".to_string(),
            can_freeze: true,
            can_unfreeze: true,
            can_emergency_override: false,
        }
    }

    /// Creates default contributor permissions.
    pub fn contributor() -> Self {
        Self {
            role: "contributor".to_string(),
            can_freeze: false,
            can_unfreeze: false,
            can_emergency_override: false,
        }
    }

    /// Converts to database Role enum.
    pub fn to_role(&self) -> Result<Role> {
        match self.role.as_str() {
            "admin" => Ok(Role::Admin),
            "maintainer" => Ok(Role::Maintainer),
            "contributor" => Ok(Role::Contributor),
            _ => Err(anyhow!("Unknown role: {}", self.role)),
        }
    }
}

impl Default for UserPermissions {
    fn default() -> Self {
        Self::contributor()
    }
}

impl UserPermissionsConfig {
    /// Loads user permissions from a YAML file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the YAML configuration file
    ///
    /// # Returns
    ///
    /// Returns the loaded configuration or an error if the file cannot be read or parsed.
    ///
    /// # Example YAML format
    ///
    /// ```yaml
    /// installations:
    ///   "12345":
    ///     installation_id: "12345"
    ///     default_permissions:
    ///       role: contributor
    ///       can_freeze: false
    ///       can_unfreeze: false
    ///     global_users:
    ///       admin_user:
    ///         role: admin
    ///         can_freeze: true
    ///         can_unfreeze: true
    ///         can_emergency_override: true
    ///     repositories:
    ///       "owner/repo":
    ///         repository: "owner/repo"
    ///         users:
    ///           maintainer_user:
    ///             role: maintainer
    ///             can_freeze: true
    ///             can_unfreeze: true
    /// ```
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: UserPermissionsConfig = serde_yaml::from_str(&content)?;
        
        // Validate the configuration
        config.validate()?;
        
        info!("Loaded user permissions configuration with {} installations", 
              config.installations.len());
        
        Ok(config)
    }

    /// Validates the configuration for consistency.
    fn validate(&self) -> Result<()> {
        for (install_key, installation) in &self.installations {
            if install_key != &installation.installation_id {
                return Err(anyhow!(
                    "Installation key '{}' does not match installation_id '{}'",
                    install_key, installation.installation_id
                ));
            }

            // Validate user permissions
            if let Some(ref default_perms) = installation.default_permissions {
                default_perms.to_role()?;
            }

            for (_, user_perms) in &installation.global_users {
                user_perms.to_role()?;
            }

            for (repo_key, repo_config) in &installation.repositories {
                if repo_key != &repo_config.repository {
                    return Err(anyhow!(
                        "Repository key '{}' does not match repository name '{}'",
                        repo_key, repo_config.repository
                    ));
                }

                for (_, user_perms) in &repo_config.users {
                    user_perms.to_role()?;
                }
            }
        }

        Ok(())
    }

    /// Gets user permissions for a specific installation and repository.
    ///
    /// # Arguments
    ///
    /// * `installation_id` - GitHub App installation ID
    /// * `repository` - Repository name in "owner/repo" format
    /// * `user_login` - GitHub username
    ///
    /// # Returns
    ///
    /// Returns the user's permissions or None if not configured.
    pub fn get_user_permissions(
        &self,
        installation_id: i64,
        repository: &str,
        user_login: &str,
    ) -> Option<UserPermissions> {
        let installation_key = installation_id.to_string();
        let installation = self.installations.get(&installation_key)?;

        // Check repository-specific permissions first
        if let Some(repo_config) = installation.repositories.get(repository) {
            if let Some(user_perms) = repo_config.users.get(user_login) {
                return Some(user_perms.clone());
            }
        }

        // Check global users for this installation
        if let Some(user_perms) = installation.global_users.get(user_login) {
            return Some(user_perms.clone());
        }

        // Fall back to default permissions
        installation.default_permissions.clone()
    }

    /// Creates an example configuration file.
    pub fn create_example_config<P: AsRef<Path>>(path: P) -> Result<()> {
        let mut installations = HashMap::new();
        
        let mut repositories = HashMap::new();
        repositories.insert("owner/repo".to_string(), RepositoryConfig {
            repository: "owner/repo".to_string(),
            users: {
                let mut users = HashMap::new();
                users.insert("maintainer_user".to_string(), UserPermissions::maintainer());
                users.insert("contributor_user".to_string(), UserPermissions::contributor());
                users
            },
        });

        let mut global_users = HashMap::new();
        global_users.insert("admin_user".to_string(), UserPermissions::admin());

        installations.insert("12345".to_string(), InstallationConfig {
            installation_id: "12345".to_string(),
            default_permissions: Some(UserPermissions::contributor()),
            repositories,
            global_users,
        });

        let config = UserPermissionsConfig { installations };

        let yaml_content = serde_yaml::to_string(&config)?;
        std::fs::write(path, yaml_content)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_user_permissions_to_role() {
        assert!(matches!(UserPermissions::admin().to_role().unwrap(), Role::Admin));
        assert!(matches!(UserPermissions::maintainer().to_role().unwrap(), Role::Maintainer));
        assert!(matches!(UserPermissions::contributor().to_role().unwrap(), Role::Contributor));
    }

    #[test]
    fn test_load_example_config() {
        let temp_file = NamedTempFile::new().unwrap();
        
        // Create example config
        UserPermissionsConfig::create_example_config(temp_file.path()).unwrap();
        
        // Load it back
        let config = UserPermissionsConfig::load_from_file(temp_file.path()).unwrap();
        
        assert_eq!(config.installations.len(), 1);
        assert!(config.installations.contains_key("12345"));
        
        let installation = &config.installations["12345"];
        assert_eq!(installation.installation_id, "12345");
        assert!(installation.default_permissions.is_some());
        assert_eq!(installation.global_users.len(), 1);
        assert!(installation.global_users.contains_key("admin_user"));
    }

    #[test]
    fn test_get_user_permissions() {
        let temp_file = NamedTempFile::new().unwrap();
        UserPermissionsConfig::create_example_config(temp_file.path()).unwrap();
        let config = UserPermissionsConfig::load_from_file(temp_file.path()).unwrap();

        // Test global admin user
        let admin_perms = config.get_user_permissions(12345, "owner/repo", "admin_user").unwrap();
        assert_eq!(admin_perms.role, "admin");
        assert!(admin_perms.can_freeze);
        assert!(admin_perms.can_unfreeze);
        assert!(admin_perms.can_emergency_override);

        // Test repository-specific maintainer
        let maintainer_perms = config.get_user_permissions(12345, "owner/repo", "maintainer_user").unwrap();
        assert_eq!(maintainer_perms.role, "maintainer");
        assert!(maintainer_perms.can_freeze);
        assert!(maintainer_perms.can_unfreeze);
        assert!(!maintainer_perms.can_emergency_override);

        // Test default permissions for unknown user
        let default_perms = config.get_user_permissions(12345, "owner/repo", "unknown_user").unwrap();
        assert_eq!(default_perms.role, "contributor");
        assert!(!default_perms.can_freeze);
        assert!(!default_perms.can_unfreeze);

        // Test unknown installation
        assert!(config.get_user_permissions(99999, "owner/repo", "admin_user").is_none());
    }
}