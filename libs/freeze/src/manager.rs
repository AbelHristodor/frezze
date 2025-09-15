use std::sync::Arc;

use crate::{
    database::{Database, models::FreezeRecord},
    github::Github,
    repository::Repository,
};
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use octocrab::models::issues::Comment;
use tracing::{error, info, warn};

use super::pr_refresh::PrRefreshService;

pub const DEFAULT_FREEZE_DURATION: chrono::Duration = chrono::Duration::hours(2);

pub struct FreezeManager {
    pub db: Arc<Database>,
    pub github: Arc<Github>,
    pub pr_refresh: PrRefreshService,
}

impl FreezeManager {
    pub fn new(db: Arc<Database>, github: Arc<Github>) -> Self {
        let pr_refresh = PrRefreshService::new(github.clone(), db.clone());
        FreezeManager {
            db,
            github,
            pr_refresh,
        }
    }

    pub async fn notify_comment_issue(
        &self,
        installation_id: i64,
        repository: &Repository,
        issue_nr: u64,
        msg: &str,
    ) -> Result<Comment> {
        // Create response comment
        let comment = self
            .github
            .create_comment(
                installation_id as u64,
                repository.owner(),
                repository.name(),
                issue_nr,
                msg,
            )
            .await?;

        Ok(comment)
    }

    pub async fn freeze(
        &self,
        installation_id: i64,
        repository: &Repository,
        duration: Option<chrono::Duration>,
        reason: Option<String>,
        initiated_by: String,
    ) -> Result<FreezeRecord> {
        // Create the record
        let start = Utc::now();
        let duration = match duration {
            Some(d) => d,
            None => DEFAULT_FREEZE_DURATION,
        };
        let record = FreezeRecord::new(
            repository.full_name(),
            installation_id,
            start,
            Some(start + duration),
            reason,
            initiated_by,
        );

        let conn = self
            .db
            .get_connection()
            .map_err(|e| anyhow!("Failed to get database connection: {}", e))?;

        // Save it to database
        let record = FreezeRecord::create(conn, &record).await?;

        // Refresh PRs after creating freeze
        match self
            .pr_refresh
            .refresh_repository_prs(
                installation_id as u64,
                repository.owner(),
                repository.name(),
                true, // Repository is now frozen
            )
            .await
        {
            Ok(result) => {
                info!(
                    "Successfully updated {} PRs for frozen repository {}",
                    result.successful_updates,
                    repository.full_name()
                );
                if !result.errors.is_empty() {
                    warn!(
                        "Some PR updates failed for {}: {} errors",
                        repository.full_name(),
                        result.errors.len()
                    );
                }
            }
            Err(e) => {
                warn!(
                    "Failed to refresh PRs for repository {}: {}",
                    repository.full_name(),
                    e
                );
                // Don't fail the freeze operation if PR refresh fails
            }
        }

        Ok(record)
    }

    pub async fn list_for_repo(
        &self,
        repository: &Repository,
        installation_id: i64,
        active: Option<bool>,
    ) -> Result<Vec<FreezeRecord>> {
        let conn = self
            .db
            .get_connection()
            .map_err(|e| anyhow!("Failed to get database connection: {}", e))?;

        let repo = repository.full_name();
        let records = FreezeRecord::list(conn, Some(installation_id), Some(&repo), active)
            .await
            .map_err(|e| anyhow!("Failed to list freeze records for repo {}: {}", repo, e))?;

        if records.is_empty() {
            info!("No freeze records found for repository: {}", repo);
            return Err(anyhow!("No freeze records found for repository: {}", repo));
        }
        info!(
            "Found {} freeze records for repository: {}",
            records.len(),
            repo
        );
        Ok(records)
    }

    pub async fn is_frozen(&self, repository: &Repository, installation_id: i64) -> Result<bool> {
        let conn = self
            .db
            .get_connection()
            .map_err(|e| anyhow!("Failed to get database connection: {}", e))?;

        let repo = repository.full_name();
        let frozen = FreezeRecord::is_frozen(conn, installation_id, &repo)
            .await
            .map_err(|e| anyhow!("Failed to check if repository {} is frozen: {}", repo, e))?;

        Ok(frozen)
    }

