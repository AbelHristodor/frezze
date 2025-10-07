use std::sync::Arc;

use crate::{
    database::{
        Database,
        models::{FreezeRecord, UnlockedPr},
    },
    freezer::messages,
    repository::Repository,
};
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use octofer::github::GitHubClient;
use tracing::{error, info, warn};

use super::pr_refresh::PrRefreshService;

pub const DEFAULT_FREEZE_DURATION: chrono::Duration = chrono::Duration::hours(2);

#[derive(Debug)]
pub struct StatusEntry {
    pub freeze_status: FreezeStatus,
    pub duration: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug)]
pub enum FreezeStatus {
    Active,
    Scheduled,
    Off,
    Error(String),
}

impl StatusEntry {
    pub fn not_frozen() -> Self {
        StatusEntry {
            freeze_status: FreezeStatus::Off,
            duration: None,
            start: None,
            end: None,
            reason: None,
        }
    }

    pub fn frozen(record: &FreezeRecord) -> Self {
        let duration = record
            .expires_at
            .map(|expires_at| messages::format_duration_display(expires_at - record.started_at));

        let start = Some(
            record
                .started_at
                .format("%Y-%m-%d %H:%M:%S UTC")
                .to_string(),
        );
        let end = record
            .expires_at
            .map(|e| e.format("%Y-%m-%d %H:%M:%S UTC").to_string());

        StatusEntry {
            freeze_status: if record.started_at <= Utc::now() {
                FreezeStatus::Active
            } else {
                FreezeStatus::Scheduled
            },
            duration,
            start,
            end,
            reason: record.reason.clone(),
        }
    }

    pub fn error(msg: &str) -> Self {
        StatusEntry {
            freeze_status: FreezeStatus::Error(msg.to_string()),
            duration: None,
            start: None,
            end: None,
            reason: None,
        }
    }
}

pub struct FreezeManager {
    pub db: Arc<Database>,
    pub github: Arc<GitHubClient>,
    pub pr_refresh: PrRefreshService,
}

impl FreezeManager {
    pub fn new(db: Arc<Database>, github: Arc<GitHubClient>) -> Self {
        let pr_refresh = PrRefreshService::new(github.clone(), db.clone());
        FreezeManager {
            db,
            github,
            pr_refresh,
        }
    }

    pub async fn notify_comment_issue(
        &self,
        installation_id: u64,
        repository: &Repository,
        issue_nr: u64,
        msg: &str,
    ) {
        // Create response comment
        let error = self
            .github
            .with_installation_async(installation_id, async move |c| {
                let repo = repository.clone();
                c.issues(repo.owner, repo.name)
                    .create_comment(issue_nr, msg)
                    .await
                    .map_err(|e| anyhow::anyhow!("Error: {:?}", e))
            })
            .await
            .err();

        if let Some(err) = error {
            error!("Unable do send comment: {:?}", err);
        }
    }

    pub async fn freeze(
        &self,
        installation_id: u64,
        repository: &Repository,
        duration: Option<chrono::Duration>,
        reason: Option<String>,
        initiated_by: String,
        issue_nr: u64,
        repos: Vec<String>,
    ) {
        // If repos are specified, this is a multi-repo freeze command
        if !repos.is_empty() {
            self.freeze_repos(
                installation_id,
                duration,
                reason,
                initiated_by,
                issue_nr,
                repos,
            )
            .await;
            return;
        }

        // Otherwise, freeze the current repository
        let outcome = match self
            .handle_freeze(installation_id, repository, duration, reason, initiated_by)
            .await
        {
            Ok(r) => {
                let duration = if let Some(d) = r.expires_at {
                    d - r.started_at
                } else {
                    DEFAULT_FREEZE_DURATION
                };

                let duration_str = messages::format_duration_display(duration);
                let reason_str = messages::format_reason_display(r.reason);
                messages::freeze_success(&repository.to_string(), &duration_str, &reason_str)
            }
            Err(e) => messages::freeze_error(&e.to_string()),
        };

        self.notify_comment_issue(installation_id, repository, issue_nr, &outcome)
            .await;
    }

