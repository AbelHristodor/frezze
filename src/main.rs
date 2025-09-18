use std::sync::Arc;

use tracing::info;
use tracing_subscriber::EnvFilter;

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

    start().await?;

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

        let a = freezer::manager::FreezeManager::new(db);

        app.on_issue_comment(handlers::issue_comment_handler, Arc::new(a))
            .await;
    });

    // Wait for the server to finish starting
    handle
        .await
        .map_err(|e| anyhow::anyhow!("Server failed to start: {:?}", e))?;

    Ok(())
}
