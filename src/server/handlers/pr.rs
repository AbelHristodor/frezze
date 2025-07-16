use crate::server::{AppState, middlewares::gh_event::GitHubEventContext};
use axum::response::{IntoResponse, Response, Result};
use octocrab::{
    models::webhook_events::*,
    params::checks::{CheckRunConclusion, CheckRunStatus},
};
use tracing::info;

pub async fn handle_pull_request(ctx: &GitHubEventContext, state: &AppState) -> Result<Response> {
    let event = &ctx.event;
    let mng = &state.freeze_manager;
    let gh = &state.gh;

    let installation_id = ctx.installation_id.ok_or("Installation ID not found")?;
    let repository = event.repository.as_ref().ok_or("Repository not found")?;
    let repo = &repository.name;
    let owner = &repository.owner.as_ref().ok_or("Owner not found")?.login;

    let mut conclusion = CheckRunConclusion::Failure; // default as if it's frozen

    if !mng.is_frozen(repo, installation_id).await.map_err(|e| {
        tracing::error!("Failed to check if repository {} is frozen: {:?}", repo, e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })? {
        info!("Repository {} is not frozen", repo);
        conclusion = CheckRunConclusion::Success; // if not frozen, set to success
    }

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
        conclusion,
        installation_id as u64,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to create check run: {:?}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(axum::http::StatusCode::OK.into_response())
}
