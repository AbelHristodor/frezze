use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use octocrab::{
    Octocrab,
    models::{InstallationRepositories, InstallationToken},
    params::apps::CreateInstallationAccessToken,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tracing::{debug, info};
use url::Url;

mod repo;
mod types;

fn parse_to_utc(date_str: &str) -> chrono::DateTime<chrono::Utc> {
    date_str
        .parse::<chrono::DateTime<chrono::Utc>>()
        .context("Failed to parse date string to DateTime<Utc>")
        .expect("Invalid date format")
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

        debug!("Expires_at: {:?}", expires_at);
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
    pub async fn with_installation<F, R>(&self, installation_id: u64, f: F) -> anyhow::Result<R>
    where
        F: FnOnce(Octocrab) -> anyhow::Result<R>,
    {
        let client = self.installation_client(installation_id).await?;
        f(client)
    }

    /// Execute an async closure with an installation client
    pub async fn with_installation_async<F, Fut, R>(
        &self,
        installation_id: u64,
        f: F,
    ) -> anyhow::Result<R>
    where
        F: FnOnce(Octocrab) -> Fut,
        Fut: std::future::Future<Output = anyhow::Result<R>>,
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

#[cfg(test)]
mod tests {
    use chrono::{Datelike, Timelike};

    use super::*;

    #[test]
    fn test_parse_to_utc_valid() {
        let datetime_str = "2025-07-10T09:14:47Z";
        let result = parse_to_utc(datetime_str);
        assert_eq!(result.year(), 2025);
        assert_eq!(result.month(), 7);
        assert_eq!(result.day(), 10);
        assert_eq!(result.hour(), 9);
        assert_eq!(result.minute(), 14);
        assert_eq!(result.second(), 47);
    }

    #[test]
    fn test_parse_to_utc_with_microseconds() {
        let datetime_str = "2025-07-10T09:14:47.123456Z";
        let _ = parse_to_utc(datetime_str);
    }

    #[test]
    fn test_parse_to_utc_invalid() {
        let datetime_str = "invalid-datetime";
        let _ = parse_to_utc(datetime_str);
    }

    #[test]
    fn test_parse_to_utc_with_timezone_offset() {
        let datetime_str = "2025-07-10T09:14:47+02:00";
        let result = parse_to_utc(datetime_str);

        assert_eq!(result.hour(), 7); // 9 - 2 = 7 UTC
    }
}
