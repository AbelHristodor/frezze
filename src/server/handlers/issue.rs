use axum::response::{IntoResponse, Response, Result};
use octocrab::models::webhook_events::WebhookEventPayload;
use tracing::{error, info};

use crate::{
    freezer::{
        self,
        commands::{Command, CommandParser},
    },
    repository::Repository,
    server::{AppState, middlewares::gh_event::GitHubEventContext},
};

pub async fn handle_issue_comment(ctx: &GitHubEventContext, state: &AppState) -> Result<Response> {
    let event = &ctx.event;
    info!("Received issue comment event: {:?}", ctx.event.kind);

    let mng = &state.freeze_manager;
    let installation_id = ctx.installation_id.ok_or("Installation ID not found")?;
    let repository_event = event.repository.as_ref().ok_or("Repository not found")?;
    let repository = Repository::new(&repository_event.owner.as_ref().ok_or("Repository owner not found")?.login, &repository_event.name);

    let WebhookEventPayload::IssueComment(comment) = &event.specific else {
        error!("Expected IssueComment event, got: {:?}", event.kind);
        return Err(axum::http::StatusCode::BAD_REQUEST)?;
    };

    let author = comment.comment.user.login.clone();
    let issue_number = comment.issue.number;

    if let Some(body) = comment.comment.body.clone() {
        let parser = CommandParser::new();

        let response_message = match parser.parse(&body) {
            Ok(cmd) => match cmd {
                Command::Freeze { duration, reason } => {
                    match mng
                        .freeze(
                            installation_id,
                            &repository,
                            duration,
                            reason.clone(),
                            author.clone(),
                        )
                        .await
                    {
                        Ok(r) => {
                            let duration = match r.expires_at {
                                Some(d) => d - r.started_at,
                                None => freezer::manager::DEFAULT_FREEZE_DURATION,
                            };

                            let duration_str = format!(" for {}", format_duration(duration));
                            let reason_str =
                                reason.map(|r| format!(" ({})", r)).unwrap_or_default();
                            format!(
                                "✅ Repository `{}` has been frozen{}{}",
                                repository, duration_str, reason_str
                            )
                        }
                        Err(e) => {
                            error!("Failed to freeze repository: {:?}", e);
                            format!("❌ Failed to freeze repository: {}", e)
                        }
                    }
                }
                Command::Unfreeze { reason: _ } => {
                    match mng
                        .unfreeze(installation_id, &repository, author.clone())
                        .await
                    {
                        Ok(_) => {
                            format!("✅ Repository `{}` has been unfrozen", repository)
                        }
                        Err(e) => {
                            error!("Failed to unfreeze repository: {:?}", e);
                            format!("❌ Failed to unfreeze repository: {}", e)
                        }
                    }
                }
                _ => "⚠️ Command not yet implemented".to_string(),
            },
            Err(e) => {
                info!("Not a valid command: {}", e);
                return Ok(axum::http::StatusCode::OK.into_response());
            }
        };

        // Create response comment
        if let Err(e) = mng
            .notify_comment_issue(
                installation_id,
                &repository,
                issue_number,
                &response_message,
            )
            .await
        {
            error!("Failed to create response comment: {}", e);
        }
    }

    Ok(axum::http::StatusCode::OK.into_response())
}

fn format_duration(duration: chrono::Duration) -> String {
    let total_seconds = duration.num_seconds();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;

    if hours > 0 {
        format!("{}h{}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}