    /// Manually refresh PRs for all repositories with active freezes
    pub async fn refresh_all_active_freezes(&self) -> Result<()> {
        info!("Starting manual refresh of all active freeze PRs");

        match self.pr_refresh.refresh_all_active_freezes().await {
            Ok(results) => {
                let total_repos = results.len();
                let total_prs: usize = results.values().map(|r| r.successful_updates).sum();
                let total_errors: usize = results.values().map(|r| r.failed_updates).sum();

                info!(
                    "Manual refresh completed: {} repositories, {} PRs updated, {} errors",
                    total_repos, total_prs, total_errors
                );

                if total_errors > 0 {
                    warn!("Some PR updates failed during manual refresh");
                    for (repo, result) in results {
                        if !result.errors.is_empty() {
                            warn!("Errors for repository {}: {:?}", repo, result.errors);
                        }
                    }
                }

                Ok(())
            }
            Err(e) => {
                error!("Manual refresh failed: {}", e);
                Err(e)
            }
        }
    }

    /// Refresh PRs for a specific repository
    pub async fn refresh_repository_prs(&self, installation_id: i64, repo: &str) -> Result<()> {
        info!("Starting manual refresh for repository: {}", repo);

        let parts: Vec<&str> = repo.split('/').collect();
        if parts.len() != 2 {
            return Err(anyhow!(
                "Invalid repository format: {}. Expected format: owner/repo",
                repo
            ));
        }
        let repository = Repository::new(parts[0], parts[1]);

        let is_frozen = self.is_frozen(&repository, installation_id).await?;

        match self
            .pr_refresh
            .refresh_repository_prs(
                installation_id as u64,
                repository.owner(),
                repository.name(),
                is_frozen,
            )
            .await
        {
            Ok(result) => {
                info!(
                    "Repository {} refresh completed: {} PRs updated, {} errors",
                    repo, result.successful_updates, result.failed_updates
                );

                if !result.errors.is_empty() {
                    warn!("Some PR updates failed: {:?}", result.errors);
                }

                Ok(())
            }
            Err(e) => {
                error!("Failed to refresh repository {}: {}", repo, e);
                Err(e)
            }
        }
    }

    pub async fn schedule_freeze(
        &self,
        installation_id: i64,
        repository: &Repository,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        reason: Option<String>,
        initiated_by: String,
    ) -> Result<()> {
        let record = FreezeRecord::new(
            repository.full_name(),
            installation_id,
            start,
            Some(end),
            reason,
            initiated_by,
        );

        let conn = self
            .db
            .get_connection()
            .map_err(|e| anyhow!("Failed to get database connection: {}", e))?;

        FreezeRecord::create(conn, &record).await?;
        Ok(())
    }

    /// Unfreeze a repository
    pub async fn unfreeze(
        &self,
        installation_id: i64,
        repository: &Repository,
        ended_by: String,
    ) -> Result<()> {
        let conn = self
            .db
            .get_connection()
            .map_err(|e| anyhow!("Failed to get database connection: {}", e))?;

        let repo = repository.full_name();
        // Get active freeze records for this repository
        let freeze_records =
            FreezeRecord::list(conn, Some(installation_id), Some(&repo), Some(true))
                .await
                .map_err(|e| anyhow!("Failed to get freeze records for repo {}: {}", repo, e))?;

        if freeze_records.is_empty() {
            return Err(anyhow!("No active freeze found for repository: {}", repo));
        }

        // End all active freezes for this repository
        for record in freeze_records {
            FreezeRecord::update_status(
                conn,
                record.id,
                crate::database::models::FreezeStatus::Ended,
                Some(ended_by.clone()),
            )
            .await
            .map_err(|e| anyhow!("Failed to end freeze record {}: {}", record.id, e))?;
        }

        // Refresh PRs after unfreezing
        match self
            .pr_refresh
            .refresh_repository_prs(
                installation_id as u64,
                repository.owner(),
                repository.name(),
                false, // Repository is now unfrozen
            )
            .await
        {
            Ok(result) => {
                info!(
                    "Successfully updated {} PRs for unfrozen repository {}",
                    result.successful_updates,
                    repository.full_name()
                );
                if !result.errors.is_empty() {
                    warn!(
                        "Some PR updates failed for {}: {} errors",
                        repository.full_name(),
                        result.errors.len()
                    );
                }
            }
            Err(e) => {
                warn!(
                    "Failed to refresh PRs for repository {}: {}",
                    repository.full_name(),
                    e
                );
                // Don't fail the unfreeze operation if PR refresh fails
            }
        }

        Ok(())
    }
}
