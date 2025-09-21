//! Database operations for freeze records, permissions, and command logs.
//!
//! This module provides CRUD operations for managing repository freeze states,
//! user permissions, and command audit logs in the SQLite database.

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

use crate::database::models::{FreezeRecord, FreezeStatus};

/// Helper function to parse SQLite datetime string to DateTime<Utc>
fn parse_datetime(datetime_str: &str) -> Result<DateTime<Utc>> {
    datetime_str.parse::<DateTime<Utc>>()
        .map_err(|e| anyhow::anyhow!("Failed to parse datetime: {}", e))
}

/// Helper function to parse optional SQLite datetime string to Option<DateTime<Utc>>
fn parse_optional_datetime(datetime_str: Option<String>) -> Result<Option<DateTime<Utc>>> {
    match datetime_str {
        Some(s) => Ok(Some(parse_datetime(&s)?)),
        None => Ok(None),
    }
}

/// Database operations for freeze records.
impl FreezeRecord {
    /// Creates a new freeze record in the database.
    ///
    /// This method checks for overlapping active freeze records before creating
    /// a new one to prevent conflicts. A freeze record is considered overlapping
    /// if it has any time period intersection with existing active freezes.
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool
    /// * `record` - The freeze record to create
    ///
    /// # Returns
    ///
    /// Returns the created freeze record on success, or an error if:
    /// - An overlapping freeze record already exists
    /// - Database operation fails
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use frezze::database::models::FreezeRecord;
    /// # use sqlx::SqlitePool;
    /// # async fn example(pool: &SqlitePool) -> anyhow::Result<()> {
    /// let record = FreezeRecord::new(
    ///     "owner/repo".to_string(),
    ///     12345,
    ///     chrono::Utc::now(),
    ///     Some(chrono::Utc::now() + chrono::Duration::hours(2)),
    ///     Some("Emergency maintenance".to_string()),
    ///     "user123".to_string(),
    /// );
    ///
    /// let created = FreezeRecord::create(pool, &record).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(pool: &SqlitePool, record: &FreezeRecord) -> Result<FreezeRecord> {
        // Check for overlapping active freeze records to prevent conflicts.
        // Three overlap scenarios are checked:
        // 1. New freeze starts during an existing freeze
        // 2. New freeze ends during an existing freeze
        // 3. New freeze completely encompasses an existing freeze
        let overlapping = sqlx::query!(
            r#"
            SELECT COUNT(*) as count FROM freeze_records 
            WHERE repository = $1 
            AND installation_id = $2 
            AND status = 'active'
            AND (
                (started_at <= $3 AND (expires_at IS NULL OR expires_at > $3))
                OR (started_at < $4 AND (expires_at IS NULL OR expires_at >= $4))
                OR ($3 <= started_at AND ($4 IS NULL OR $4 > started_at))
            )
            "#,
            record.repository,
            record.installation_id,
            record.started_at,
            record.expires_at
        )
        .fetch_one(pool)
        .await?;

        if overlapping.count > 0 {
            return Err(anyhow::anyhow!(
                "A freeze record already exists for this time period"
            ));
        }

        // Insert the new freeze record
        let status_str = record.status.to_string();
        sqlx::query!(
            r#"
            INSERT INTO freeze_records 
            (id, repository, installation_id, started_at, expires_at, ended_at, reason, initiated_by, ended_by, status, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
            record.id,
            record.repository,
            record.installation_id,
            record.started_at,
            record.expires_at,
            record.ended_at,
            record.reason,
            record.initiated_by,
            record.ended_by,
            status_str,
            record.created_at
        )
        .execute(pool)
        .await?;

        Ok(record.clone())
    }

