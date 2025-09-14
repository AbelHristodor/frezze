use clap::Parser;
use tracing::info;
use tracing_subscriber::EnvFilter;
use std::sync::Arc;

use crate::server::{Server, config::ServerConfig};
use crate::database::Database;
use crate::github::Github;
use crate::freezer::manager::FreezeManager;

mod cli;
mod database;
mod freezer;
mod github;
mod server;

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
                    let cfg = ServerConfig {
                        address,
                        port,
                        database_url,
                        gh_app_id,
                        migrations_path,
                        gh_private_key_path,
                        gh_private_key_base64,
                    };

                    // Start the server
                    info!("Starting the server...");
                    start(cfg).await?
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
        cli::Commands::Refresh { command } => {
            match command {
                cli::RefreshCommands::All {
                    database_url,
                    gh_app_id,
                    gh_private_key_path,
                    gh_private_key_base64,
                } => {
                    info!("Refreshing all PRs...");
                    refresh_all_prs(database_url, gh_app_id, gh_private_key_path, gh_private_key_base64).await?;
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
                    ).await?;
                }
            }
        }
    }

    Ok(())
}

async fn start(cfg: ServerConfig) -> Result<(), anyhow::Error> {
    let handle = tokio::spawn(async move {
        let server = Server::new(&cfg).await.unwrap();
        server.start().await.unwrap();
    });

    // Wait for the server to finish starting
    handle
        .await
        .map_err(|e| anyhow::anyhow!("Server failed to start: {:?}", e))?;

    Ok(())
}

async fn refresh_all_prs(
    database_url: String,
    gh_app_id: u64,
    gh_private_key_path: Option<String>,
    gh_private_key_base64: Option<String>,
) -> Result<(), anyhow::Error> {
    let db = Arc::new(
        Database::new(&database_url, "migrations", 10)
            .connect()
            .await?
            .migrate()
            .await?
    );
    let github_key = get_github_key(gh_private_key_path, gh_private_key_base64)?;
    let github = Arc::new(Github::new(gh_app_id, &github_key).await);
    let freeze_manager = FreezeManager::new(db, github);

    freeze_manager.refresh_prs().await?;
    info!("Successfully refreshed all PRs");
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
    let db = Arc::new(
        Database::new(&database_url, "migrations", 10)
            .connect()
            .await?
            .migrate()
            .await?
    );
    let github_key = get_github_key(gh_private_key_path, gh_private_key_base64)?;
    let github = Arc::new(Github::new(gh_app_id, &github_key).await);
    let freeze_manager = FreezeManager::new(db, github);

    freeze_manager.refresh_repository_prs(installation_id, &repository).await?;
    info!("Successfully refreshed PRs for repository: {}", repository);
    Ok(())
}

fn get_github_key(
    gh_private_key_path: Option<String>,
    gh_private_key_base64: Option<String>,
) -> Result<Vec<u8>, anyhow::Error> {
    if let Some(path) = gh_private_key_path {
        std::fs::read(path).map_err(|e| anyhow::anyhow!("Failed to read GitHub private key file: {}", e))
    } else if let Some(base64_key) = gh_private_key_base64 {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode(base64_key)
            .map_err(|e| anyhow::anyhow!("Failed to decode GitHub private key from base64: {}", e))
    } else {
        Err(anyhow::anyhow!("Either GitHub private key path or base64 key must be provided"))
    }
}
