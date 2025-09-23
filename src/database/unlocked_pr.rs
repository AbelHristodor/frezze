use anyhow::Result;
use chrono::Utc;
use sqlx::SqlitePool;

use crate::database::models::UnlockedPr;

impl UnlockedPr {
    /// Unlock a specific PR during a freeze
    pub async fn unlock_pr(
        pool: &SqlitePool,
        installation_id: i64,
        repository: &str,
        pr_number: u64,
        unlocked_by: &str,
    ) -> Result<(), anyhow::Error> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        let pr = pr_number as i64;
        let unlocked_at = now.to_rfc3339();

        sqlx::query!(
            r#"
            INSERT OR REPLACE INTO unlocked_prs
            (id, repository, installation_id, pr_number, unlocked_by, unlocked_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
            id,
            repository,
            installation_id,
            pr,
            unlocked_by,
            unlocked_at
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Check if a PR is unlocked during a freeze
    pub async fn is_pr_unlocked(
        pool: &SqlitePool,
        installation_id: i64,
        repository: &str,
        pr_number: u64,
    ) -> Result<bool> {
        let pr = pr_number as i64;
        let result = sqlx::query!(
            r#"
            SELECT COUNT(*) as count FROM unlocked_prs
            WHERE installation_id = ? AND repository = ? AND pr_number = ?
            "#,
            installation_id,
            repository,
            pr
        )
        .fetch_one(pool)
        .await?;

        Ok(result.count > 0)
    }

    /// Clear all unlocked PRs for a repository (called when freeze ends)
    pub async fn clear_unlocked_prs(
        pool: &SqlitePool,
        installation_id: i64,
        repository: &str,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            DELETE FROM unlocked_prs
            WHERE installation_id = ? AND repository = ?
            "#,
            installation_id,
            repository
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
