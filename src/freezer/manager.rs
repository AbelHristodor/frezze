use std::sync::Arc;

use crate::database::{Database, models::FreezeRecord};
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use tracing::info;

pub struct FreezeManager {
    pub db: Arc<Database>,
}

impl FreezeManager {
    pub fn new(db: Arc<Database>) -> Self {
        FreezeManager { db }
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
}
