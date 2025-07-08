use crate::server::AppState;
use axum::{Json, extract::State, response::IntoResponse};
use tracing::info;

pub async fn get_pull_requests(State(state): State<AppState>) -> impl IntoResponse {
    let installations = state.gh.get_installations().await.unwrap();
    if installations.is_empty() {
        return Json("".to_string());
    }
    let repos = state
        .gh
        .get_installation_repositories(installations[0].id.0)
        .await
        .unwrap();
    let repo = &repos[0];
    info!(
        "Found {} repositories for installation {}",
        repos.len(),
        installations[0].id.0
    );
    Json("hello world".to_string())
}
