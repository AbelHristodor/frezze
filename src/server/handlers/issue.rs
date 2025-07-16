use axum::response::{IntoResponse, Response, Result};
use octocrab::models::webhook_events::WebhookEventPayload;
use tracing::info;

use crate::server::{AppState, middlewares::gh_event::GitHubEventContext};

pub async fn handle_issue_comment(ctx: &GitHubEventContext, state: &AppState) -> Result<Response> {
    let event = &ctx.event;
    info!("Received issue comment event: {:?}", ctx.event.kind);

    let WebhookEventPayload::IssueComment(comment) = &event.specific else {
        tracing::error!("Expected PullRequest event, got: {:?}", event.kind);
        return Err(axum::http::StatusCode::BAD_REQUEST)?;
    };

    info!("Issue comment: {:?}", comment.comment);

    Ok(axum::http::StatusCode::OK.into_response())
}
