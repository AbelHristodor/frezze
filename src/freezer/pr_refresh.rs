use std::sync::Arc;
use anyhow::{Result, anyhow};
use chrono::Utc;
use octocrab::params::checks::{CheckRunStatus, CheckRunConclusion};
use tracing::{info, error};
use tokio::task::JoinSet;

use crate::{
    github::Github,
    database::{Database, models::{FreezeRecord, FreezeStatus}},
};

#[derive(Clone)]
pub struct PrRefresher {
    github: Arc<Github>,
    db: Arc<Database>,
}

impl PrRefresher {
    pub fn new(github: Arc<Github>, db: Arc<Database>) -> Self {
        Self { github, db }
    }

    /// Refresh all open PRs for repositories with active or scheduled freeze records
    pub async fn refresh_all_prs(&self) -> Result<()> {
        let conn = self.db.get_connection()
            .map_err(|e| anyhow!("Failed to get database connection: {}", e))?;

        let freeze_records = FreezeRecord::get_scheduled_to_activate(conn).await
            .map_err(|e| anyhow!("Failed to get scheduled freeze records: {}", e))?;

        let mut tasks = JoinSet::new();

        for record in freeze_records {
            let github = self.github.clone();
            tasks.spawn(async move {
                Self::refresh_repository_prs_static(&github, &record).await
            });
        }

        while let Some(result) = tasks.join_next().await {
            if let Err(e) = result {
                error!("Task failed: {}", e);
            }
        }

        Ok(())
    }

    /// Refresh PRs for a specific repository
    pub async fn refresh_repository(&self, installation_id: i64, repository: &str) -> Result<()> {
        let conn = self.db.get_connection()
            .map_err(|e| anyhow!("Failed to get database connection: {}", e))?;

        let freeze_records = FreezeRecord::list(conn, Some(installation_id), Some(repository), Some(true)).await
            .map_err(|e| anyhow!("Failed to get freeze records for repository {}: {}", repository, e))?;

        if let Some(record) = freeze_records.first() {
            Self::refresh_repository_prs_static(&self.github, record).await?;
        } else {
            self.refresh_repository_prs_with_status(installation_id, repository, false).await?;
        }

        Ok(())
    }

    /// Static method for refreshing repository PRs (used in spawned tasks)
    async fn refresh_repository_prs_static(github: &Arc<Github>, freeze_record: &FreezeRecord) -> Result<()> {
        let is_frozen = Self::is_currently_frozen_static(freeze_record);
        Self::refresh_repository_prs_with_status_static(
            github,
            freeze_record.installation_id,
            &freeze_record.repository,
            is_frozen,
        ).await
    }

    /// Refresh PRs for a repository with a specific freeze status
    async fn refresh_repository_prs_with_status(
        &self,
        installation_id: i64,
        repository: &str,
        is_frozen: bool,
    ) -> Result<()> {
        Self::refresh_repository_prs_with_status_static(&self.github, installation_id, repository, is_frozen).await
    }

    /// Static method for refreshing PRs with status (used in spawned tasks)
    async fn refresh_repository_prs_with_status_static(
        github: &Arc<Github>,
        installation_id: i64,
        repository: &str,
        is_frozen: bool,
    ) -> Result<()> {
        let repo_parts: Vec<&str> = repository.split('/').collect();
        if repo_parts.len() != 2 {
            return Err(anyhow!("Invalid repository format: {}", repository));
        }
        let (owner, repo) = (repo_parts[0], repo_parts[1]);

        let prs = Self::get_open_prs_static(github, installation_id as u64, owner, repo).await?;
        
        info!("Found {} open PRs for repository {}", prs.len(), repository);

        let (status, conclusion) = if is_frozen {
            (CheckRunStatus::Completed, CheckRunConclusion::Failure)
        } else {
            (CheckRunStatus::Completed, CheckRunConclusion::Success)
        };

        let mut pr_tasks = JoinSet::new();

        for pr in prs {
            let github = Arc::clone(github);
            let owner = owner.to_string();
            let repo = repo.to_string();
            let head_sha = pr.head.sha.clone();
            let pr_number = pr.number;
            let status = status.clone();
            let conclusion = conclusion.clone();
            let installation_id = installation_id as u64;

            pr_tasks.spawn(async move {
                if let Err(e) = github.create_check_run(
                    &owner,
                    &repo,
                    &head_sha,
                    status,
                    conclusion,
                    installation_id,
                ).await {
                    error!("Failed to create check run for PR #{}: {}", pr_number, e);
                }
            });
        }

        while let Some(_) = pr_tasks.join_next().await {}

        Ok(())
    }

    /// Static method for getting open PRs (used in spawned tasks)
    async fn get_open_prs_static(
        github: &Arc<Github>,
        installation_id: u64,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<octocrab::models::pulls::PullRequest>> {
        github.with_installation_async(installation_id, |client| async move {
            let prs = client
                .pulls(owner, repo)
                .list()
                .state(octocrab::params::State::Open)
                .send()
                .await
                .map_err(|e| anyhow!("Failed to get open PRs: {}", e))?;

            Ok(prs.items)
        }).await
    }

    /// Static method for checking if frozen (used in spawned tasks)
    fn is_currently_frozen_static(freeze_record: &FreezeRecord) -> bool {
        let now = Utc::now();
        
        match freeze_record.status {
            FreezeStatus::Active => {
                if now >= freeze_record.started_at {
                    if let Some(expires_at) = freeze_record.expires_at {
                        now < expires_at
                    } else {
                        true
                    }
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
