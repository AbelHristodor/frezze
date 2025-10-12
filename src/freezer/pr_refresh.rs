//! PR refresh system for updating check runs on all open pull requests.
//!
//! This module provides functionality to efficiently update all open PRs with
//! freeze check runs while respecting GitHub API rate limits.

use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::{Result, anyhow};
use octofer::github::{GitHubClient, models::checks::CheckRun, pulls::PullRequest};
use octofer::octocrab::params::checks::{CheckRunConclusion, CheckRunOutput, CheckRunStatus};
use tracing::{error, info, warn};

use crate::{
    database::{
        Database,
        models::{FreezeRecord, UnlockedPr},
    },
    repository::Repository,
};

const FREZZE_CHECK_RUN_NAME: &str = "Frezze";

/// Format freeze information for check run output
fn format_freeze_details(freeze_record: &FreezeRecord) -> CheckRunOutput {
    let start_time = freeze_record
        .started_at
        .format("%Y-%m-%d %H:%M:%S UTC")
        .to_string();
    let end_time = freeze_record
        .expires_at
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "No end time set".to_string());
    let reason = freeze_record
        .reason
        .as_deref()
        .unwrap_or("No reason provided");
    let author = &freeze_record.initiated_by;
    let branch = freeze_record
        .branch
        .as_ref()
        .map(|b| format!(" for branch '{}'", b))
        .unwrap_or_else(|| " for all branches".to_string());

    let title = format!("Repository is frozen{} by {}", branch, author);
    let summary = format!("This repository is currently under a freeze restriction{}", branch);

    let branch_text = freeze_record
        .branch
        .as_ref()
        .map(|b| format!("- **Branch**: {}\n", b))
        .unwrap_or_else(|| "- **Branch**: All branches\n".to_string());

    let text = format!(
        "**Repository Freeze Details**\n\n\
        {}\
        - **Author**: {}\n\
        - **Start**: {}\n\
        - **End**: {}\n\
        - **Reason**: {}\n\n\
        This PR cannot be merged while the repository is frozen. Please wait for the freeze to end or contact the freeze author.",
        branch_text, author, start_time, end_time, reason
    );

    CheckRunOutput {
        title,
        summary,
        text: Some(text),
        annotations: Vec::new(),
        images: Vec::new(),
    }
}

/// Format success output for unfrozen repository
fn format_success_output() -> CheckRunOutput {
    CheckRunOutput {
        title: "Repository is not frozen".to_string(),
        summary: "This repository is currently not under any freeze restrictions".to_string(),
        text: Some("PRs can be merged normally.".to_string()),
        annotations: Vec::new(),
        images: Vec::new(),
    }
}

/// Information about a pull request needed for check run updates
#[derive(Debug, Clone)]
pub struct PullRequestInfo {
    pub number: u64,
    pub head_sha: String,
    pub base_ref: String,
}

/// Results of a PR refresh operation
#[derive(Debug)]
pub struct RefreshResult {
    pub total_prs: usize,
    pub successful_updates: usize,
    pub failed_updates: usize,
    pub errors: Vec<String>,
}

/// Configuration for PR refresh operations
#[derive(Debug, Clone)]
pub struct RefreshConfig {
    /// Maximum number of concurrent API requests
    pub max_concurrent_requests: usize,
    /// Delay between batches to respect rate limits
    pub batch_delay_ms: u64,
    /// Maximum number of retries per PR
    pub max_retries: usize,
    /// Base delay for exponential backoff in ms
    pub base_retry_delay_ms: u64,
}

impl Default for RefreshConfig {
    fn default() -> Self {
        Self {
            max_concurrent_requests: 10,
            batch_delay_ms: 100,
            max_retries: 3,
            base_retry_delay_ms: 1000,
        }
    }
}

/// Service for managing PR refresh operations
pub struct PrRefreshService {
    github: Arc<GitHubClient>,
    db: Arc<Database>,
    config: RefreshConfig,
}

