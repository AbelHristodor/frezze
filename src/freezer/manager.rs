use std::sync::Arc;

use crate::{
    database::{Database, models::FreezeRecord},
    github::Github,
    freezer::pr_refresh::PrRefresher,
};
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use tracing::info;

pub const DEFAULT_FREEZE_DURATION: chrono::Duration = chrono::Duration::hours(2);

pub struct FreezeManager {
    pub db: Arc<Database>,
    pub github: Arc<Github>,
    pub pr_refresher: PrRefresher,
}

impl FreezeManager {
    pub fn new(db: Arc<Database>, github: Arc<Github>) -> Self {
        let pr_refresher = PrRefresher::new(github.clone(), db.clone());
        FreezeManager { 
            db, 
            github,
            pr_refresher,
        }
    }

    pub async fn freeze(
        &self,
        installation_id: i64,
        repo: &str,
        duration: Option<chrono::Duration>,
        reason: Option<String>,
        initiated_by: String,
    ) -> Result<FreezeRecord> {
        let start = Utc::now();
        let duration = match duration {
            Some(d) => d,
            None => DEFAULT_FREEZE_DURATION,
        };
        let end = Some(start + duration);
        let record = FreezeRecord::new(
            repo.into(),
            installation_id,
            start,
            end,
            reason,
            initiated_by,
        );

        let conn = self
            .db
            .get_connection()
            .map_err(|e| anyhow!("Failed to get database connection: {}", e))?;

        let record = FreezeRecord::create(conn, &record).await?;

        // Refresh PRs after creating freeze
        self.refresh_prs().await?;
        Ok(record)
    }

    pub async fn list_for_repo(
        &self,
        repo: &str,
        installation_id: i64,
        active: Option<bool>,
    ) -> Result<Vec<FreezeRecord>> {
        let conn = self
            .db
            .get_connection()
            .map_err(|e| anyhow!("Failed to get database connection: {}", e))?;

        let records = FreezeRecord::list(conn, Some(installation_id), Some(repo), active)
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

    pub async fn is_frozen(&self, repo: &str, installation_id: i64) -> Result<bool> {
        let conn = self
            .db
            .get_connection()
            .map_err(|e| anyhow!("Failed to get database connection: {}", e))?;

        let frozen = FreezeRecord::is_frozen(conn, installation_id, repo)
            .await
            .map_err(|e| anyhow!("Failed to check if repository {} is frozen: {}", repo, e))?;

        Ok(frozen)
    }

    pub async fn schedule_freeze(
        &self,
        installation_id: i64,
        repo: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        reason: Option<String>,
        initiated_by: String,
    ) -> Result<()> {
        let record = FreezeRecord::new(
            repo.into(),
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

    /// Refresh all open PRs to sync with current freeze status
    pub async fn refresh_prs(&self) -> Result<()> {
        self.pr_refresher.refresh_all_prs().await
    }

    /// Refresh PRs for a specific repository
    pub async fn refresh_repository_prs(&self, installation_id: i64, repository: &str) -> Result<()> {
        self.pr_refresher.refresh_repository(installation_id, repository).await
    }

    /// Unfreeze a repository
    pub async fn unfreeze(&self, installation_id: i64, repo: &str, ended_by: String) -> Result<()> {
        let conn = self.db.get_connection()
            .map_err(|e| anyhow!("Failed to get database connection: {}", e))?;

        // Get active freeze records for this repository
        let freeze_records = FreezeRecord::list(conn, Some(installation_id), Some(repo), Some(true)).await
            .map_err(|e| anyhow!("Failed to get freeze records for repo {}: {}", repo, e))?;

        if freeze_records.is_empty() {
            return Err(anyhow!("No active freeze found for repository: {}", repo));
        }

        // End all active freezes for this repository
        for record in freeze_records {
            FreezeRecord::update_status(conn, record.id, crate::database::models::FreezeStatus::Ended, Some(ended_by.clone())).await
                .map_err(|e| anyhow!("Failed to end freeze record {}: {}", record.id, e))?;
        }

        // Refresh PRs after unfreezing
        //self.refresh_repository_prs(installation_id, repo).await?;
        Ok(())
    }
}
