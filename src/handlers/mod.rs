use tracing::info;

pub async fn issue_comment_handler(context: octofer::Context) -> anyhow::Result<()> {
    info!("Issue comment event received!");
    info!("Event type: {}", context.event_type());
    info!("Installation ID: {:?}", context.installation_id());

    let client = match context.github_client {
        Some(c) => c,
        None => panic!(),
    };

    Ok(())
}
