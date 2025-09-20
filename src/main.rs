use std::sync::Arc;

use tracing::info;

mod cli;
mod config;
mod database;
mod freezer;
mod github;
mod handlers;
mod permissions;
mod repository;
mod worker;

use octofer::{
    Octofer,
    config::GitHubConfig,
    github::{GitHubAuth, GitHubClient},
};

use crate::{config::UserPermissionsConfig, database::Database};

struct AppState {
    database: Arc<Database>,
    user_config: Option<Arc<UserPermissionsConfig>>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // Start the application
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
        let mut app = Octofer::new(config).await?;

        // TODO: create a helper function
        let db = Database::new(
            "postgres://postgres:postgres@localhost:5432/postgres",
            "migrations",
            10,
        )
        .connect()
        .await?;

        // Load permission config file
        let conf = UserPermissionsConfig::load_from_file("./users.yaml")?;

        let state = AppState {
            database: Arc::new(db),
            user_config: Some(Arc::new(conf)),
        };

        // Start the worker that refreshes PRs status checks in the bg
        let worker_db = state.database.clone();
        tokio::spawn(async move {
            // Start the freeze scheduler worker
            worker(worker_db).await;
        });

        // Attach on the issue_comment handler
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

async fn worker(db: Arc<Database>) {
    let gh_cfg = GitHubConfig::from_env().expect("Unable to load github cfg");
    let gh_auth = GitHubAuth::from_config(&gh_cfg);
    let gh = GitHubClient::new(gh_auth)
        .await
        .expect("Unable to start github client");
    let worker = worker::FreezeSchedulerWorker::new(db, gh.into());
    worker.start().await;
}
