use crate::{
    repository::Repository,
    server::{AppState, middlewares::gh_event::GitHubEventContext},
};
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
    let repository_event = event.repository.as_ref().ok_or("Repository not found")?;
    let repository = Repository::new(&repository_event.owner.as_ref().ok_or("Owner not found")?.login, &repository_event.name);

    let mut conclusion = CheckRunConclusion::Failure; // default as if it's frozen

    if !mng.is_frozen(&repository, installation_id).await.map_err(|e| {
        tracing::error!("Failed to check if repository {} is frozen: {:?}", repository, e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })? {
        info!("Repository {} is not frozen", repository);
        conclusion = CheckRunConclusion::Success; // if not frozen, set to success
    }

    let WebhookEventPayload::PullRequest(pr_event) = &event.specific else {
        tracing::error!("Expected PullRequest event, got: {:?}", event.kind);
        return Err(axum::http::StatusCode::BAD_REQUEST)?;
    };

    info!("Received PR event for repository: {}", repository);

    let head_sha = &pr_event.pull_request.head.sha;

    gh.create_check_run(
        repository.owner(),
        repository.name(),
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
