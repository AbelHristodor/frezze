use axum::response::{Response, Result};
use octocrab::params::checks::{CheckRunConclusion, CheckRunStatus};
use tracing::info;

use super::helpers;

pub async fn handle_pull_request(
    ctx: &crate::server::middlewares::gh_event::GitHubEventContext,
    state: &crate::server::AppState,
) -> Result<Response> {
    let repo_name = ctx
        .event
        .repository
        .as_ref()
        .map(|r| r.name.as_str())
        .unwrap_or("unknown");
    helpers::log_event_received("pull request", &format!("repository: {}", repo_name));

    let (repository, installation_id) = helpers::extract_repository_info(ctx)?;
    let head_sha = helpers::extract_pull_request_info(ctx)?;

    let mng = &state.freeze_manager;
    let gh = &state.gh;

    let mut conclusion = CheckRunConclusion::Failure; // default as if it's frozen

    if !mng
        .is_frozen(&repository, installation_id)
        .await
        .map_err(|e| {
            tracing::error!(
                "Failed to check if repository {} is frozen: {:?}",
                repository,
                e
            );
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?
    {
        info!("Repository {} is not frozen", repository);
        conclusion = CheckRunConclusion::Success; // if not frozen, set to success
    }

    gh.create_check_run(
        repository.owner(),
        repository.name(),
        &head_sha,
        CheckRunStatus::Completed,
        conclusion,
        installation_id as u64,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to create check run: {:?}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    helpers::success_response()
}
