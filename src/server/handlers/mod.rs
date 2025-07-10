use crate::server::AppState;
use axum::{extract::State, response::IntoResponse};
use tracing::info;

pub async fn get_rulesets(State(state): State<AppState>) -> impl IntoResponse {
    let installations = state.gh.get_installations().await.unwrap();
    let repos = state
        .gh
        .get_installation_repositories(installations[0].id.0)
        .await
        .unwrap();
    info!(
        "Found {} repositories for installation {}",
        repos.len(),
        installations[0].id.0
    );
}
