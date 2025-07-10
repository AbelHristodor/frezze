use crate::database::Database;
use crate::github::Github;
use anyhow::{Result, anyhow};
use tracing::info;

#[derive(Debug, Default)]
pub struct FreezeProtectionSettings {
    pub required_status_checks: Vec<String>,
}

pub struct FreezerManager {
    pub github: Github,
    pub db: Database,
}

impl FreezerManager {
    pub fn new(github: Github, db: Database) -> Self {
        FreezerManager { github, db }
    }

    pub async fn setup_freeze_protection_rules(&self, installation_id: u64) -> Result<()> {
        info!(
            "Setting up freeze protection rules for installation {}",
            installation_id
        );

        // Fetch all repositories for the installation
        let repos = self
            .github
            .get_installation_repositories(installation_id)
            .await?;
        if repos.is_empty() {
            return Err(anyhow!(
                "No repositories found for installation {}",
                installation_id
            ));
        }
        // Iterate over each repository and apply freeze protection rules
        for repo in repos {
            let owner = match repo.owner {
                Some(owner) => owner.login,
                None => {
                    return Err(anyhow!("Repository {} has no owner", repo.name));
                }
            };
            let repo_name = format!("{}/{}", owner, repo.name);
            info!("Applying freeze protection to repository: {}", repo_name);
        }

        Ok(())
    }

    pub async fn create_freeze_protection(
        &self,
        installation_id: u64,
        owner: &str,
        repo: &str,
    ) -> Result<()> {
        let settings = FreezeProtectionSettings::default();
        let existing_rules = self.github.client();

        Ok(())
    }
}
