use axum::response::{Response, Result};
use tracing::warn;

use crate::{
    freezer::commands::{Command, CommandParser},
    repository,
    server::{self, middlewares::gh_event},
};

use super::helpers;

pub async fn handle_issue_comment(
    ctx: &gh_event::GitHubEventContext,
    state: &server::AppState,
) -> Result<Response> {
    helpers::log_event_received("issue comment", &format!("{:?}", ctx.event.kind));

    let (repository, installation_id) = helpers::extract_repository_info(ctx)?;
    let (author, issue_number, body) = helpers::extract_issue_comment_info(ctx)?;

    if let Some(body) = body {
        let parser = CommandParser::new();

        match parser.parse(&body) {
            Ok(cmd) => {
                handle_freeze_command(
                    cmd,
                    state,
                    installation_id,
                    &repository,
                    &author,
                    issue_number,
                )
                .await
            }
            Err(e) => {
                warn!("Not a valid command: {}", e);
                return helpers::success_response();
            }
        };
    }

    helpers::success_response()
}

async fn handle_freeze_command(
    cmd: Command,
    state: &server::AppState,
    installation_id: i64,
    repository: &repository::Repository,
    author: &str,
    issue_number: u64,
) -> () {
    let mng = &state.freeze_manager;

    match cmd {
        Command::Freeze { duration, reason } => {
            let _ = mng
                .freeze(
                    installation_id,
                    repository,
                    duration,
                    reason.clone(),
                    author.to_string(),
                    issue_number,
                )
                .await;
        }
        Command::Unfreeze { reason: _ } => {
            let _ = mng
                .unfreeze(
                    installation_id,
                    repository,
                    author.to_string(),
                    issue_number,
                )
                .await;
        }
        _ => todo!("Not implemented yet!"),
    }
}
