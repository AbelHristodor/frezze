use clap::Parser;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::database::Database;
use crate::freezer::manager::FreezeManager;
use crate::github::Github;

mod cli;
mod database;
mod freezer;
mod github;
mod handlers;
mod repository;

use octofer::Octofer;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // Setup tracing subscriber
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .or_else(|_| EnvFilter::try_new("frezze=info,tower_http=debug"))
                .unwrap(),
        )
        .compact()
        .init();

    // Parse command line arguments
    let cmd = cli::Cli::parse();

    // Match the command and execute the corresponding logic
    match cmd.command {
        cli::Commands::Server { command } => {
            match command {
                cli::ServerCommands::Start {
                    address,
                    port,
                    database_url,
                    migrations_path,
                    gh_app_id,
                    gh_private_key_path,
                    gh_private_key_base64,
                } => {
                    // Start the server
                    info!("Starting the server...");
                    start().await?
                }
            }
        }
        cli::Commands::Webhook { command } => {
            match command {
                cli::WebhookCommands::Start => {
                    // Start the webhook server
                    info!("Starting the webhook...");
                }
            }
        }
        cli::Commands::Refresh { command } => match command {
            cli::RefreshCommands::All {
                database_url,
                gh_app_id,
                gh_private_key_path,
                gh_private_key_base64,
            } => {
                info!("Refreshing all active freeze PRs...");
                refresh_all_active_freezes(
                    database_url,
                    gh_app_id,
                    gh_private_key_path,
                    gh_private_key_base64,
                )
                .await?;
            }
            cli::RefreshCommands::Repository {
                repository,
                installation_id,
                database_url,
                gh_app_id,
                gh_private_key_path,
                gh_private_key_base64,
            } => {
                info!("Refreshing PRs for repository: {}", repository);
                refresh_repository_prs(
                    repository,
                    installation_id,
                    database_url,
                    gh_app_id,
                    gh_private_key_path,
                    gh_private_key_base64,
                )
                .await?;
            }
        },
    }

    Ok(())
}

async fn start() -> Result<(), anyhow::Error> {
    let handle = tokio::spawn(async move {
        // Initialize tracing using configuration
        let config = octofer::Config::from_env().unwrap_or_else(|_| octofer::Config::default());

        // Initialize logging based on configuration
        config.init_logging();

        info!("Starting Octofer app: example-github-app");

        // Create a new Octofer app with the configuration
        let mut app = Octofer::new(config).await.unwrap_or_else(|_| {
            info!("Failed to create app with config, using default");
            Octofer::new_default()
        });

        app.on_issue_comment(handlers::issue_comment_handler).await;
    });

    // Wait for the server to finish starting
    handle
        .await
        .map_err(|e| anyhow::anyhow!("Server failed to start: {:?}", e))?;

    Ok(())
}

fn get_github_key(
    gh_private_key_path: Option<String>,
    gh_private_key_base64: Option<String>,
) -> Result<Vec<u8>, anyhow::Error> {
    if let Some(path) = gh_private_key_path {
        std::fs::read(path)
            .map_err(|e| anyhow::anyhow!("Failed to read GitHub private key file: {}", e))
    } else if let Some(base64_key) = gh_private_key_base64 {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode(base64_key)
            .map_err(|e| anyhow::anyhow!("Failed to decode GitHub private key from base64: {}", e))
    } else {
        Err(anyhow::anyhow!(
            "Either GitHub private key path or base64 key must be provided"
        ))
    }
}

async fn refresh_all_active_freezes(
    database_url: String,
    gh_app_id: u64,
    gh_private_key_path: Option<String>,
    gh_private_key_base64: Option<String>,
) -> Result<(), anyhow::Error> {
    // Get GitHub key
    let github_key = get_github_key(gh_private_key_path, gh_private_key_base64)?;

    // Initialize database
    let db = Arc::new(Database::new(&database_url, "migrations", 10));

    // Initialize GitHub client
    let github = Arc::new(Github::new(gh_app_id, &github_key).await);

    // Create freeze manager
    let freeze_manager = FreezeManager::new(db, github);

    // Refresh all active freezes
    freeze_manager.refresh_all_active_freezes().await?;

    info!("All active freeze PRs refreshed successfully");
    Ok(())
}

async fn refresh_repository_prs(
    repository: String,
    installation_id: i64,
    database_url: String,
    gh_app_id: u64,
    gh_private_key_path: Option<String>,
    gh_private_key_base64: Option<String>,
) -> Result<(), anyhow::Error> {
    // Get GitHub key
    let github_key = get_github_key(gh_private_key_path, gh_private_key_base64)?;

    // Initialize database
    let db = Arc::new(Database::new(&database_url, "migrations", 10));

    // Initialize GitHub client
    let github = Arc::new(Github::new(gh_app_id, &github_key).await);

    // Create freeze manager
    let freeze_manager = FreezeManager::new(db, github);

    // Refresh repository PRs
    freeze_manager
        .refresh_repository_prs(installation_id, &repository)
        .await?;

    info!("Repository {} PRs refreshed successfully", repository);
    Ok(())
}
