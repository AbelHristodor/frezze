/// Helper functions for common handler operations to implement DRY principle.
use axum::response::{IntoResponse, Response, Result};
use octocrab::models::webhook_events::WebhookEventPayload;
use tracing::{error, info};

use crate::{
    repository::Repository,
    server::{AppState, middlewares::gh_event::GitHubEventContext},
};

/// Extract repository information from GitHub event context
pub fn extract_repository_info(ctx: &GitHubEventContext) -> Result<(Repository, i64)> {
    let event = &ctx.event;
    let installation_id = ctx.installation_id.ok_or("Installation ID not found")?;
    let repository_event = event.repository.as_ref().ok_or("Repository not found")?;
    let repository = Repository::new(
        &repository_event
            .owner
            .as_ref()
            .ok_or("Repository owner not found")?
            .login,
        &repository_event.name,
    );

    Ok((repository, installation_id))
}

/// Extract issue comment information from event payload
pub fn extract_issue_comment_info(
    ctx: &GitHubEventContext,
) -> Result<(String, u64, Option<String>)> {
    let WebhookEventPayload::IssueComment(comment) = &ctx.event.specific else {
        error!("Expected IssueComment event, got: {:?}", ctx.event.kind);
        return Err(axum::http::StatusCode::BAD_REQUEST)?;
    };

    let author = comment.comment.user.login.clone();
    let issue_number = comment.issue.number;
    let body = comment.comment.body.clone();

    Ok((author, issue_number, body))
}

/// Extract pull request information from event payload  
pub fn extract_pull_request_info(ctx: &GitHubEventContext) -> Result<String> {
    let WebhookEventPayload::PullRequest(pr_event) = &ctx.event.specific else {
        error!("Expected PullRequest event, got: {:?}", ctx.event.kind);
        return Err(axum::http::StatusCode::BAD_REQUEST)?;
    };

    Ok(pr_event.pull_request.head.sha.clone())
}

/// Send a response comment and handle errors consistently
pub async fn send_response_comment(
    state: &AppState,
    installation_id: i64,
    repository: &Repository,
    issue_number: u64,
    message: &str,
) -> Result<()> {
    if let Err(e) = state
        .freeze_manager
        .notify_comment_issue(installation_id, repository, issue_number, message)
        .await
    {
        error!("Failed to create response comment: {}", e);
    }
    Ok(())
}

/// Standard success response for webhook handlers
pub fn success_response() -> Result<Response> {
    Ok(axum::http::StatusCode::OK.into_response())
}

/// Log event reception with consistent format
pub fn log_event_received(event_type: &str, context: &str) {
    info!("Received {} event: {}", event_type, context);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_response() {
        let response = success_response();
        assert!(response.is_ok());
    }

    #[test]
    fn test_log_event_received() {
        // This test just ensures the function doesn't panic
        log_event_received("test", "test context");
    }
}
