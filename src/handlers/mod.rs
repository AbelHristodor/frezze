use octocrab::models::webhook_events::WebhookEventPayload;
use tracing::info;

use crate::freezer::commands;

#[derive(Clone)]
pub struct Hello {}

pub async fn issue_comment_handler(context: octofer::Context, extra: Hello) -> anyhow::Result<()> {
    info!("Issue comment event received!");
    info!("Event type: {}", context.kind());
    info!("Installation ID: {:?}", context.installation_id());

    let client = match context.github_client {
        Some(c) => c,
        None => panic!(),
    };

    if let Some(e) = context.event {
        let WebhookEventPayload::IssueComment(comment) = &e.specific else {
            panic!();
        };

        let author = comment.comment.user.login.clone();
        let issue_number = comment.issue.number;

        if let Some(body) = comment.comment.body.clone() {
            // Parse just the first line
            let parser = commands::parse(body.lines().next().unwrap_or(&body))?;

            match parser.command {
                commands::Command::Freeze(freeze_args) => todo!(),
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
