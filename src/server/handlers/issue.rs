use axum::response::{IntoResponse, Response, Result};
use octocrab::models::webhook_events::WebhookEventPayload;
use tracing::info;

use crate::{
    freezer::commands::{Command, CommandParser},
    server::{AppState, middlewares::gh_event::GitHubEventContext},
};

pub async fn handle_issue_comment(ctx: &GitHubEventContext, state: &AppState) -> Result<Response> {
    let event = &ctx.event;
    info!("Received issue comment event: {:?}", ctx.event.kind);

    let mng = &state.freeze_manager;
    let installation_id = ctx.installation_id.ok_or("Installation ID not found")?;
    let repository = event.repository.as_ref().ok_or("Repository not found")?;
    let repo = &repository.name;

    let WebhookEventPayload::IssueComment(comment) = &event.specific else {
        tracing::error!("Expected IssueComment event, got: {:?}", event.kind);
        return Err(axum::http::StatusCode::BAD_REQUEST)?;
    };

    info!("Issue comment: {:?}", comment.comment);

    let author = comment.comment.user.login.clone();

    if let Some(body) = comment.comment.body.clone() {
        let parser = CommandParser::new();
        let cmd = parser.parse(&body).unwrap();

        match cmd {
            Command::Freeze { duration, reason } => {
                mng.freeze(installation_id, repo, duration, reason, author)
                    .await
                    .map_err(|e| {
                        tracing::error!("Failed to freeze repository: {:?}", e);
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR
                    })?;
            }
            _ => todo!(),
        }
    }

    Ok(axum::http::StatusCode::OK.into_response())
}
