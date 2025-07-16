use axum::{
    extract::{Request, State},
    response::{IntoResponse, Response, Result},
};
use octocrab::models::webhook_events::{WebhookEvent, WebhookEventType};
use tracing::info;

use crate::server::{AppState, middlewares::gh_event::GitHubEventExt};

mod issue;
mod pr;

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

pub async fn webhook(State(state): State<AppState>, req: Request) -> Result<Response> {
    info!("Received webhook event");
    let ctx = req
        .github_event()
        .ok_or(axum::http::StatusCode::BAD_REQUEST)?;

    match &ctx.event.kind {
        WebhookEventType::PullRequest => pr::handle_pull_request(&ctx, &state).await,
        WebhookEventType::IssueComment => issue::handle_issue_comment(&ctx, &state).await,
        _ => handle_unknown_event(&ctx.event).await,
    }
}

async fn handle_unknown_event(event: &WebhookEvent) -> Result<Response> {
    info!("Received unknown event type: {:?}", event.kind);
    Ok(axum::http::StatusCode::OK.into_response())
}
