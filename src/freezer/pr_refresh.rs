//! PR refresh system for updating check runs on all open pull requests.
//!
//! This module provides functionality to efficiently update all open PRs with 
//! freeze check runs while respecting GitHub API rate limits.

use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::{anyhow, Result};
use octocrab::params::checks::{CheckRunConclusion, CheckRunStatus};
use tracing::{error, info, warn};

use crate::{
    database::{models::FreezeRecord, Database},
    github::Github,
};

/// Information about a pull request needed for check run updates
#[derive(Debug, Clone)]
pub struct PullRequestInfo {
    pub number: u64,
    pub head_sha: String,
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
    github: Arc<Github>,
    db: Arc<Database>,
    config: RefreshConfig,
}

impl PrRefreshService {
    pub fn new(github: Arc<Github>, db: Arc<Database>) -> Self {
        Self {
            github,
            db,
            config: RefreshConfig::default(),
        }
    }

    pub fn with_config(github: Arc<Github>, db: Arc<Database>, config: RefreshConfig) -> Self {
        Self {
            github,
            db,
            config,
        }
    }

    /// Refresh check runs for all open PRs in a specific repository
    pub async fn refresh_repository_prs(
        &self,
        installation_id: u64,
        owner: &str,
        repo: &str,
        is_frozen: bool,
    ) -> Result<RefreshResult> {
        info!(
            "Starting PR refresh for repository {}/{} (frozen: {})",
            owner, repo, is_frozen
        );

        // Get all open PRs with their head SHAs
        let prs = self.get_open_prs_with_sha(installation_id, owner, repo).await?;
        
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

        // Determine check run conclusion based on freeze status
        let conclusion = if is_frozen {
            CheckRunConclusion::Failure
        } else {
            CheckRunConclusion::Success
        };

        // Update PRs in batches with rate limiting
        self.update_prs_in_batches(installation_id, owner, repo, &prs, conclusion)
            .await
    }

    /// Refresh check runs for all repositories with active freezes
    pub async fn refresh_all_active_freezes(&self) -> Result<HashMap<String, RefreshResult>> {
        info!("Starting global PR refresh for all active freezes");

        let conn = self.db.get_connection()
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
                    true, // Active freeze means frozen
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
                    .state(octocrab::params::State::Open)
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
                    })
                    .collect();

                Ok(prs)
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
        conclusion: CheckRunConclusion,
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
                let conclusion = conclusion.clone();
                let config = self.config.clone();

                let handle = tokio::spawn(async move {
                    Self::update_pr_with_retry(
                        github,
                        installation_id,
                        &owner,
                        &repo,
                        &pr,
                        conclusion,
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
        github: Arc<Github>,
        installation_id: u64,
        owner: &str,
        repo: &str,
        pr: &PullRequestInfo,
        conclusion: CheckRunConclusion,
        config: RefreshConfig,
    ) -> Result<()> {
        let mut attempt = 0;

        while attempt <= config.max_retries {
            match github
                .create_check_run(
                    owner,
                    repo,
                    &pr.head_sha,
                    CheckRunStatus::Completed,
                    conclusion.clone(),
                    installation_id,
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
                        error!("Failed to update PR #{} after {} attempts: {}", pr.number, attempt, e);
                        return Err(e);
                    }
                }
            }
        }

        Err(anyhow!("Exhausted all retry attempts for PR #{}", pr.number))
    }
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
        };
        
        assert_eq!(pr_info.number, 42);
        assert_eq!(pr_info.head_sha, "abc123def456");
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
}