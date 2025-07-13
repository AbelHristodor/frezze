use std::sync::Arc;

use crate::{
    github::Github,
    server::{AppState, middlewares::gh_event::GitHubEventExt},
};
use axum::{
    extract::{Request, State},
    response::{IntoResponse, Response, Result},
};
use octocrab::{
    models::webhook_events::*,
    params::checks::{CheckRunConclusion, CheckRunStatus},
};
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

pub async fn webhook(State(state): State<AppState>, req: Request) -> Result<Response> {
    info!("Received webhook event");
    let ctx = req
        .github_event()
        .ok_or(axum::http::StatusCode::BAD_REQUEST)?;

    match &ctx.event.kind {
        WebhookEventType::PullRequest => handle_pull_request(&ctx.event, &state.gh).await,
        _ => handle_unknown_event(&ctx.event).await,
    }
}

async fn handle_unknown_event(event: &WebhookEvent) -> Result<Response> {
    info!("Received unknown event type: {:?}", event.kind);
    Ok(axum::http::StatusCode::OK.into_response())
}

async fn handle_pull_request(event: &WebhookEvent, gh: &Arc<Github>) -> Result<Response> {
    let installation_id = get_installation_id(event)?;
    let repository = event.repository.as_ref().ok_or("Repository not found")?;
    let repo = &repository.name;
    let owner = &repository.owner.as_ref().ok_or("Owner not found")?.login;

    let WebhookEventPayload::PullRequest(pr_event) = &event.specific else {
        tracing::error!("Expected PullRequest event, got: {:?}", event.kind);
        return Err(axum::http::StatusCode::BAD_REQUEST)?;
    };

    info!("Received PR event for repository: {}/{}", owner, repo);

    let head_sha = &pr_event.pull_request.head.sha;

    gh.create_check_run(
        owner,
        repo,
        head_sha,
        CheckRunStatus::Completed,
        CheckRunConclusion::Success,
        installation_id,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to create check run: {:?}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(axum::http::StatusCode::OK.into_response())
}

fn get_installation_id(event: &WebhookEvent) -> Result<u64, axum::http::StatusCode> {
    event
        .installation
        .as_ref()
        .map(|i| i.id().0)
        .ok_or(axum::http::StatusCode::BAD_REQUEST)
}
