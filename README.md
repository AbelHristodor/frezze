# GitHub Freeze Bot - Rust Project Structure

## Project Root Structure

```
frezze/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── .env.example
├── .gitignore
├── Dockerfile
├── docker-compose.yml
├── migrations/
│   └── 001_initial.sql
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── config/
│   │   ├── mod.rs
│   │   └── settings.rs
│   ├── github/
│   │   ├── mod.rs
│   │   ├── auth.rs
│   │   ├── client.rs
│   │   ├── webhooks.rs
│   │   └── types.rs
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── parser.rs
│   │   ├── freeze.rs
│   │   ├── unfreeze.rs
│   │   ├── status.rs
│   │   └── help.rs
│   ├── freeze/
│   │   ├── mod.rs
│   │   ├── manager.rs
│   │   ├── scheduler.rs
│   │   └── protection.rs
│   ├── database/
│   │   ├── mod.rs
│   │   ├── models.rs
│   │   ├── connection.rs
│   │   └── migrations.rs
│   ├── handlers/
│   │   ├── mod.rs
│   │   ├── webhook.rs
│   │   ├── comment.rs
│   │   └── event.rs
│   ├── notifications/
│   │   ├── mod.rs
│   │   ├── slack.rs
│   │   └── discord.rs
│   ├── web/
│   │   ├── mod.rs
│   │   ├── routes.rs
│   │   ├── middleware.rs
│   │   └── responses.rs
│   ├── utils/
│   │   ├── mod.rs
│   │   ├── time.rs
│   │   └── validation.rs
│   └── errors/
│       ├── mod.rs
│       └── types.rs
├── tests/
│   ├── integration/
│   │   ├── mod.rs
│   │   ├── webhook_tests.rs
│   │   ├── command_tests.rs
│   │   └── freeze_tests.rs
│   └── fixtures/
│       ├── webhook_payloads.json
│       └── test_data.sql
├── docs/
│   ├── API.md
│   ├── COMMANDS.md
│   └── DEPLOYMENT.md
└── scripts/
    ├── setup.sh
    └── deploy.sh
```

## Cargo.toml

```toml
[package]
name = "frezze"
version = "0.1.0"
edition = "2021"
authors = ["Your Name <your.email@example.com>"]
description = "GitHub App for managing repository freezes with comment commands"
license = "MIT"
repository = "https://github.com/yourusername/frezze"

[dependencies]
# Web framework
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace"] }
hyper = "1.0"
tokio = { version = "1.0", features = ["full"] }

# GitHub API
octocrab = "0.38"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
reqwest = { version = "0.11", features = ["json"] }

# JWT for GitHub App authentication
jsonwebtoken = "9.0"
base64 = "0.21"

# Database
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "chrono", "uuid"] }
uuid = { version = "1.0", features = ["serde", "v4"] }

# Configuration
config = "0.13"
dotenvy = "0.15"

# Logging and tracing
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-actix-web = "0.7"

# Time handling
chrono = { version = "0.4", features = ["serde"] }

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Async utilities
futures = "0.3"
tokio-cron-scheduler = "0.9"

# Validation
validator = { version = "0.16", features = ["derive"] }

# Security
hmac = "0.12"
sha2 = "0.10"
hex = "0.4"

[dev-dependencies]
mockall = "0.11"
wiremock = "0.5"
tempfile = "3.0"
```

## Key Source Files

### src/main.rs

```rust
use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use tracing_subscriber;

mod config;
mod github;
mod commands;
mod freeze;
mod database;
mod handlers;
mod notifications;
mod web;
mod utils;
mod errors;

use config::Settings;
use database::Database;
use github::GitHubClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::init();

    let settings = Settings::new()?;
    let database = Database::new(&settings.database_url).await?;
    let github_client = GitHubClient::new(&settings.github)?;

    let app = Router::new()
        .route("/", get(web::routes::health))
        .route("/webhooks/github", post(handlers::webhook::handle_webhook))
        .route("/api/freeze", post(web::routes::freeze_repos))
        .route("/api/status", get(web::routes::get_status))
        .layer(TraceLayer::new_for_http())
        .with_state(AppState {
            database,
            github_client,
            settings,
        });

    let addr = SocketAddr::from(([0, 0, 0, 0], settings.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    tracing::info!("Server starting on {}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}

#[derive(Clone)]
pub struct AppState {
    pub database: Database,
    pub github_client: GitHubClient,
    pub settings: Settings,
}
```

