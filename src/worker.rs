use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tokio::time::interval;
use tracing::{error, info, warn};

use crate::{
    database::{
        Database,
        models::{FreezeRecord, FreezeStatus},
    },
    freezer::manager::FreezeManager,
    repository::Repository,
};
use octofer::github::GitHubClient;

/// Worker that checks for scheduled freezes and activates them when their time comes
pub struct FreezeSchedulerWorker {
    db: Arc<Database>,
    github: Arc<GitHubClient>,
}

impl FreezeSchedulerWorker {
    pub fn new(db: Arc<Database>, github: Arc<GitHubClient>) -> Self {
        Self { db, github }
    }

    /// Start the worker that checks for scheduled freezes every minute
    pub async fn start(&self) {
        info!("Starting freeze scheduler worker");

        let mut interval = interval(Duration::from_secs(60)); // Check every minute

        loop {
            interval.tick().await;

            if let Err(e) = self.check_and_activate_scheduled_freezes().await {
                error!("Error checking scheduled freezes: {}", e);
            }
        }
    }

    /// Check for scheduled freezes that should be activated and activate them
    async fn check_and_activate_scheduled_freezes(&self) -> anyhow::Result<()> {
        let conn = self
            .db
            .get_connection()
            .map_err(|e| anyhow::anyhow!("Failed to get database connection: {}", e))?;

        // Get all scheduled freezes that should be activated now
        let scheduled_freezes = self.get_scheduled_freezes_to_activate(conn).await?;

        if scheduled_freezes.is_empty() {
            return Ok(());
        }

        info!(
            "Found {} scheduled freezes to activate",
            scheduled_freezes.len()
        );

        for freeze_record in scheduled_freezes {
            match self.activate_scheduled_freeze(&freeze_record).await {
                Ok(_) => {
                    info!(
                        "Successfully activated scheduled freeze for repository: {}",
                        freeze_record.repository
                    );
                }
                Err(e) => {
                    error!(
                        "Failed to activate scheduled freeze for repository {}: {}",
                        freeze_record.repository, e
                    );
                }
            }
        }

        Ok(())
    }

    /// Get scheduled freezes that should be activated now
    async fn get_scheduled_freezes_to_activate(
        &self,
        conn: &sqlx::PgPool,
    ) -> anyhow::Result<Vec<FreezeRecord>> {
        let now = Utc::now();

        let rows = sqlx::query!(
            r#"
            SELECT * FROM freeze_records 
            WHERE status = 'scheduled' 
            AND started_at <= $1
            ORDER BY started_at ASC
            "#,
            now
        )
        .fetch_all(conn)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to query scheduled freezes: {}", e))?;

        let mut records = Vec::new();
        for row in rows {
            records.push(FreezeRecord {
                id: row.id,
                repository: row.repository,
                installation_id: row.installation_id,
                started_at: row.started_at,
                expires_at: row.expires_at,
                ended_at: row.ended_at,
                reason: row.reason,
                initiated_by: row.initiated_by,
                ended_by: row.ended_by,
                status: FreezeStatus::from(row.status.as_str()),
                created_at: row.created_at,
            });
        }

        Ok(records)
    }

    /// Activate a scheduled freeze by updating its status and applying the freeze
    async fn activate_scheduled_freeze(&self, freeze_record: &FreezeRecord) -> anyhow::Result<()> {
        let conn = self
            .db
            .get_connection()
            .map_err(|e| anyhow::anyhow!("Failed to get database connection: {}", e))?;

        // Update the freeze status to active
        FreezeRecord::update_status(conn, freeze_record.id, FreezeStatus::Active, None)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to update freeze status: {}", e))?;

        // Parse repository name
        let parts: Vec<&str> = freeze_record.repository.split('/').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!(
                "Invalid repository format: {}",
                freeze_record.repository
            ));
        }

        let repository = Repository::new(parts[0], parts[1]);

        // Apply the freeze using the freeze manager
        let freeze_manager = FreezeManager::new(self.db.clone(), self.github.clone());

        // We call the internal handle_freeze method directly since we already have the record
        // and don't want to create a duplicate entry
        match self
            .apply_freeze_to_repository(
                &freeze_manager,
                freeze_record.installation_id as u64,
                &repository,
            )
            .await
        {
            Ok(_) => {
                info!(
                    "Successfully applied freeze to repository: {}",
                    freeze_record.repository
                );
            }
            Err(e) => {
                warn!(
                    "Failed to apply freeze operations to repository {}: {}. Freeze record is still marked as active.",
                    freeze_record.repository, e
                );
                // Note: We don't fail the activation here because the database record
                // is already updated. The freeze is "active" from a tracking perspective
                // even if some GitHub operations failed.
            }
        }

        Ok(())
    }

    /// Apply freeze operations to a repository (like updating PRs, etc.)
    async fn apply_freeze_to_repository(
        &self,
        freeze_manager: &FreezeManager,
        installation_id: u64,
        repository: &Repository,
    ) -> anyhow::Result<()> {
        // Apply PR refresh operations (mark PRs as failing, etc.)
        freeze_manager
            .pr_refresh
            .refresh_repository_prs(
                installation_id,
                repository.owner(),
                repository.name(),
                true, // Repository is now frozen
            )
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to refresh PRs for repository {}: {}",
                    repository.full_name(),
                    e
                )
            })?;

        info!(
            "Successfully applied freeze operations to repository: {}",
            repository.full_name()
        );
        Ok(())
    }
}

