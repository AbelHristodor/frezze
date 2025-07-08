use anyhow::{Result, anyhow};
use chrono::{NaiveDateTime, TimeZone, Utc};
use octocrab::{
    Octocrab,
    models::{InstallationRepositories, InstallationToken},
    params::apps::CreateInstallationAccessToken,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tracing::info;
use url::Url;

mod branch_protection;

fn parse_to_utc(date_str: &str) -> chrono::DateTime<chrono::Utc> {
    // Parse into NaiveDateTime (no timezone)
    let naive =
        NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S").expect("Failed to parse date");

    // Convert NaiveDateTime to DateTime<Utc>

    Utc.from_utc_datetime(&naive)
}

struct CachedInstallationClient {
    client: octocrab::Octocrab,
    token: InstallationToken,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl CachedInstallationClient {
    fn is_expired(&self) -> bool {
        let default_expires_at = self.created_at + chrono::Duration::hours(1);

        let buffer = chrono::Duration::minutes(5);
        let expires_at = self
            .token
            .expires_at
            .clone()
            .unwrap_or(default_expires_at.to_string());

        Utc::now() + buffer >= parse_to_utc(&expires_at)
    }
}

pub struct Github {
    client: octocrab::Octocrab,
    app_id: u64,
    installation_clients: Arc<RwLock<HashMap<u64, CachedInstallationClient>>>,
}

impl Github {
    pub async fn new(app_id: u64, key: &[u8]) -> Self {
        let client = octocrab::OctocrabBuilder::new()
            .add_retry_config(octocrab::service::middleware::retry::RetryConfig::Simple(
                20,
            ))
            .app(
                app_id.into(),
                jsonwebtoken::EncodingKey::from_rsa_pem(key)
                    .expect("Failed to create encoding key from PEM"),
            )
            .build()
            .unwrap();

        Github {
            client,
            app_id,
            installation_clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_installations(&self) -> Result<Vec<octocrab::models::Installation>> {
        let installations = self
            .client
            .apps()
            .installations()
            .send()
            .await
            .map_err(|e| anyhow!("Failed to fetch installations: {}", e))?
            .take_items();
        info!("Fetched {} installations", installations.len());

        Ok(installations)
    }

    pub async fn get_installation_token(&self, installation_id: u64) -> Result<Octocrab> {
        // Check if we have a cached client that's still valid
        {
            let clients = self.installation_clients.read().await;
            if let Some(cached) = clients.get(&installation_id) {
                if !cached.is_expired() {
                    return Ok(cached.client.clone());
                }
            }
        }

        // Create a new installation client
        let client = self.create_installation_client(installation_id).await?;
        Ok(client)
    }

    async fn create_installation_client(&self, installation_id: u64) -> Result<Octocrab> {
        info!(
            "Creating new installation client for ID: {}",
            installation_id
        );
        let token = self
            .create_installation_token(installation_id, None)
            .await?;

        let client = Octocrab::builder()
            .add_retry_config(octocrab::service::middleware::retry::RetryConfig::Simple(
                20,
            ))
            .personal_token(token.token.clone())
            .build()
            .map_err(|e| anyhow!("Failed to create installation client: {}", e))?;

        // Cache the client
        let cached_client = CachedInstallationClient {
            client: client.clone(),
            token,
            created_at: Utc::now(),
        };

        {
            let mut clients = self.installation_clients.write().await;
            clients.insert(installation_id, cached_client);
        }

        Ok(client)
    }

    /// Get a client authenticated as a specific installation
    pub async fn installation_client(&self, installation_id: u64) -> Result<Octocrab> {
        // Check if we have a cached client that's still valid
        {
            let clients = self.installation_clients.read().await;
            if let Some(cached) = clients.get(&installation_id) {
                if !cached.is_expired() {
                    return Ok(cached.client.clone());
                }
            }
        }

        // Create a new installation client
        let client = self.create_installation_client(installation_id).await?;
        Ok(client)
    }

    /// Create a new installation access token for the given installation ID
    async fn create_installation_token(
        &self,
        installation_id: u64,
        repositories: Option<Vec<String>>,
    ) -> Result<InstallationToken> {
        let installations = self.get_installations().await?;

        let installation = installations
            .iter()
            .find(|i| i.id.0 == installation_id)
            .ok_or_else(|| anyhow!("Installation with ID {} not found", installation_id))?;

        let access_tokens_url = installation
            .access_tokens_url
            .as_ref()
            .ok_or_else(|| anyhow!("No access tokens URL for installation {}", installation_id))?;

        let mut create_token_request = CreateInstallationAccessToken::default();
        if let Some(repos) = repositories {
            create_token_request.repositories = repos;
        }

        let url = Url::parse(access_tokens_url)
            .map_err(|e| anyhow!("Invalid access tokens URL: {}", e))?;

        let token: InstallationToken = self
            .client
            .post(url.path(), Some(&create_token_request))
            .await
            .map_err(|e| anyhow!("Failed to create installation token: {}", e))?;

        info!(
            "Created installation token for installation {}",
            installation_id
        );
        Ok(token)
    }

    /// Get repositories accessible by an installation
    pub async fn get_installation_repositories(
        &self,
        installation_id: u64,
    ) -> Result<Vec<octocrab::models::Repository>> {
        let client = self.installation_client(installation_id).await?;

        let installation_repos: InstallationRepositories = client
            .get("/installation/repositories", None::<&()>)
            .await
            .map_err(|e| anyhow!("Failed to get installation repositories: {}", e))?;

        info!(
            "Installation {} has access to {} repositories",
            installation_id,
            installation_repos.repositories.len()
        );

        Ok(installation_repos.repositories)
    }
    /// Execute a closure with an installation client
    pub async fn with_installation<F, R>(&self, installation_id: u64, f: F) -> Result<R>
    where
        F: FnOnce(Octocrab) -> R,
    {
        let client = self.installation_client(installation_id).await?;
        Ok(f(client))
    }

    /// Execute an async closure with an installation client
    pub async fn with_installation_async<F, Fut, R>(&self, installation_id: u64, f: F) -> Result<R>
    where
        F: FnOnce(Octocrab) -> Fut,
        Fut: std::future::Future<Output = Result<R>>,
    {
        let client = self.installation_client(installation_id).await?;
        f(client).await
    }

    /// Clear cached installation client (useful for testing or forcing refresh)
    pub async fn clear_installation_cache(&self, installation_id: Option<u64>) {
        let mut clients = self.installation_clients.write().await;

        if let Some(id) = installation_id {
            clients.remove(&id);
            info!("Cleared cache for installation {}", id);
        } else {
            clients.clear();
            info!("Cleared all installation caches");
        }
    }

    /// Get the app client (for app-level operations)
    pub fn client(&self) -> &Octocrab {
        &self.client
    }
}

impl Github {
    /// Get a specific repository through an installation
    pub async fn get_repository(
        &self,
        installation_id: u64,
        owner: &str,
        repo: &str,
    ) -> Result<octocrab::models::Repository> {
        self.with_installation_async(installation_id, |client| async move {
            client
                .repos(owner, repo)
                .get()
                .await
                .map_err(|e| anyhow!("Failed to get repository {}/{}: {}", owner, repo, e))
        })
        .await
    }

    /// Create a comment on an issue or PR
    pub async fn create_comment(
        &self,
        installation_id: u64,
        owner: &str,
        repo: &str,
        issue_number: u64,
        body: &str,
    ) -> Result<octocrab::models::issues::Comment> {
        self.with_installation_async(installation_id, |client| async move {
            client
                .issues(owner, repo)
                .create_comment(issue_number, body)
                .await
                .map_err(|e| anyhow!("Failed to create comment: {}", e))
        })
        .await
    }
}
