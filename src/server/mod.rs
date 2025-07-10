use std::{path::Path, sync::Arc};

use crate::server::config::ServerConfig;
use crate::{database::Database, github};
use axum::Router;
use axum::routing::get;
use base64::Engine;
use base64::engine::general_purpose;
use sqlx::migrate::Migrator;
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::{Level, info};

pub mod config;
pub mod handlers;

pub struct Server {
    pub address: std::net::Ipv4Addr,
    pub port: u16,
    pub database_url: String,
    pub gh_app_id: u64,
    pub migrations_path: String,
    pub gh_private_key_path: Option<String>,
    pub gh_private_key_base64: Option<String>,

    db: Arc<Database>,
}

#[derive(Clone)]
pub struct AppState {
    db: Arc<Database>,
    gh: Arc<github::Github>,
}

impl Server {
    pub async fn new(config: &ServerConfig) -> Result<Self, anyhow::Error> {
        // Validate the configuration
        config.validate()?;

        let mut db = Database::new(config.database_url.clone(), 10);
        db.connect().await?;

        // Initialize the server with the provided configuration
        let server = Server {
            address: config.address,
            port: config.port,
            database_url: config.database_url.clone(),
            gh_app_id: config.gh_app_id,
            migrations_path: config.migrations_path.clone(),
            gh_private_key_path: config.gh_private_key_path.clone(),
            gh_private_key_base64: config.gh_private_key_base64.clone(),
            db: Arc::new(db),
        };

        Ok(server)
    }

    async fn init(&self) -> Result<(), anyhow::Error> {
        // Run database migrations
        Migrator::new(Path::new(&self.migrations_path))
            .await?
            .run(self.db.get_connection()?)
            .await?;

        info!("Database migrations applied successfully");
        Ok(())
    }

    pub async fn start(&self) -> Result<(), anyhow::Error> {
        self.init().await?;

        let gh_private_key: &[u8] = if let Some(path) = &self.gh_private_key_path {
            info!("Using key from path");
            let key_path = Path::new(path);
            if !key_path.exists() {
                return Err(anyhow::anyhow!(
                    "GitHub private key file does not exist: {}",
                    path
                ));
            }
            &std::fs::read(key_path).map_err(|e| {
                anyhow::anyhow!("Failed to read GitHub private key from file: {}", e)
            })?
        } else if let Some(base64_key) = &self.gh_private_key_base64 {
            info!("Using key from base64 string");
            &general_purpose::STANDARD.decode(base64_key).map_err(|e| {
                anyhow::anyhow!("Failed to decode GitHub private key from base64: {}", e)
            })?
        } else {
            return Err(anyhow::anyhow!("GitHub private key not provided"));
        };

        let github = github::Github::new(self.gh_app_id, gh_private_key).await;

        let listener = tokio::net::TcpListener::bind((self.address, self.port)).await?;
        info!("Server started on {}", listener.local_addr().unwrap());

        axum::serve(
            listener,
            get_router(AppState {
                db: self.db.clone(),
                gh: Arc::new(github),
            }),
        )
        .await?;

        Ok(())
    }
}

/// Creates the Axum router with the necessary routes and middleware.
fn get_router(state: AppState) -> Router {
    let cors = tower_http::cors::CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    Router::new()
        .route("/", get(handlers::get_rulesets))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().include_headers(true))
                .on_request(DefaultOnRequest::new().level(Level::INFO))
                .on_response(
                    DefaultOnResponse::new()
                        .level(Level::INFO)
                        .latency_unit(tower_http::LatencyUnit::Micros),
                ),
        )
        .layer(cors)
        .with_state(state)
}
