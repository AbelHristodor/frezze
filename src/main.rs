use clap::Parser;
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::server::{Server, config::ServerConfig};

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
                .or_else(|_| EnvFilter::try_new("freeze_bot=info,tower_http=debug"))
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
