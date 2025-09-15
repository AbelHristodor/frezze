//! Database operations for freeze records, permissions, and command logs.
//!
//! This module provides CRUD operations for managing repository freeze states,
//! user permissions, and command audit logs in the PostgreSQL database.

use anyhow::Result;
use chrono::Utc;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::models::{
    CommandLog, CommandResult, FreezeRecord, FreezeStatus, PermissionRecord, Role,
};

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
    /// # use sqlx::PgPool;
    /// # async fn example(pool: &PgPool) -> anyhow::Result<()> {
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
    pub async fn create(pool: &PgPool, record: &FreezeRecord) -> Result<FreezeRecord> {
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

        if overlapping.count.unwrap_or(0) > 0 {
            return Err(anyhow::anyhow!(
                "A freeze record already exists for this time period"
            ));
        }

        // Insert the new freeze record
        let row = sqlx::query!(
            r#"
            INSERT INTO freeze_records 
            (id, repository, installation_id, started_at, expires_at, ended_at, reason, initiated_by, ended_by, status, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
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
            record.status.to_string(),
            record.created_at
        )
        .fetch_one(pool)
        .await?;

        Ok(FreezeRecord {
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
        })
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
    /// # use sqlx::PgPool;
    /// # async fn example(pool: &PgPool) -> anyhow::Result<()> {
    /// // Get all active freezes for a specific repository
    /// let active_freezes = FreezeRecord::list(pool, Some(12345), Some("owner/repo"), Some(true)).await?;
    ///
    /// // Get all freezes for an installation
    /// let all_freezes = FreezeRecord::list(pool, Some(12345), None, None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(
        pool: &PgPool,
        installation_id: Option<i64>,
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

        if let Some(is_active) = active {
            if is_active {
                query.push_str(" AND status = 'active'");
            }
        }

        query.push_str(" ORDER BY created_at DESC");

        let mut sql_query = sqlx::query(&query);

        if let Some(inst_id) = installation_id {
            sql_query = sql_query.bind(inst_id);
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
    /// * `id` - UUID of the freeze record to update
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
    /// # use sqlx::PgPool;
    /// # use uuid::Uuid;
    /// # async fn example(pool: &PgPool, freeze_id: Uuid) -> anyhow::Result<()> {
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
        pool: &PgPool,
        id: Uuid,
        status: FreezeStatus,
        ended_by: Option<String>,
    ) -> Result<Option<FreezeRecord>> {
        let status_str = match status {
            FreezeStatus::Active => "active",
            FreezeStatus::Expired => "expired",
            FreezeStatus::Ended => "ended",
        };

        let ended_at = if matches!(status, FreezeStatus::Ended) {
            Some(Utc::now())
        } else {
            None
        };

        let row = sqlx::query!(
            r#"
            UPDATE freeze_records 
            SET status = $1, ended_at = $2, ended_by = $3
            WHERE id = $4
            RETURNING *
            "#,
            status_str,
            ended_at,
            ended_by,
            id
        )
        .fetch_optional(pool)
        .await?;

        match row {
            Some(row) => Ok(Some(FreezeRecord {
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
            })),
            None => Ok(None),
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
    pub async fn get_active_freezes(pool: &PgPool) -> Result<Vec<FreezeRecord>> {
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
    pub async fn is_frozen(pool: &PgPool, installation_id: i64, repository: &str) -> Result<bool> {
        let row = sqlx::query!(
            "SELECT EXISTS(SELECT 1 FROM freeze_records WHERE installation_id = $1 AND repository = $2 AND status = 'active')",
            installation_id,
            repository
        )
        .fetch_one(pool)
        .await?;
        Ok(row.exists.unwrap_or(false))
    }
}

/// Database operations for user permission records.
///
/// Permission records define what actions users can perform on repositories,
/// including freeze/unfreeze operations and emergency overrides.
impl PermissionRecord {
    /// Creates a new permission record in the database.
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool
    /// * `record` - The permission record to create
    ///
    /// # Returns
    ///
    /// Returns the created permission record on success.
    pub async fn create(pool: &PgPool, record: &PermissionRecord) -> Result<PermissionRecord> {
        let row = sqlx::query!(
            r#"
            INSERT INTO permission_records 
            (id, installation_id, repository, user_login, role, can_freeze, can_unfreeze, can_emergency_override, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
            record.id,
            record.installation_id,
            record.repository,
            record.user_login,
            record.role.to_string(),
            record.can_freeze,
            record.can_unfreeze,
            record.can_emergency_override,
            record.created_at
        )
        .fetch_one(pool)
        .await?;

        Ok(PermissionRecord {
            id: row.id,
            installation_id: row.installation_id,
            repository: row.repository,
            user_login: row.user_login,
            role: Role::from(row.role.as_str()),
            can_freeze: row.can_freeze,
            can_unfreeze: row.can_unfreeze,
            can_emergency_override: row.can_emergency_override,
            created_at: row.created_at,
        })
    }

    /// Retrieves a permission record by its ID.
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool
    /// * `id` - UUID of the permission record
    ///
    /// # Returns
    ///
    /// Returns the permission record if found, or `None` if not found.
    pub async fn get(pool: &PgPool, id: Uuid) -> Result<Option<PermissionRecord>> {
        let row = sqlx::query!("SELECT * FROM permission_records WHERE id = $1", id)
            .fetch_optional(pool)
            .await?;

        match row {
            Some(row) => Ok(Some(PermissionRecord {
                id: row.id,
                installation_id: row.installation_id,
                repository: row.repository,
                user_login: row.user_login,
                role: Role::from(row.role.as_str()),
                can_freeze: row.can_freeze,
                can_unfreeze: row.can_unfreeze,
                can_emergency_override: row.can_emergency_override,
                created_at: row.created_at,
            })),
            None => Ok(None),
        }
    }

    /// Retrieves a permission record for a specific user and repository.
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool
    /// * `installation_id` - GitHub installation ID
    /// * `repository` - Repository name in "owner/repo" format
    /// * `user_login` - GitHub username
    ///
    /// # Returns
    ///
    /// Returns the permission record if found, or `None` if the user has no permissions for the repository.
    pub async fn get_by_user_and_repo(
        pool: &PgPool,
        installation_id: i64,
        repository: &str,
        user_login: &str,
    ) -> Result<Option<PermissionRecord>> {
        let row = sqlx::query!(
            "SELECT * FROM permission_records WHERE installation_id = $1 AND repository = $2 AND user_login = $3",
            installation_id,
            repository,
            user_login
        )
        .fetch_optional(pool)
        .await?;

        match row {
            Some(row) => Ok(Some(PermissionRecord {
                id: row.id,
                installation_id: row.installation_id,
                repository: row.repository,
                user_login: row.user_login,
                role: Role::from(row.role.as_str()),
                can_freeze: row.can_freeze,
                can_unfreeze: row.can_unfreeze,
                can_emergency_override: row.can_emergency_override,
                created_at: row.created_at,
            })),
            None => Ok(None),
        }
    }

    pub async fn list(
        pool: &PgPool,
        installation_id: Option<i64>,
        repository: Option<&str>,
    ) -> Result<Vec<PermissionRecord>> {
        let mut query = "SELECT * FROM permission_records WHERE 1=1".to_string();
        let mut params = Vec::new();

        if let Some(inst_id) = installation_id {
            query.push_str(&format!(" AND installation_id = ${}", params.len() + 1));
            params.push(inst_id.to_string());
        }

        if let Some(repo) = repository {
            query.push_str(&format!(" AND repository = ${}", params.len() + 1));
            params.push(repo.to_string());
        }

        query.push_str(" ORDER BY created_at DESC");

        let mut sql_query = sqlx::query(&query);
        for param in &params {
            sql_query = sql_query.bind(param);
        }

        let rows = sql_query.fetch_all(pool).await?;

        let mut records = Vec::new();
        for row in rows {
            records.push(PermissionRecord {
                id: row.get("id"),
                installation_id: row.get("installation_id"),
                repository: row.get("repository"),
                user_login: row.get("user_login"),
                role: Role::from(row.get::<String, _>("role").as_str()),
                can_freeze: row.get("can_freeze"),
                can_unfreeze: row.get("can_unfreeze"),
                can_emergency_override: row.get("can_emergency_override"),
                created_at: row.get("created_at"),
            });
        }

        Ok(records)
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<bool> {
        let result = sqlx::query!("DELETE FROM permission_records WHERE id = $1", id)
            .execute(pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}

/// Database operations for command audit logs.
///
/// Command logs track all freeze/unfreeze commands executed through the system,
/// including their results and any error messages for audit purposes.
impl CommandLog {
    /// Creates a new command log entry in the database.
    ///
    /// # Arguments
    ///
    /// * `pool` - Database connection pool
    /// * `log` - The command log entry to create
    ///
    /// # Returns
    ///
    /// Returns the created command log entry on success.
    pub async fn create(pool: &PgPool, log: &CommandLog) -> Result<CommandLog> {
        let row = sqlx::query!(
            r#"
            INSERT INTO command_logs 
            (id, installation_id, repository, user_login, command, comment_id, result, error_message, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
            log.id,
            log.installation_id,
            log.repository,
            log.user_login,
            log.command,
            log.comment_id,
            log.result.to_string(),
            log.error_message,
            log.created_at
        )
        .fetch_one(pool)
        .await?;

        Ok(CommandLog {
            id: row.id,
            installation_id: row.installation_id,
            repository: row.repository,
            user_login: row.user_login,
            command: row.command,
            comment_id: row.comment_id,
            result: CommandResult::from(row.result.as_str()),
            error_message: row.error_message,
            created_at: row.created_at,
        })
    }

    pub async fn get(pool: &PgPool, id: Uuid) -> Result<Option<CommandLog>> {
        let row = sqlx::query!("SELECT * FROM command_logs WHERE id = $1", id)
            .fetch_optional(pool)
            .await?;

        match row {
            Some(row) => Ok(Some(CommandLog {
                id: row.id,
                installation_id: row.installation_id,
                repository: row.repository,
                user_login: row.user_login,
                command: row.command,
                comment_id: row.comment_id,
                result: CommandResult::from(row.result.as_str()),
                error_message: row.error_message,
                created_at: row.created_at,
            })),
            None => Ok(None),
        }
    }

    pub async fn list(
        pool: &PgPool,
        installation_id: Option<i64>,
        repository: Option<&str>,
        user_login: Option<&str>,
    ) -> Result<Vec<CommandLog>> {
        let mut query = "SELECT * FROM command_logs WHERE 1=1".to_string();
        let mut params = Vec::new();

        if let Some(inst_id) = installation_id {
            query.push_str(&format!(" AND installation_id = ${}", params.len() + 1));
            params.push(inst_id.to_string());
        }

        if let Some(repo) = repository {
            query.push_str(&format!(" AND repository = ${}", params.len() + 1));
            params.push(repo.to_string());
        }

        if let Some(user) = user_login {
            query.push_str(&format!(" AND user_login = ${}", params.len() + 1));
            params.push(user.to_string());
        }

        query.push_str(" ORDER BY created_at DESC");

        let mut sql_query = sqlx::query(&query);
        for param in &params {
            sql_query = sql_query.bind(param);
        }

        let rows = sql_query.fetch_all(pool).await?;

        let mut logs = Vec::new();
        for row in rows {
            logs.push(CommandLog {
                id: row.get("id"),
                installation_id: row.get("installation_id"),
                repository: row.get("repository"),
                user_login: row.get("user_login"),
                command: row.get("command"),
                comment_id: row.get("comment_id"),
                result: CommandResult::from(row.get::<String, _>("result").as_str()),
                error_message: row.get("error_message"),
                created_at: row.get("created_at"),
            });
        }

        Ok(logs)
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<bool> {
        let result = sqlx::query!("DELETE FROM command_logs WHERE id = $1", id)
            .execute(pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}