impl PrRefreshService {
    pub fn new(github: Arc<GitHubClient>, db: Arc<Database>) -> Self {
        Self {
            github,
            db,
            config: RefreshConfig::default(),
        }
    }

    pub fn with_config(
        github: Arc<GitHubClient>,
        db: Arc<Database>,
        config: RefreshConfig,
    ) -> Self {
        Self { github, db, config }
    }

    /// Refresh check runs for all open PRs in a specific repository
    /// 
    /// This method updates ALL open PRs with appropriate check run status:
    /// - If there's a branch-specific freeze, only PRs targeting that branch get failure status
    /// - If there's a freeze without a specific branch, all PRs get failure status
    /// - If there's no freeze, all PRs get success status
    pub async fn refresh_repository_prs(
        &self,
        installation_id: u64,
        owner: &str,
        repo: &str,
        freeze_record: Option<&FreezeRecord>,
    ) -> Result<RefreshResult> {
        let is_frozen = freeze_record.is_some();
        info!(
            "Starting PR refresh for repository {}/{} (frozen: {})",
            owner, repo, is_frozen
        );

        // Get all open PRs with their head SHAs
        let prs = self
            .get_open_prs_with_sha(installation_id, owner, repo)
            .await?;

        if prs.is_empty() {
            info!("No open PRs found for repository {}/{}", owner, repo);
            return Ok(RefreshResult {
                total_prs: 0,
                successful_updates: 0,
                failed_updates: 0,
                errors: Vec::new(),
            });
        }

        info!("Found {} open PRs to update", prs.len());

        // Update all PRs - each PR will be checked individually to determine if it's frozen
        self.update_prs_in_batches(
            installation_id,
            owner,
            repo,
            &prs,
            freeze_record,
        )
        .await
    }

    /// Refresh check runs for all repositories with active freezes
    pub async fn refresh_all_active_freezes(&self) -> Result<HashMap<String, RefreshResult>> {
        info!("Starting global PR refresh for all active freezes");

        let conn = self
            .db
            .get_connection()
            .map_err(|e| anyhow!("Failed to get database connection: {}", e))?;

        let active_freezes = FreezeRecord::get_active_freezes(conn).await?;

        if active_freezes.is_empty() {
            info!("No active freezes found");
            return Ok(HashMap::new());
        }

        info!("Found {} active freezes to process", active_freezes.len());

        let mut results = HashMap::new();

        for freeze in active_freezes {
            // Parse repository owner/name
            let parts: Vec<&str> = freeze.repository.split('/').collect();
            if parts.len() != 2 {
                warn!("Invalid repository format: {}", freeze.repository);
                continue;
            }
            let (owner, repo) = (parts[0], parts[1]);

            match self
                .refresh_repository_prs(
                    freeze.installation_id as u64,
                    owner,
                    repo,
                    Some(&freeze), // Pass the freeze record
                )
                .await
            {
                Ok(result) => {
                    info!(
                        "Successfully refreshed {}/{}: {} PRs updated",
                        owner, repo, result.successful_updates
                    );
                    results.insert(freeze.repository.clone(), result);
                }
                Err(e) => {
                    error!("Failed to refresh repository {}: {}", freeze.repository, e);
                    results.insert(
                        freeze.repository.clone(),
                        RefreshResult {
                            total_prs: 0,
                            successful_updates: 0,
                            failed_updates: 0,
                            errors: vec![format!("Repository refresh failed: {}", e)],
                        },
                    );
                }
            }

            // Add delay between repositories to respect rate limits
            tokio::time::sleep(Duration::from_millis(self.config.batch_delay_ms)).await;
        }

        Ok(results)
    }

