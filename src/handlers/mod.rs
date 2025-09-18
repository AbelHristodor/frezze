use std::sync::Arc;

use octocrab::models::webhook_events::WebhookEventPayload;
use tracing::info;

use crate::{
    AppState,
    freezer::{self, commands},
};

pub async fn issue_comment_handler(
    context: octofer::Context,
    extra: Arc<AppState>,
) -> anyhow::Result<()> {
    info!("Issue comment event received!");
    info!("Event type: {}", context.kind());
    info!("Installation ID: {:?}", context.installation_id());

    let client = match context.github_client {
        Some(c) => c,
        None => panic!(),
    };
    let install_id = context
        .installation_id
        .ok_or(anyhow::anyhow!("Cannot get installation_id"))?;

    let mng = freezer::manager::FreezeManager::new(extra.database.clone(), client);

    if let Some(e) = context.event {
        let WebhookEventPayload::IssueComment(comment) = &e.specific else {
            panic!();
        };

        let author = comment.comment.user.login.clone();
        let issue_number = comment.issue.number;
        let repo = e
            .repository
            .ok_or(anyhow::anyhow!("Cannot get repository from eveent"))?;

        if let Some(body) = comment.comment.body.clone() {
            // Parse just the first line
            let parser = commands::parse(body.lines().next().unwrap_or(&body))?;

            match parser.command {
                commands::Command::Freeze(freeze_args) => {
                    mng.freeze(
                        install_id,
                        &repo.into(),
                        freeze_args.duration,
                        freeze_args.reason,
                        author,
                        issue_number,
                    )
                    .await;
                }
                commands::Command::FreezeAll(freeze_args) => todo!(),
                commands::Command::Unfreeze(unfreeze_args) => todo!(),
                commands::Command::UnfreezeAll(unfreeze_args) => todo!(),
                commands::Command::Status(status_args) => todo!(),
                commands::Command::ScheduleFreeze(schedule_freeze_args) => todo!(),
            }
        }
    } else {
        todo!()
    }

    Ok(())
}
