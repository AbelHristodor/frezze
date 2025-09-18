use std::sync::Arc;

use tracing::info;

mod cli;
mod database;
mod freezer;
mod github;
mod handlers;
mod repository;
mod worker;

use octofer::Octofer;

use crate::database::Database;

struct AppState {
    database: Arc<Database>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    start().await?;

    Ok(())
}

async fn start() -> Result<(), anyhow::Error> {
    let handle = tokio::spawn(async move {
        // Initialize tracing using configuration
        let config = octofer::Config::from_env().unwrap_or_else(|_| octofer::Config::default());

        // Initialize logging based on configuration
        config.init_logging();

        info!("Starting Octofer app");

        // Create a new Octofer app with the configuration
        let mut app = Octofer::new(config).await.unwrap_or_else(|e| {
            info!("Failed to create app with config, using default: {:?}", e);
            Octofer::new_default()
        });

        let db = Database::new(
            "postgres://postgres:postgres@localhost:5432/postgres",
            "migrations",
            10,
        )
        .connect()
        .await
        .unwrap();

        let state = AppState {
            database: Arc::new(db),
        };

        // Start the freeze scheduler worker
        let worker_db = state.database.clone();
        let worker_github = app.github_client(); // Assuming we can get the GitHub client from the app
        let worker = worker::FreezeSchedulerWorker::new(worker_db, worker_github);
        
        tokio::spawn(async move {
            worker.start().await;
        });

        app.on_issue_comment(handlers::issue_comment_handler, Arc::new(state))
            .await;

        app.start().await
    });

    // Wait for the server to finish starting
    handle
        .await?
        .map_err(|e| anyhow::anyhow!("Server failed to start: {:?}", e))?;

    Ok(())
}
