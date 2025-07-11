use crate::server::AppState;
use anyhow::anyhow;
use axum::{
    body::Bytes,
    extract::{Request, State},
    http::HeaderMap,
    response::{IntoResponse, Response, Result},
};
use octocrab::models::webhook_events::*;
use tracing::info;

const GH_EVENT_HEADER: &str = "X-GitHub-Event";

pub async fn get_rulesets(State(state): State<AppState>) -> impl IntoResponse {
    let installations = state.gh.get_installations().await.unwrap();
    let repos = state
        .gh
        .get_installation_repositories(installations[0].id.0)
        .await
        .unwrap();
    info!(
        "Found {} repositories for installation {}",
        repos.len(),
        installations[0].id.0
    );
}

pub async fn webhook(headers: HeaderMap, body: Bytes) -> Result<Response> {
    info!("Received webhook event");
    let event_header = headers
        .get(GH_EVENT_HEADER)
        .ok_or(anyhow!("Missing required header: {}", GH_EVENT_HEADER))
        .map_err(|e| {
            tracing::error!("Missing header {}: {}", GH_EVENT_HEADER, e);
            axum::http::StatusCode::BAD_REQUEST
        })?
        .to_str()
        .map_err(|e| {
            tracing::error!("Invalid header value for {}: {}", GH_EVENT_HEADER, e);
            axum::http::StatusCode::BAD_REQUEST
        })?;

    let event = WebhookEvent::try_from_header_and_body(event_header, &body).map_err(|e| {
        tracing::error!("Failed to parse webhook event: {}", e);
        axum::http::StatusCode::BAD_REQUEST
    })?;
    match event.kind {
        WebhookEventType::BranchProtectionRule => todo!(),
        WebhookEventType::CheckRun => todo!(),
        WebhookEventType::CheckSuite => todo!(),
        WebhookEventType::CommitComment => todo!(),
        WebhookEventType::Create => todo!(),
        WebhookEventType::Delete => todo!(),
        WebhookEventType::Installation => todo!(),
        WebhookEventType::InstallationRepositories => todo!(),
        WebhookEventType::InstallationTarget => todo!(),
        WebhookEventType::IssueComment => todo!(),
        WebhookEventType::Ping => todo!(),
        WebhookEventType::PullRequest => todo!(),
        WebhookEventType::PullRequestReview => todo!(),
        WebhookEventType::PullRequestReviewComment => todo!(),
        WebhookEventType::PullRequestReviewThread => todo!(),
        WebhookEventType::Push => todo!(),
        WebhookEventType::Unknown(_) => todo!(),
        _ => todo!(),
    }
}
