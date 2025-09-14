use axum::response::{Response, Result};
use tracing::info;

use crate::freezer::{
    self,
    commands::{Command, CommandParser},
};

use super::{helpers, messages};

pub async fn handle_issue_comment(
    ctx: &crate::server::middlewares::gh_event::GitHubEventContext,
    state: &crate::server::AppState,
) -> Result<Response> {
    helpers::log_event_received("issue comment", &format!("{:?}", ctx.event.kind));

    let (repository, installation_id) = helpers::extract_repository_info(ctx)?;
    let (author, issue_number, body) = helpers::extract_issue_comment_info(ctx)?;

    if let Some(body) = body {
        let parser = CommandParser::new();

        let response_message = match parser.parse(&body) {
            Ok(cmd) => {
                handle_freeze_command(cmd, state, installation_id, &repository, &author).await
            }
            Err(e) => {
                info!("Not a valid command: {}", e);
                return helpers::success_response();
            }
        };

        helpers::send_response_comment(
            state,
            installation_id,
            &repository,
            issue_number,
            &response_message,
        )
        .await?;
    }

    helpers::success_response()
}

async fn handle_freeze_command(
    cmd: Command,
    state: &crate::server::AppState,
    installation_id: i64,
    repository: &crate::repository::Repository,
    author: &str,
) -> String {
    let mng = &state.freeze_manager;

    match cmd {
        Command::Freeze { duration, reason } => {
            match mng
                .freeze(
                    installation_id,
                    repository,
                    duration,
                    reason.clone(),
                    author.to_string(),
                )
                .await
            {
                Ok(r) => {
                    let duration = match r.expires_at {
                        Some(d) => d - r.started_at,
                        None => freezer::manager::DEFAULT_FREEZE_DURATION,
                    };

                    let duration_str = messages::format_duration_display(duration);
                    let reason_str = messages::format_reason_display(reason);
                    messages::freeze_success(&repository.to_string(), &duration_str, &reason_str)
                }
                Err(e) => {
                    tracing::error!("Failed to freeze repository: {:?}", e);
                    messages::freeze_error(&e.to_string())
                }
            }
        }
        Command::Unfreeze { reason: _ } => {
            match mng
                .unfreeze(installation_id, repository, author.to_string())
                .await
            {
                Ok(_) => messages::unfreeze_success(&repository.to_string()),
                Err(e) => {
                    tracing::error!("Failed to unfreeze repository: {:?}", e);
                    messages::unfreeze_error(&e.to_string())
                }
            }
        }
        _ => messages::command_not_implemented(),
    }
}
