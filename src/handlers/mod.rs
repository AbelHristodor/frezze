use std::sync::Arc;

use octocrab::models::webhook_events::WebhookEventPayload;
use tracing::{error, info};

use crate::{
    AppState,
    freezer::{self, commands, errors::ParsingError, messages},
    permissions::{PermissionResult, PermissionService},
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
    let installation_id = context
        .installation_id
        .ok_or(anyhow::anyhow!("Cannot get installation_id"))?;

    let mng = freezer::manager::FreezeManager::new(extra.database.clone(), client);

    if let Some(e) = context.event {
        let WebhookEventPayload::IssueComment(comment) = &e.specific else {
            panic!();
        };

        let author = comment.comment.user.login.clone();
        let issue_nr = comment.issue.number;
        let repo = e
            .repository
            .ok_or(anyhow::anyhow!("Cannot get repository from event"))?;

        if let Some(body) = comment.comment.body.clone() {
            // Parse just the first line
            let parser = match commands::parse(body.lines().next().unwrap_or(&body)) {
                Ok(p) => p,
                Err(e) => {
                    if let ParsingError::NotACommand = e {
                        info!("Not a command... skipping");
                        return Ok(());
                    } else {
                        error!("Error parsing command: {e}");
                        return Err(e.into());
                    }
                }
            };

            // Check permissions before executing command
            if let Some(ref user_config) = extra.user_config {
                let permission_service = PermissionService::new(user_config.clone());
                let repository: crate::repository::Repository = repo.clone().into();
                let repo_name = repository.full_name();

                match permission_service
                    .check_permission(installation_id as i64, &repo_name, &author, &parser.command)
                    .await
                {
                    Ok(PermissionResult::Allowed) => {
                        // Permission granted, proceed with command execution
                    }
                    Ok(PermissionResult::Denied(reason)) => {
                        let error_msg = messages::permission_denied(&author, &reason);
                        mng.notify_comment_issue(
                            installation_id,
                            &repo.into(),
                            issue_nr,
                            &error_msg,
                        )
                        .await;
                        return Ok(());
                    }
                    Err(e) => {
                        error!("Error checking permissions for user {}: {}", author, e);
                        let error_msg = messages::permission_check_failed(&author, &e.to_string());
                        mng.notify_comment_issue(
                            installation_id,
                            &repo.into(),
                            issue_nr,
                            &error_msg,
                        )
                        .await;
                        return Ok(());
                    }
                }
            } else {
                // No user config provided - deny all commands except status
                match parser.command {
                    commands::Command::Status(_) => {
                        // Status is always allowed when no config is provided
                    }
                    _ => {
                        let error_msg = messages::permission_denied(
                            &author,
                            "No permission configuration file loaded. Contact your administrator.",
                        );
                        mng.notify_comment_issue(
                            installation_id,
                            &repo.into(),
                            issue_nr,
                            &error_msg,
                        )
                        .await;
                        return Ok(());
                    }
                }
            }

            match parser.command {
                commands::Command::Freeze(freeze_args) => {
                    mng.freeze(
                        installation_id,
                        &repo.into(),
                        freeze_args.duration,
                        freeze_args.reason,
                        author,
                        issue_nr,
                        freeze_args.repos,
                    )
                    .await;
                }
                commands::Command::FreezeAll(freeze_args) => {
                    mng.freeze_all(
                        installation_id,
                        freeze_args.duration,
                        freeze_args.reason,
                        author,
                        issue_nr,
                        freeze_args.repos,
                    )
                    .await;
                }
                commands::Command::Unfreeze(unfreeze_args) => {
                    mng.unfreeze(
                        installation_id,
                        &repo.into(),
                        author,
                        unfreeze_args.reason,
                        issue_nr,
                    )
                    .await;
                }
                commands::Command::UnfreezeAll => {
                    mng.unfreeze_all(installation_id, author, issue_nr).await;
                }
                commands::Command::Status(status_args) => {
                    mng.get_status(installation_id, status_args.repos, issue_nr, &repo.into())
                        .await;
                }
                commands::Command::ScheduleFreeze(schedule_freeze_args) => {
                    let repository = repo.clone().into();
                    let reason_for_display = schedule_freeze_args.reason.clone();
                    match mng
                        .schedule_freeze(
                            installation_id,
                            &repository,
                            schedule_freeze_args.from,
                            schedule_freeze_args.to,
                            schedule_freeze_args.duration,
                            schedule_freeze_args.reason,
                            author.clone(),
                        )
                        .await
                    {
                        Ok(_) => {
                            let start_str =
                                schedule_freeze_args.from.format("%Y-%m-%d %H:%M:%S UTC");
                            let end_str = schedule_freeze_args
                                .to
                                .or_else(|| {
                                    schedule_freeze_args
                                        .duration
                                        .map(|d| schedule_freeze_args.from + d)
                                })
                                .unwrap_or_else(|| {
                                    schedule_freeze_args.from
                                        + crate::freezer::manager::DEFAULT_FREEZE_DURATION
                                })
                                .format("%Y-%m-%d %H:%M:%S UTC");

                            let success_msg = format!(
                                "## ⏰ Freeze Scheduled\n\n\
                                📅 **Repository `{}` freeze has been scheduled**\n\n\
                                **Start**: {}\n\
                                **End**: {}\n\
                                **Reason**: {}\n\n\
                                > The freeze will automatically activate at the scheduled time.",
                                repository.full_name(),
                                start_str,
                                end_str,
                                reason_for_display
                                    .unwrap_or_else(|| "No reason provided".to_string())
                            );
                            mng.notify_comment_issue(
                                installation_id,
                                &repository,
                                issue_nr,
                                &success_msg,
                            )
                            .await;
                        }
                        Err(e) => {
                            let error_msg = format!(
                                "## ❌ Schedule Failed\n\n\
                                🚫 **Failed to schedule freeze**\n\n\
                                ```\n{}\n```\n\n\
                                *Please check your parameters and try again.*",
                                e
                            );
                            mng.notify_comment_issue(
                                installation_id,
                                &repository,
                                issue_nr,
                                &error_msg,
                            )
                            .await;
                        }
                    }
                }
                commands::Command::UnlockPr(unlock_pr_args) => {
                    let pr_number = if let Some(pr) = unlock_pr_args.pr_number {
                        pr
                    } else {
                        issue_nr
                    };

                    let repository = repo.clone();
                    mng.unlock_pr(
                        installation_id,
                        &repository.into(),
                        pr_number,
                        author,
                        unlock_pr_args.reason,
                        issue_nr,
                    )
                    .await;
                }
            }
        }
    } else {
        todo!()
    }

    Ok(())
}