### src/commands/mod.rs

```rust
pub mod parser;
pub mod freeze;
pub mod unfreeze;
pub mod status;
pub mod help;

use crate::errors::BotError;
use crate::github::types::CommentContext;
use crate::AppState;

#[derive(Debug, Clone)]
pub enum Command {
    Freeze {
        duration: Option<chrono::Duration>,
        repos: Vec<String>,
        reason: Option<String>,
    },
    Unfreeze {
        repos: Vec<String>,
        reason: Option<String>,
    },
    Status {
        repos: Vec<String>,
    },
    Help,
    Schedule {
        at: chrono::DateTime<chrono::Utc>,
        command: Box<Command>,
    },
}

pub async fn execute_command(
    state: &AppState,
    command: Command,
    context: CommentContext,
) -> Result<String, BotError> {
    match command {
        Command::Freeze { duration, repos, reason } => {
            freeze::execute(state, context, duration, repos, reason).await
        }
        Command::Unfreeze { repos, reason } => {
            unfreeze::execute(state, context, repos, reason).await
        }
        Command::Status { repos } => {
            status::execute(state, context, repos).await
        }
        Command::Help => {
            help::execute().await
        }
        Command::Schedule { at, command } => {
            // Schedule the command for later execution
            todo!("Implement command scheduling")
        }
    }
}
```

### src/github/types.rs

```rust
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentContext {
    pub installation_id: u64,
    pub repository: Repository,
    pub sender: User,
    pub comment: Comment,
    pub issue_or_pr: IssueOrPr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub owner: User,
    pub default_branch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub login: String,
    pub avatar_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: u64,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum IssueOrPr {
    Issue(Issue),
    PullRequest(PullRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub id: u64,
    pub number: u32,
    pub title: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    pub id: u64,
    pub number: u32,
    pub title: String,
    pub state: String,
    pub head: Branch,
    pub base: Branch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub ref_name: String,
    pub sha: String,
    pub repo: Repository,
}
```

### src/freeze/manager.rs

```rust
use crate::database::models::FreezeRecord;
use crate::github::GitHubClient;
use crate::errors::BotError;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

pub struct FreezeManager {
    pub github_client: GitHubClient,
    pub database: crate::database::Database,
}

impl FreezeManager {
    pub async fn freeze_repository(
        &self,
        installation_id: u64,
        repo_full_name: &str,
        duration: Option<chrono::Duration>,
        reason: Option<String>,
        initiated_by: &str,
    ) -> Result<FreezeRecord, BotError> {
        // 1. Create branch protection rules
        self.apply_freeze_protection(installation_id, repo_full_name).await?;
        
        // 2. Record freeze in database
        let freeze_record = FreezeRecord {
            id: uuid::Uuid::new_v4(),
            repository: repo_full_name.to_string(),
            installation_id,
            started_at: Utc::now(),
            expires_at: duration.map(|d| Utc::now() + d),
            reason,
            initiated_by: initiated_by.to_string(),
            status: "active".to_string(),
            created_at: Utc::now(),
        };
        
        self.database.create_freeze_record(&freeze_record).await?;
        
        // 3. Send notifications
        self.send_freeze_notification(&freeze_record).await?;
        
        Ok(freeze_record)
    }
    
    pub async fn unfreeze_repository(
        &self,
        installation_id: u64,
        repo_full_name: &str,
        reason: Option<String>,
        initiated_by: &str,
    ) -> Result<(), BotError> {
        // 1. Remove branch protection rules
        self.remove_freeze_protection(installation_id, repo_full_name).await?;
        
        // 2. Update freeze record in database
        self.database.end_freeze_record(repo_full_name, reason, initiated_by).await?;
        
        // 3. Send notifications
        self.send_unfreeze_notification(repo_full_name, initiated_by).await?;
        
        Ok(())
    }
    
    async fn apply_freeze_protection(
        &self,
        installation_id: u64,
        repo_full_name: &str,
    ) -> Result<(), BotError> {
        // Implementation to add strict branch protection rules
        // This would use the GitHub API to modify branch protection
        todo!("Implement freeze protection rules")
    }
    
    async fn remove_freeze_protection(
        &self,
        installation_id: u64,
        repo_full_name: &str,
    ) -> Result<(), BotError> {
        // Implementation to restore original branch protection rules
        todo!("Implement protection rule restoration")
    }
    
    async fn send_freeze_notification(&self, freeze_record: &FreezeRecord) -> Result<(), BotError> {
        // Send notifications to Slack/Discord/etc.
        todo!("Implement notifications")
    }
    
    async fn send_unfreeze_notification(
        &self,
        repo_full_name: &str,
        initiated_by: &str,
    ) -> Result<(), BotError> {
        // Send unfreeze notifications
        todo!("Implement unfreeze notifications")
    }
}
```

