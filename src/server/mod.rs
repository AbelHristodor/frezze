use crate::server::config::ServerConfig;
use crate::{database::Database, github};
use anyhow::{Result, anyhow};
use axum::Router;
use axum::routing::get;
use base64::Engine;
use base64::engine::general_purpose;
use std::{path::Path, sync::Arc};
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::{Level, info};

pub mod config;
mod handlers;

pub struct Server {
    pub address: std::net::Ipv4Addr,
    pub port: u16,
    pub gh_app_id: u64,
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
    pub async fn new(config: &ServerConfig) -> Result<Self> {
        // Validate the configuration
        config.validate()?;

        let db = Database::new(&config.database_url, &config.migrations_path, 10)
            .connect()
            .await?
            .migrate()
            .await?;

        // Initialize the server with the provided configuration
        let server = Server {
            address: config.address,
            port: config.port,
            gh_app_id: config.gh_app_id,
            gh_private_key_path: config.gh_private_key_path.clone(),
            gh_private_key_base64: config.gh_private_key_base64.clone(),
            db: Arc::new(db),
        };

        Ok(server)
    }

    pub async fn start(&self) -> Result<()> {
        let github = github::Github::new(self.gh_app_id, &self.read_gh_private_key()?).await;

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

    pub fn read_gh_private_key(&self) -> Result<Vec<u8>> {
        if let Some(path) = &self.gh_private_key_path {
            info!("Using key from path");
            let key_path = Path::new(path);
            if !key_path.exists() {
                return Err(anyhow!("GitHub private key file does not exist: {}", path));
            }
            // Return Vec<u8> directly
            std::fs::read(key_path)
                .map_err(|e| anyhow!("Failed to read GitHub private key from file: {}", e))
        } else if let Some(base64_key) = &self.gh_private_key_base64 {
            info!("Using key from base64 string");
            // Return Vec<u8> directly
            general_purpose::STANDARD
                .decode(base64_key)
                .map_err(|e| anyhow!("Failed to decode GitHub private key from base64: {}", e))
        } else {
            Err(anyhow!("GitHub private key not provided"))
        }
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