    async fn handle_freeze(
        &self,
        installation_id: u64,
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
                installation_id,
                repository.owner(),
                repository.name(),
                Some(&record), // Pass the created freeze record
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

    pub async fn freeze_all(
        &self,
        installation_id: u64,
        duration: Option<chrono::Duration>,
        reason: Option<String>,
        initiated_by: String,
        issue_nr: u64,
        repos: Vec<String>,
    ) {
        // If specific repos are provided, filter to those repos only
        if !repos.is_empty() {
            self.freeze_repos(
                installation_id,
                duration,
                reason,
                initiated_by,
                issue_nr,
                repos,
            )
            .await;
            return;
        }

        // Get all repositories for this installation
        let repositories = match self.get_installation_repositories(installation_id).await {
            Ok(repos) => repos,
            Err(e) => {
                let _ = messages::freeze_error(&format!("Failed to get repositories: {}", e));
                // We don't have a specific repository for this case, so we'll need to create a dummy one
                // This is a limitation of the current architecture
                error!(
                    "Failed to get repositories for installation {}: {}",
                    installation_id, e
                );
                return;
            }
        };

        if repositories.is_empty() {
            let _ = messages::freeze_error("No repositories accessible for this installation");
            error!("No repositories found for installation {}", installation_id);
            return;
        }

        let mut successful_freezes = 0;
        let mut failed_freezes = 0;
        let mut error_messages = Vec::new();

        for repo in &repositories {
            let repository = Repository::new(&repo.owner.as_ref().unwrap().login, &repo.name);

            match self
                .handle_freeze(
                    installation_id,
                    &repository,
                    duration,
                    reason.clone(),
                    initiated_by.clone(),
                )
                .await
            {
                Ok(_) => {
                    successful_freezes += 1;
                    info!("Successfully froze repository: {}", repository.full_name());
                }
                Err(e) => {
                    failed_freezes += 1;
                    let error = format!("Failed to freeze {}: {}", repository.full_name(), e);
                    error_messages.push(error.clone());
                    error!("{}", error);
                }
            }
        }

        let outcome = if failed_freezes == 0 {
            messages::freeze_all_success(successful_freezes)
        } else {
            messages::freeze_all_partial_success(
                successful_freezes,
                failed_freezes,
                &error_messages,
            )
        };

        // For freeze_all, we need to pick a repository to comment on. Let's use the first one
        if let Some(first_repo) = repositories.first() {
            let repository =
                Repository::new(&first_repo.owner.as_ref().unwrap().login, &first_repo.name);
            self.notify_comment_issue(installation_id, &repository, issue_nr, &outcome)
                .await;
        }
    }

    async fn freeze_repos(
        &self,
        installation_id: u64,
        duration: Option<chrono::Duration>,
        reason: Option<String>,
        initiated_by: String,
        issue_nr: u64,
        repo_names: Vec<String>,
    ) {
        let mut successful_freezes = 0;
        let mut failed_freezes = 0;
        let mut error_messages = Vec::new();
        let mut first_repository = None;

        for repo_name in &repo_names {
            // Parse repository name (expecting format "owner/repo" or just "repo")
            let repository = if repo_name.contains('/') {
                let parts: Vec<&str> = repo_name.split('/').collect();
                if parts.len() != 2 {
                    failed_freezes += 1;
                    let error = format!("Invalid repository format: {}", repo_name);
                    error_messages.push(error.clone());
                    error!("{}", error);
                    continue;
                }
                Repository::new(parts[0], parts[1])
            } else {
                // If no owner specified, we can't determine the repository
                failed_freezes += 1;
                let error = format!(
                    "Invalid repository format '{}'. Expected 'owner/repo'",
                    repo_name
                );
                error_messages.push(error.clone());
                error!("{}", error);
                continue;
            };

            if first_repository.is_none() {
                first_repository = Some(repository.clone());
            }

            match self
                .handle_freeze(
                    installation_id,
                    &repository,
                    duration,
                    reason.clone(),
                    initiated_by.clone(),
                )
                .await
            {
                Ok(_) => {
                    successful_freezes += 1;
                    info!("Successfully froze repository: {}", repository.full_name());
                }
                Err(e) => {
                    failed_freezes += 1;
                    let error = format!("Failed to freeze {}: {}", repository.full_name(), e);
                    error_messages.push(error.clone());
                    error!("{}", error);
                }
            }
        }

        let outcome = if failed_freezes == 0 {
            messages::freeze_all_success(successful_freezes)
        } else {
            messages::freeze_all_partial_success(
                successful_freezes,
                failed_freezes,
                &error_messages,
            )
        };

        // Comment on the first repository (or the one that triggered the command)
        if let Some(repository) = first_repository {
            self.notify_comment_issue(installation_id, &repository, issue_nr, &outcome)
                .await;
        }
    }

    pub async fn unfreeze_all(&self, installation_id: u64, ended_by: String, issue_nr: u64) {
        // Get all repositories for this installation
        let repositories = match self.get_installation_repositories(installation_id).await {
            Ok(repos) => repos,
            Err(e) => {
                error!(
                    "Failed to get repositories for installation {}: {}",
                    installation_id, e
                );
                return;
            }
        };

        if repositories.is_empty() {
            error!("No repositories found for installation {}", installation_id);
            return;
        }

        let mut successful_unfreezes = 0;
        let mut failed_unfreezes = 0;
        let mut error_messages = Vec::new();

        for repo in &repositories {
            let repository = Repository::new(&repo.owner.as_ref().unwrap().login, &repo.name);

            match self
                .handle_unfreeze(installation_id, &repository, ended_by.clone())
                .await
            {
                Ok(_) => {
                    successful_unfreezes += 1;
                    info!(
                        "Successfully unfroze repository: {}",
                        repository.full_name()
                    );
                }
                Err(e) => {
                    failed_unfreezes += 1;
                    let error = format!("Failed to unfreeze {}: {}", repository.full_name(), e);
                    error_messages.push(error.clone());
                    error!("{}", error);
                }
            }
        }

        let outcome = if failed_unfreezes == 0 {
            messages::unfreeze_all_success(successful_unfreezes)
        } else {
            messages::unfreeze_all_partial_success(
                successful_unfreezes,
                failed_unfreezes,
                &error_messages,
            )
        };

        // For unfreeze_all, we need to pick a repository to comment on. Let's use the first one
        if let Some(first_repo) = repositories.first() {
            let repository =
                Repository::new(&first_repo.owner.as_ref().unwrap().login, &first_repo.name);
            self.notify_comment_issue(installation_id, &repository, issue_nr, &outcome)
                .await;
        }
    }

    async fn get_installation_repositories(
        &self,
        installation_id: u64,
    ) -> Result<Vec<octocrab::models::Repository>> {
        self.github
            .with_installation_async(installation_id, |client| async move {
                // Use the manual HTTP approach for the installation repositories endpoint
                let url = "/installation/repositories";
                let response: serde_json::Value = client
                    .get(url, None::<&()>)
                    .await
                    .map_err(|e| anyhow!("Failed to get installation repositories: {}", e))?;

                let repositories = response
                    .get("repositories")
                    .and_then(|r| r.as_array())
                    .ok_or_else(|| anyhow!("Invalid response format"))?;

                let mut repos = Vec::new();
                for repo_value in repositories {
                    let repo: octocrab::models::Repository =
                        serde_json::from_value(repo_value.clone())
                            .map_err(|e| anyhow!("Failed to deserialize repository: {}", e))?;
                    repos.push(repo);
                }

                Ok(repos)
            })
            .await
    }

    pub async fn get_status(
        &self,
        installation_id: u64,
        repos: Vec<String>,
        issue_nr: u64,
        repository: &Repository,
    ) {
        let mut status_entries = Vec::new();

        if repos.is_empty() {
            // If no specific repos requested, get all repositories for this installation
            match self.get_installation_repositories(installation_id).await {
                Ok(all_repos) => {
                    for repo in all_repos {
                        let repo_name =
                            format!("{}/{}", repo.owner.as_ref().unwrap().login, repo.name);
                        let repository =
                            Repository::new(&repo.owner.as_ref().unwrap().login, &repo.name);
                        let entry = self
                            .get_repository_status(installation_id, &repository)
                            .await;
                        status_entries.push((repo_name, entry));
                    }
                }
                Err(e) => {
                    let error_msg =
                        messages::status_error(&format!("Failed to get repositories: {}", e));
                    self.notify_comment_issue(installation_id, repository, issue_nr, &error_msg)
                        .await;
                    return;
                }
            }
        } else {
            // Get status for specific repositories
            for repo_name in repos {
                let parts: Vec<&str> = repo_name.split('/').collect();
                if parts.len() != 2 {
                    status_entries.push((
                        repo_name.clone(),
                        StatusEntry::error("Invalid repository format"),
                    ));
                    continue;
                }

                let repository = Repository::new(parts[0], parts[1]);
                let entry = self
                    .get_repository_status(installation_id, &repository)
                    .await;
                status_entries.push((repo_name, entry));
            }
        }

        let status_msg = messages::format_status_table(status_entries);
        self.notify_comment_issue(installation_id, repository, issue_nr, &status_msg)
            .await;
    }

    async fn get_repository_status(
        &self,
        installation_id: u64,
        repository: &Repository,
    ) -> StatusEntry {
        let conn = match self.db.get_connection() {
            Ok(conn) => conn,
            Err(e) => return StatusEntry::error(&format!("Database error: {}", e)),
        };

        match FreezeRecord::list(
            conn,
            Some(installation_id),
            Some(&repository.full_name()),
            Some(true),
        )
        .await
        {
            Ok(records) => {
                if records.is_empty() {
                    StatusEntry::not_frozen()
                } else {
                    // Take the most recent active freeze
                    let record = &records[0];
                    StatusEntry::frozen(record)
                }
            }
            Err(e) => StatusEntry::error(&format!("Failed to get freeze records: {}", e)),
        }
    }

    pub async fn schedule_freeze(
        &self,
        installation_id: u64,
        repository: &Repository,
        start: DateTime<Utc>,
        end: Option<DateTime<Utc>>,
        duration: Option<chrono::Duration>,
        reason: Option<String>,
        initiated_by: String,
    ) -> Result<()> {
        let end_time = match (end, duration) {
            (Some(end), _) => end,
            (None, Some(dur)) => start + dur,
            (None, None) => start + DEFAULT_FREEZE_DURATION,
        };

        let record = FreezeRecord::new_scheduled(
            repository.full_name(),
            installation_id,
            start,
            Some(end_time),
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

    pub async fn unfreeze(
        &self,
        installation_id: u64,
        repository: &Repository,
        ended_by: String,
        reason: Option<String>,
        issue_nr: u64,
    ) {
        let outcome = match self
            .handle_unfreeze(installation_id, repository, ended_by)
            .await
        {
            Ok(_) => {
                let reason_str = messages::format_reason_display(reason);
                messages::unfreeze_success(&repository.to_string(), &reason_str)
            }
            Err(e) => {
                tracing::error!("Failed to unfreeze repository: {:?}", e);
                messages::unfreeze_error(&e.to_string())
            }
        };

        self.notify_comment_issue(installation_id, repository, issue_nr, &outcome)
            .await;
    }

    /// Unfreeze a repository
    async fn handle_unfreeze(
        &self,
        installation_id: u64,
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
            let record_id = record.id.clone();
            FreezeRecord::update_status(
                conn,
                record.id,
                crate::database::models::FreezeStatus::Ended,
                Some(ended_by.clone()),
            )
            .await
            .map_err(|e| anyhow!("Failed to end freeze record {}: {}", record_id, e))?;
        }

        // Refresh PRs after unfreezing
        match self
            .pr_refresh
            .refresh_repository_prs(
                installation_id,
                repository.owner(),
                repository.name(),
                None, // Repository is now unfrozen - no freeze record
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
    /// Get the active freeze record for a repository, if one exists
    async fn get_active_freeze(
        &self,
        repository: &Repository,
        installation_id: i64,
    ) -> Result<Option<FreezeRecord>> {
        let conn = self
            .db
            .get_connection()
            .map_err(|e| anyhow!("Failed to get database connection: {}", e))?;

        let repo = repository.full_name();
        let freeze_record = FreezeRecord::get_active_freeze(conn, installation_id, &repo)
            .await
            .map_err(|e| anyhow!("Failed to get active freeze for repository {}: {}", repo, e))?;

        Ok(freeze_record)
    }

    pub async fn unlock_pr(
        &self,
        installation_id: u64,
        repository: &Repository,
        pr_number: u64,
        author: String,
        reason: Option<String>,
        issue_nr: u64,
    ) {
        let repo_name = repository.full_name();

        // Check if repository is currently frozen
        match self
            .get_active_freeze(repository, installation_id as i64)
            .await
        {
            Ok(Some(_)) => {
                // Repository is frozen, proceed with unlock
                match UnlockedPr::unlock_pr(
                    self.db.pool(),
                    installation_id as i64,
                    &repo_name,
                    pr_number,
                    &author,
                )
                .await
                {
                    Ok(_) => {
                        let reason_str = messages::format_reason_display(reason);
                        let success_msg = messages::pr_unlock_success(pr_number, &reason_str);
                        self.notify_comment_issue(
                            installation_id,
                            repository,
                            issue_nr,
                            &success_msg,
                        )
                        .await;

                        // Refresh the specific PR to update its status
                        if let Err(e) = self
                            .pr_refresh
                            .refresh_single_pr(installation_id as i64, repository, pr_number)
                            .await
                        {
                            error!("Failed to refresh PR {} after unlock: {}", pr_number, e);
                        }
                    }
                    Err(e) => {
                        error!("Failed to unlock PR {}: {}", pr_number, e);
                        let error_msg = messages::pr_unlock_failed(pr_number, &e.to_string());
                        self.notify_comment_issue(
                            installation_id,
                            repository,
                            issue_nr,
                            &error_msg,
                        )
                        .await;
                    }
                }
            }
            Ok(None) => {
                // Repository is not frozen
                let error_msg = messages::pr_unlock_not_frozen(&repo_name);
                self.notify_comment_issue(installation_id, repository, issue_nr, &error_msg)
                    .await;
            }
            Err(e) => {
                error!("Failed to check freeze status for {}: {}", repo_name, e);
                let error_msg = messages::pr_unlock_failed(pr_number, &e.to_string());
                self.notify_comment_issue(installation_id, repository, issue_nr, &error_msg)
                    .await;
            }
        }
    }
}
