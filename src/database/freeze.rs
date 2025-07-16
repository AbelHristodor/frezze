use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::database::models::{
    CommandLog, CommandResult, FreezeRecord, FreezeStatus, PermissionRecord, Role,
};

impl FreezeRecord {
    pub async fn create(pool: &PgPool, record: &FreezeRecord) -> Result<FreezeRecord> {
        let status_str = match record.status {
            FreezeStatus::Active => "active",
            FreezeStatus::Expired => "expired",
            FreezeStatus::Ended => "ended",
        };

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
            status_str,
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

    pub async fn get(pool: &PgPool, id: Uuid) -> Result<Option<FreezeRecord>> {
        let row = sqlx::query!("SELECT * FROM freeze_records WHERE id = $1", id)
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

    pub async fn list(
        pool: &PgPool,
        installation_id: Option<i64>,
        repository: Option<&str>,
        active: Option<bool>,
    ) -> Result<Vec<FreezeRecord>> {
        let mut query = "SELECT * FROM freeze_records WHERE 1=1".to_string();
        let mut params = Vec::new();

        if let Some(inst_id) = installation_id {
            query.push_str(&format!(" AND installation_id = ${}", params.len() + 1));
            params.push(inst_id.to_string());
        }

        if let Some(repo) = repository {
            query.push_str(&format!(" AND repository = ${}", params.len() + 1));
            params.push(repo.to_string());
        }

        if let Some(is_active) = active {
            if is_active {
                query.push_str(" AND status = 'active'");
            }
        }

        query.push_str(" ORDER BY created_at DESC");

        let mut sql_query = sqlx::query(&query);
        for param in &params {
            sql_query = sql_query.bind(param);
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

    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<bool> {
        let result = sqlx::query!("DELETE FROM freeze_records WHERE id = $1", id)
            .execute(pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

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

// PermissionRecord CRUD operations
impl PermissionRecord {
    pub async fn create(pool: &PgPool, record: &PermissionRecord) -> Result<PermissionRecord> {
        let role_str = match record.role {
            Role::Admin => "admin",
            Role::Maintainer => "maintainer",
            Role::Contributor => "contributor",
        };

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
            role_str,
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

// CommandLog CRUD operations
impl CommandLog {
    pub async fn create(pool: &PgPool, log: &CommandLog) -> Result<CommandLog> {
        let result_str = match log.result {
            CommandResult::Success => "success",
            CommandResult::Error => "error",
            CommandResult::Unauthorized => "unauthorized",
        };

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
            result_str,
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

// Helper functions for creating new records
pub fn new_freeze_record(
    repository: String,
    installation_id: i64,
    started_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
    reason: Option<String>,
    initiated_by: String,
) -> FreezeRecord {
    FreezeRecord {
        id: Uuid::new_v4(),
        repository,
        installation_id,
        started_at,
        expires_at,
        ended_at: None,
        reason,
        initiated_by,
        ended_by: None,
        status: FreezeStatus::Active,
        created_at: Utc::now(),
    }
}

pub fn new_permission_record(
    installation_id: i64,
    repository: String,
    user_login: String,
    role: Role,
    can_freeze: bool,
    can_unfreeze: bool,
    can_emergency_override: bool,
) -> PermissionRecord {
    PermissionRecord {
        id: Uuid::new_v4(),
        installation_id,
        repository,
        user_login,
        role,
        can_freeze,
        can_unfreeze,
        can_emergency_override,
        created_at: Utc::now(),
    }
}

pub fn new_command_log(
    installation_id: i64,
    repository: String,
    user_login: String,
    command: String,
    comment_id: i64,
    result: CommandResult,
    error_message: Option<String>,
) -> CommandLog {
    CommandLog {
        id: Uuid::new_v4(),
        installation_id,
        repository,
        user_login,
        command,
        comment_id,
        result,
        error_message,
        created_at: Utc::now(),
    }
}