    /// Get open PRs for a repository with their head SHAs
    async fn get_open_prs_with_sha(
        &self,
        installation_id: u64,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<PullRequestInfo>> {
        self.github
            .with_installation_async(installation_id, |client| async move {
                let page = client
                    .pulls(owner, repo)
                    .list()
                    .state(octofer::octocrab::params::State::Open)
                    .per_page(100)
                    .send()
                    .await
                    .map_err(|e| {
                        error!("Failed to fetch open PRs: {:?}", e);
                        anyhow!("Failed to fetch open PRs: {}", e)
                    })?;

                let prs = page
                    .items
                    .into_iter()
                    .map(|pr| PullRequestInfo {
                        number: pr.number,
                        head_sha: pr.head.sha,
                        base_ref: pr.base.ref_field,
                    })
                    .collect();

                Ok(prs)
            })
            .await
    }

    async fn get_pr(
        &self,
        installation_id: u64,
        owner: &str,
        repo: &str,
        pr_number: u64,
    ) -> Result<PullRequest> {
        self.github
            .with_installation_async(installation_id, |client| async move {
                let pr = client
                    .pulls(owner, repo)
                    .get(pr_number)
                    .await
                    .map_err(|e| {
                        error!("Failed to fetch open PRs: {:?}", e);
                        anyhow!("Failed to fetch open PRs: {}", e)
                    })?;

                Ok(pr)
            })
            .await
    }

    /// Update PRs in batches with proper rate limiting and error handling
    async fn update_prs_in_batches(
        &self,
        installation_id: u64,
        owner: &str,
        repo: &str,
        prs: &[PullRequestInfo],
        freeze_record: Option<&FreezeRecord>,
    ) -> Result<RefreshResult> {
        let mut successful_updates = 0;
        let mut failed_updates = 0;
        let mut errors = Vec::new();

        // Process PRs in chunks to respect concurrent request limits
        for chunk in prs.chunks(self.config.max_concurrent_requests) {
            let mut handles = Vec::new();

            for pr in chunk {
                let github = self.github.clone();
                let pr = pr.clone();
                let owner = owner.to_string();
                let repo = repo.to_string();
                let config = self.config.clone();
                let freeze_record = freeze_record.cloned();

                let handle = tokio::spawn(async move {
                    // Determine if this specific PR is frozen
                    let is_pr_frozen = if let Some(ref freeze) = freeze_record {
                        if let Some(ref freeze_branch) = freeze.branch {
                            // Branch-specific freeze: only freeze if PR targets that branch
                            &pr.base_ref == freeze_branch
                        } else {
                            // No branch specified: freeze all PRs
                            true
                        }
                    } else {
                        // No freeze: all PRs are unfrozen
                        false
                    };

                    let conclusion = if is_pr_frozen {
                        CheckRunConclusion::Failure
                    } else {
                        CheckRunConclusion::Success
                    };

                    Self::update_pr_with_retry(
                        github,
                        installation_id,
                        &owner,
                        &repo,
                        &pr,
                        conclusion,
                        freeze_record.as_ref(),
                        config,
                    )
                    .await
                });

                handles.push(handle);
            }

            // Wait for all updates in this batch to complete
            for handle in handles {
                match handle.await {
                    Ok(Ok(_)) => successful_updates += 1,
                    Ok(Err(e)) => {
                        failed_updates += 1;
                        errors.push(e.to_string());
                    }
                    Err(e) => {
                        failed_updates += 1;
                        errors.push(format!("Task join error: {}", e));
                    }
                }
            }

            // Add delay between batches
            if chunk.len() == self.config.max_concurrent_requests {
                tokio::time::sleep(Duration::from_millis(self.config.batch_delay_ms)).await;
            }
        }

        Ok(RefreshResult {
            total_prs: prs.len(),
            successful_updates,
            failed_updates,
            errors,
        })
    }

    /// Update a single PR with retry logic
    async fn update_pr_with_retry(
        github: Arc<GitHubClient>,
        installation_id: u64,
        owner: &str,
        repo: &str,
        pr: &PullRequestInfo,
        conclusion: CheckRunConclusion,
        freeze_record: Option<&FreezeRecord>,
        config: RefreshConfig,
    ) -> Result<()> {
        let mut attempt = 0;

        while attempt <= config.max_retries {
            match create_check_run(
                &github,
                owner,
                repo,
                &pr.head_sha,
                CheckRunStatus::Completed,
                conclusion,
                installation_id,
                freeze_record,
            )
            .await
            {
                Ok(_) => {
                    if attempt > 0 {
                        info!(
                            "Successfully updated PR #{} after {} retries",
                            pr.number, attempt
                        );
                    }
                    return Ok(());
                }
                Err(e) => {
                    attempt += 1;
                    if attempt <= config.max_retries {
                        let delay = config.base_retry_delay_ms * 2_u64.pow((attempt - 1) as u32);
                        warn!(
                            "Failed to update PR #{} (attempt {}), retrying in {}ms: {}",
                            pr.number, attempt, delay, e
                        );
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                    } else {
                        error!(
                            "Failed to update PR #{} after {} attempts: {}",
                            pr.number, attempt, e
                        );
                        return Err(e);
                    }
                }
            }
        }

        Err(anyhow!(
            "Exhausted all retry attempts for PR #{}",
            pr.number
        ))
    }

    /// Refresh a single PR's check run status
    pub async fn refresh_single_pr(
        &self,
        installation_id: i64,
        repository: &Repository,
        pr_number: u64,
    ) -> Result<()> {
        // Get PR info
        let pr = self
            .get_pr(
                installation_id as u64,
                &repository.owner,
                &repository.name,
                pr_number,
            )
            .await?;

        let pr_base_ref = pr.base.ref_field.clone();

        // Check freeze status - get active freeze for this repo
        let repo_name = repository.full_name();
        let conn = self.db.get_connection()
            .map_err(|e| anyhow!("Failed to get database connection: {}", e))?;
        
        let freeze_record = FreezeRecord::get_active_freeze(conn, installation_id, &repo_name).await?;
        
        // Check if this PR is affected by the freeze
        let is_frozen = if let Some(ref freeze) = freeze_record {
            // If freeze has a specific branch, check if it matches this PR's base branch
            if let Some(ref freeze_branch) = freeze.branch {
                freeze_branch == &pr_base_ref
            } else {
                // No branch specified, freeze applies to all branches
                true
            }
        } else {
            false
        };

        let is_unlocked = if is_frozen {
            UnlockedPr::is_pr_unlocked(self.db.pool(), installation_id, &repo_name, pr_number)
                .await?
        } else {
            false
        };

        let pr_info = PullRequestInfo {
            number: pr.number,
            head_sha: pr.head.sha,
            base_ref: pr_base_ref,
        };

        // Determine check run conclusion based on freeze status
        let conclusion = if is_frozen && !is_unlocked {
            CheckRunConclusion::Failure
        } else {
            CheckRunConclusion::Success
        };

        Self::update_pr_with_retry(
            self.github.clone(),
            installation_id as u64,
            &repository.owner,
            &repository.name,
            &pr_info,
            conclusion,
            freeze_record.as_ref(),
            RefreshConfig::default(),
        )
        .await?;

        Ok(())
    }
}

async fn create_check_run(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    head_sha: &str,
    status: CheckRunStatus,
    conclusion: CheckRunConclusion,
    installation_id: u64,
    freeze_record: Option<&FreezeRecord>,
) -> Result<CheckRun> {
    let output = if let Some(freeze) = freeze_record {
        format_freeze_details(freeze)
    } else {
        format_success_output()
    };

    let result = client
        .app_client()
        .installation(installation_id.into())?
        .checks(owner, repo)
        .create_check_run(FREZZE_CHECK_RUN_NAME, head_sha)
        .status(status)
        .conclusion(conclusion)
        .output(output)
        .send()
        .await
        .map_err(|e| {
            error!("Failed to create check run: {:?}", e);
            anyhow!("Failed to create check run: {}", e)
        })?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_refresh_config_default() {
        let config = RefreshConfig::default();
        assert_eq!(config.max_concurrent_requests, 10);
        assert_eq!(config.batch_delay_ms, 100);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.base_retry_delay_ms, 1000);
    }

