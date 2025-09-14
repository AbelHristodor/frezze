use axum::response::{IntoResponse, Response, Result};
use octocrab::models::webhook_events::WebhookEventPayload;
use tracing::{info, error};

use crate::{
    freezer::{self, commands::{Command, CommandParser}},
    server::{middlewares::gh_event::GitHubEventContext, AppState},
};

pub async fn handle_issue_comment(ctx: &GitHubEventContext, state: &AppState) -> Result<Response> {
    let event = &ctx.event;
    info!("Received issue comment event: {:?}", ctx.event.kind);

    let mng = &state.freeze_manager;
    let installation_id = ctx.installation_id.ok_or("Installation ID not found")?;
    let repository = event.repository.as_ref().ok_or("Repository not found")?;
    let repo_name = &repository.name;
    let owner = &repository.owner.as_ref().ok_or("Repository owner not found")?.login;

    let WebhookEventPayload::IssueComment(comment) = &event.specific else {
        error!("Expected IssueComment event, got: {:?}", event.kind);
        return Err(axum::http::StatusCode::BAD_REQUEST)?;
    };

    let author = comment.comment.user.login.clone();
    let issue_number = comment.issue.number;

    if let Some(body) = comment.comment.body.clone() {
        let parser = CommandParser::new();
        
        let response_message = match parser.parse(&body) {
            Ok(cmd) => {
                match cmd {
                    Command::Freeze { duration, reason } => {
                        match mng.freeze(installation_id, repo_name, duration, reason.clone(), author.clone()).await {
                            Ok(r) => {
                                let duration = match r.expires_at {
                                    Some(d) => d - r.started_at,
                                    None => freezer::manager::DEFAULT_FREEZE_DURATION,
                                };

                                let duration_str = format!(" for {}", format_duration(duration));
                                let reason_str = reason.map(|r| format!(" ({})", r)).unwrap_or_default();
                                format!("✅ Repository `{}` has been frozen{}{}", repo_name, duration_str, reason_str)
                            }
                            Err(e) => {
                                error!("Failed to freeze repository: {:?}", e);
                                format!("❌ Failed to freeze repository: {}", e)
                            }
                        }
                    },
                    Command::Unfreeze { reason: _ } => {
                        match mng.unfreeze(installation_id, repo_name, author.clone()).await {
                            Ok(_) => {
                                format!("✅ Repository `{}` has been unfrozen", repo_name)
                            }
                            Err(e) => {
                                error!("Failed to unfreeze repository: {:?}", e);
                                format!("❌ Failed to unfreeze repository: {}", e)
                            }
                        }
                    },
                    _ => "⚠️ Command not yet implemented".to_string(),
                }
            }
            Err(e) => {
                info!("Not a valid command: {}", e);
                return Ok(axum::http::StatusCode::OK.into_response());
            }
        };

        // Create response comment
        if let Err(e) = state.gh.create_comment(
            installation_id as u64,
            owner,
            repo_name,
            issue_number,
            &response_message,
        ).await {
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
