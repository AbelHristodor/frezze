use std::sync::Arc;

use crate::{
    database::{Database, models::FreezeRecord},
    github::Github,
    repository::Repository,
};
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use octocrab::models::issues::Comment;
use tracing::info;

pub const DEFAULT_FREEZE_DURATION: chrono::Duration = chrono::Duration::hours(2);

pub struct FreezeManager {
    pub db: Arc<Database>,
    pub github: Arc<Github>,
}

impl FreezeManager {
    pub fn new(db: Arc<Database>, github: Arc<Github>) -> Self {
        FreezeManager { db, github }
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
            .create_comment(installation_id as u64, repository.owner(), repository.name(), issue_nr, msg)
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

        let records = FreezeRecord::list(conn, Some(installation_id), Some(&repository.full_name()), active)
            .await
            .map_err(|e| anyhow!("Failed to list freeze records for repo {}: {}", repository, e))?;

        if records.is_empty() {
            info!("No freeze records found for repository: {}", repository);
            return Err(anyhow!("No freeze records found for repository: {}", repository));
        }
        info!(
            "Found {} freeze records for repository: {}",
            records.len(),
            repository
        );
        Ok(records)
    }

    pub async fn is_frozen(&self, repository: &Repository, installation_id: i64) -> Result<bool> {
        let conn = self
            .db
            .get_connection()
            .map_err(|e| anyhow!("Failed to get database connection: {}", e))?;

        let frozen = FreezeRecord::is_frozen(conn, installation_id, &repository.full_name())
            .await
            .map_err(|e| anyhow!("Failed to check if repository {} is frozen: {}", repository, e))?;

        Ok(frozen)
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
    pub async fn unfreeze(&self, installation_id: i64, repository: &Repository, ended_by: String) -> Result<()> {
        let conn = self
            .db
            .get_connection()
            .map_err(|e| anyhow!("Failed to get database connection: {}", e))?;

        // Get active freeze records for this repository
        let freeze_records =
            FreezeRecord::list(conn, Some(installation_id), Some(&repository.full_name()), Some(true))
                .await
                .map_err(|e| anyhow!("Failed to get freeze records for repo {}: {}", repository, e))?;

        if freeze_records.is_empty() {
            return Err(anyhow!("No active freeze found for repository: {}", repository));
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
        Ok(())
    }
}