    #[test]
    fn test_pull_request_info_creation() {
        let pr_info = PullRequestInfo {
            number: 42,
            head_sha: "abc123def456".to_string(),
            base_ref: "main".to_string(),
        };

        assert_eq!(pr_info.number, 42);
        assert_eq!(pr_info.head_sha, "abc123def456");
        assert_eq!(pr_info.base_ref, "main");
    }

    #[test]
    fn test_refresh_result_creation() {
        let result = RefreshResult {
            total_prs: 5,
            successful_updates: 4,
            failed_updates: 1,
            errors: vec!["Error updating PR #3".to_string()],
        };

        assert_eq!(result.total_prs, 5);
        assert_eq!(result.successful_updates, 4);
        assert_eq!(result.failed_updates, 1);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0], "Error updating PR #3");
    }

    #[test]
    fn test_refresh_config_custom() {
        let config = RefreshConfig {
            max_concurrent_requests: 5,
            batch_delay_ms: 200,
            max_retries: 5,
            base_retry_delay_ms: 500,
        };

        assert_eq!(config.max_concurrent_requests, 5);
        assert_eq!(config.batch_delay_ms, 200);
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.base_retry_delay_ms, 500);
    }

    #[test]
    fn test_format_freeze_details() {
        use crate::database::models::{FreezeRecord, FreezeStatus};
        use chrono::Utc;

        let freeze_record = FreezeRecord {
            id: "test-id".to_string(),
            repository: "owner/repo".to_string(),
            installation_id: 12345,
            started_at: Utc::now(),
            expires_at: Some(Utc::now() + chrono::Duration::hours(2)),
            ended_at: None,
            reason: Some("Emergency maintenance".to_string()),
            initiated_by: "test-user".to_string(),
            ended_by: None,
            status: FreezeStatus::Active,
            branch: None,
            created_at: Utc::now(),
        };

        let output = format_freeze_details(&freeze_record);

        assert_eq!(output.title, "Repository is frozen for all branches by test-user");
        assert_eq!(
            output.summary,
            "This repository is currently under a freeze restriction for all branches"
        );
        assert!(output.text.is_some());

        let text = output.text.unwrap();
        assert!(text.contains("test-user"));
        assert!(text.contains("Emergency maintenance"));
        assert!(text.contains("All branches"));
        assert!(text.contains("This PR cannot be merged while the repository is frozen"));
        assert_eq!(output.annotations.len(), 0);
        assert_eq!(output.images.len(), 0);
    }

    #[test]
    fn test_format_freeze_details_no_reason() {
        use crate::database::models::{FreezeRecord, FreezeStatus};
        use chrono::Utc;

        let freeze_record = FreezeRecord {
            id: "test-id".to_string(),
            repository: "owner/repo".to_string(),
            installation_id: 12345,
            started_at: Utc::now(),
            expires_at: None,
            ended_at: None,
            reason: None,
            initiated_by: "test-user".to_string(),
            ended_by: None,
            status: FreezeStatus::Active,
            branch: None,
            created_at: Utc::now(),
        };

        let output = format_freeze_details(&freeze_record);

        assert_eq!(output.title, "Repository is frozen for all branches by test-user");
        assert!(output.text.is_some());

        let text = output.text.unwrap();
        assert!(text.contains("No reason provided"));
        assert!(text.contains("No end time set"));
        assert!(text.contains("All branches"));
    }

    #[test]
    fn test_format_success_output() {
        let output = format_success_output();

        assert_eq!(output.title, "Repository is not frozen");
        assert_eq!(
            output.summary,
            "This repository is currently not under any freeze restrictions"
        );
        assert!(output.text.is_some());

        let text = output.text.unwrap();
        assert!(text.contains("PRs can be merged normally"));
        assert_eq!(output.annotations.len(), 0);
        assert_eq!(output.images.len(), 0);
    }
}