    /// Retrieves freeze records from the database with optional filtering.
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool
    /// * `installation_id` - Optional GitHub installation ID filter
    /// * `repository` - Optional repository name filter (format: "owner/repo")
    /// * `active` - Optional filter for active status only
    ///
    /// # Returns
    ///
    /// Returns a vector of freeze records matching the filters, ordered by creation date (newest first).
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use frezze::database::models::FreezeRecord;
    /// # use sqlx::SqlitePool;
    /// # async fn example(pool: &SqlitePool) -> anyhow::Result<()> {
    /// // Get all active freezes for a specific repository
    /// let active_freezes = FreezeRecord::list(pool, Some(12345), Some("owner/repo"), Some(true)).await?;
    ///
    /// // Get all freezes for an installation
    /// let all_freezes = FreezeRecord::list(pool, Some(12345), None, None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(
        pool: &SqlitePool,
        installation_id: Option<u64>,
        repository: Option<&str>,
        active: Option<bool>,
    ) -> Result<Vec<FreezeRecord>> {
        let mut query = "SELECT * FROM freeze_records WHERE 1=1".to_string();
        let mut param_count = 0;

        if installation_id.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND installation_id = ${}", param_count));
        }

        if repository.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND repository = ${}", param_count));
        }

        if let Some(is_active) = active
            && is_active
        {
            query.push_str(" AND status = 'active'");
        }

        query.push_str(" ORDER BY created_at DESC");

        let mut sql_query = sqlx::query(&query);

        if let Some(inst_id) = installation_id {
            sql_query = sql_query.bind(inst_id as i64);
        }

        if let Some(repo) = repository {
            sql_query = sql_query.bind(repo);
        }

        let rows = sql_query.fetch_all(pool).await?;

        let mut records = Vec::new();
        for row in rows {
            records.push(FreezeRecord {
                id: row.get("id"),
                repository: row.get("repository"),
                installation_id: row.get("installation_id"),
                started_at: row.get("started_at"),
                expires_at: row.get("expires_at"),
                ended_at: row.get("ended_at"),
                reason: row.get("reason"),
                initiated_by: row.get("initiated_by"),
                ended_by: row.get("ended_by"),
                status: FreezeStatus::from(row.get::<String, _>("status").as_str()),
                created_at: row.get("created_at"),
            });
        }

        Ok(records)
    }

    /// Updates the status of a freeze record.
    ///
    /// When updating to `FreezeStatus::Ended`, automatically sets the `ended_at`
    /// timestamp to the current time.
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool
    /// * `id` - String UUID of the freeze record to update
    /// * `status` - New status to set
    /// * `ended_by` - Optional username of who ended the freeze (used when status is Ended)
    ///
    /// # Returns
    ///
    /// Returns the updated freeze record if found, or `None` if no record exists with the given ID.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use frezze::database::models::{FreezeRecord, FreezeStatus};
    /// # use sqlx::SqlitePool;
    /// # async fn example(pool: &SqlitePool, freeze_id: String) -> anyhow::Result<()> {
    /// // End a freeze
    /// let updated = FreezeRecord::update_status(
    ///     pool,
    ///     freeze_id,
    ///     FreezeStatus::Ended,
    ///     Some("admin".to_string())
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn update_status(
        pool: &SqlitePool,
        id: String,
        status: FreezeStatus,
        ended_by: Option<String>,
    ) -> Result<Option<FreezeRecord>> {
        let status_str = match status {
            FreezeStatus::Scheduled => "scheduled",
            FreezeStatus::Active => "active",
            FreezeStatus::Expired => "expired",
            FreezeStatus::Ended => "ended",
        };

        let ended_at = if matches!(status, FreezeStatus::Ended) {
            Some(Utc::now())
        } else {
            None
        };

        let result = sqlx::query!(
            r#"
            UPDATE freeze_records 
            SET status = $1, ended_at = $2, ended_by = $3
            WHERE id = $4
            "#,
            status_str,
            ended_at,
            ended_by,
            id
        )
        .execute(pool)
        .await?;

        if result.rows_affected() > 0 {
            // Fetch the updated record
            let row = sqlx::query!(
                "SELECT * FROM freeze_records WHERE id = $1",
                id
            )
            .fetch_optional(pool)
            .await?;

            match row {
                Some(row) => Ok(Some(FreezeRecord {
                    id: row.id.unwrap_or_default(),
                    repository: row.repository,
                    installation_id: row.installation_id,
                    started_at: parse_datetime(&row.started_at)?,
                    expires_at: parse_optional_datetime(row.expires_at)?,
                    ended_at: parse_optional_datetime(row.ended_at)?,
                    reason: row.reason,
                    initiated_by: row.initiated_by,
                    ended_by: row.ended_by,
                    status: FreezeStatus::from(row.status.as_str()),
                    created_at: parse_datetime(&row.created_at)?,
                })),
                None => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    /// Retrieves freeze records that should currently be active.
    ///
    /// Returns all freeze records with 'active' status where:
    /// - The start time has passed (started_at <= now)
    /// - The freeze hasn't expired (expires_at is NULL or > now)
    ///
    /// This is used by the background PR refresh system to determine which
    /// repositories should have their PRs updated with freeze status.
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool
    ///
    /// # Returns
    ///
    /// Returns a vector of currently active freeze records, ordered by start time.
    pub async fn get_active_freezes(pool: &SqlitePool) -> Result<Vec<FreezeRecord>> {
        let now = Utc::now();
        let rows = sqlx::query!(
            r#"
            SELECT * FROM freeze_records 
            WHERE status = 'active' 
            AND started_at <= $1 
            AND (expires_at IS NULL OR expires_at > $1)
            ORDER BY started_at ASC
            "#,
            now
        )
        .fetch_all(pool)
        .await?;

        let mut records = Vec::new();
        for row in rows {
            records.push(FreezeRecord {
                id: row.id.unwrap_or_default(),
                repository: row.repository,
                installation_id: row.installation_id,
                started_at: parse_datetime(&row.started_at).unwrap_or_else(|_| Utc::now()),
                expires_at: parse_optional_datetime(row.expires_at).unwrap_or(None),
                ended_at: parse_optional_datetime(row.ended_at).unwrap_or(None),
                reason: row.reason,
                initiated_by: row.initiated_by,
                ended_by: row.ended_by,
                status: FreezeStatus::from(row.status.as_str()),
                created_at: parse_datetime(&row.created_at).unwrap_or_else(|_| Utc::now()),
            });
        }

        Ok(records)
    }

    /// Checks if a repository is currently frozen.
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool
    /// * `installation_id` - GitHub installation ID
    /// * `repository` - Repository name in "owner/repo" format
    ///
    /// # Returns
    ///
    /// Returns `true` if there are any active freeze records for the repository, `false` otherwise.
    pub async fn is_frozen(
        pool: &SqlitePool,
        installation_id: i64,
        repository: &str,
    ) -> Result<bool> {
        let row = sqlx::query!(
            "SELECT EXISTS(SELECT 1 FROM freeze_records WHERE installation_id = $1 AND repository = $2 AND status = 'active') as exists_active",
            installation_id,
            repository
        )
        .fetch_one(pool)
        .await?;
        Ok(row.exists_active != 0)
    }
}