### src/database/models.rs

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FreezeRecord {
    pub id: Uuid,
    pub repository: String,
    pub installation_id: i64,
    pub started_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub reason: Option<String>,
    pub initiated_by: String,
    pub ended_by: Option<String>,
    pub status: String, // active, expired, ended
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PermissionRecord {
    pub id: Uuid,
    pub installation_id: i64,
    pub repository: String,
    pub user_login: String,
    pub role: String, // admin, maintainer, contributor
    pub can_freeze: bool,
    pub can_unfreeze: bool,
    pub can_emergency_override: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CommandLog {
    pub id: Uuid,
    pub installation_id: i64,
    pub repository: String,
    pub user_login: String,
    pub command: String,
    pub comment_id: i64,
    pub result: String, // success, error, unauthorized
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}
```

## Command Examples

The bot will support these commands in PR/Issue comments:

- `/freeze` - Freeze all repos in org
- `/freeze repo1 repo2` - Freeze specific repos
- `/freeze --duration 2h --reason "Release v1.2.3"` - Freeze with duration and reason
- `/unfreeze` - Unfreeze all frozen repos
- `/unfreeze repo1 --reason "Hotfix applied"` - Unfreeze specific repo
- `/freeze-status` - Show current freeze status
- `/freeze-help` - Show command help
- `/schedule-freeze --at "2024-01-15T10:00:00Z"` - Schedule freeze

## Database Schema (migrations/001_initial.sql)

```sql
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE freeze_records (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    repository VARCHAR NOT NULL,
    installation_id BIGINT NOT NULL,
    started_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ,
    ended_at TIMESTAMPTZ,
    reason TEXT,
    initiated_by VARCHAR NOT NULL,
    ended_by VARCHAR,
    status VARCHAR NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE permission_records (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    installation_id BIGINT NOT NULL,
    repository VARCHAR NOT NULL,
    user_login VARCHAR NOT NULL,
    role VARCHAR NOT NULL,
    can_freeze BOOLEAN NOT NULL DEFAULT FALSE,
    can_unfreeze BOOLEAN NOT NULL DEFAULT FALSE,
    can_emergency_override BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(installation_id, repository, user_login)
);

CREATE TABLE command_logs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    installation_id BIGINT NOT NULL,
    repository VARCHAR NOT NULL,
    user_login VARCHAR NOT NULL,
    command VARCHAR NOT NULL,
    comment_id BIGINT NOT NULL,
    result VARCHAR NOT NULL,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_freeze_records_repo ON freeze_records(repository, status);
CREATE INDEX idx_freeze_records_installation ON freeze_records(installation_id);
CREATE INDEX idx_permission_records_user ON permission_records(installation_id, user_login);
CREATE INDEX idx_command_logs_repo ON command_logs(repository, created_at);
```

This structure provides a solid foundation for a GitHub App that can manage repository freezes through comment commands, with proper database storage, permissions, logging, and extensibility for notifications and scheduling.
