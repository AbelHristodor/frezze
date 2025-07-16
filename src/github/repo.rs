use crate::github::{
    Github,
    types::{
        self,
        ruleset::input::{CreateRepositoryRulesetInput, StatusCheck},
    },
};
use anyhow::{Result, anyhow};

const GITHUB_CHECK_SUITE_NAME: &str = "Freeze";
/*
* The flow is:
* - Get the repository id (node_id), either with Graphql or Rest API
* - Use the Graphql mutation to create new branch protection rules: https://docs.github.com/en/graphql/reference/objects#repository
* - check if the branch protection rule exists, if not, create it
* - update all PRs with a check run that fails/succedes based on the freeze period
* - build the webhook that listens to events from PRs
{
  "input": {
    "sourceId": "R_kgDOHZL5sw",
    "conditions": {
      "refName": {
        "include": ["refs/heads/main"],
        "exclude": []
      }
    },
    "name": "Frezze Ruleset",
    "enforcement": "ACTIVE",
    "target": "BRANCH",
    "rules": [
      {
        "type": "REQUIRED_STATUS_CHECKS",
        "parameters": {
          "requiredStatusChecks": {
            "strictRequiredStatusChecksPolicy": true,
            "requiredStatusChecks": [
              {
                "context": "frezze",
                "integrationId": 1553098
              }
            ]
          }
        }
      }
    ]
  }
*
 */

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

    /// Queries the GitHub GraphQL API to get the repository ID for a given owner and repository name.
    pub async fn get_rulesets(
        &self,
        owner: &str,
        repo: &str,
        installation_id: u64,
    ) -> anyhow::Result<types::repository::RulesetConnection> {
        let response = self
            .with_installation_async(installation_id, |client| async move {
                let query = include_str!("graphql/repo.graphql");
                let variables = serde_json::json!({
                    "owner": owner,
                    "name": repo,
                    "followRenames": true,

                });
                let payload = serde_json::json!({
                    "query": query,
                    "variables": variables
                });

                let response: types::repository::RespositoryResponse =
                    client.graphql(&payload).await?;

                Ok(response)
            })
            .await?;
        if let Some(data) = response.data {
            if let Some(repository) = data.repository {
                return Ok(repository.rulesets);
            }
        }

        Err(anyhow::anyhow!("Repository not found or no ID returned"))
    }

    pub async fn create_ruleset(
        &self,
        repo_id: &str,
        app_id: u64,
        installation_id: u64,
    ) -> anyhow::Result<types::repository::Ruleset> {
        let response = self
            .with_installation_async(installation_id, |client| async move {
                let query = include_str!("graphql/create_ruleset.graphql");
                let input = serde_json::to_string(&CreateRepositoryRulesetInput::new(
                    repo_id.to_string(),
                    "Frezze Ruleset".to_string(),
                    vec!["refs/heads/main".to_string()],
                    vec![StatusCheck {
                        context: "frezze".to_string(),
                        integration_id: app_id,
                    }],
                ))?;
                let payload = serde_json::json!({
                    "query": query,
                    "variables": input
                });
                let response: types::ruleset::CreateRulesetResponse =
                    client.graphql(&payload).await?;
                Ok(response)
            })
            .await?;
        if let Some(data) = response.data {
            if let Some(ruleset) = data.create_repository_ruleset {
                return Ok(ruleset);
            }
            if let Some(errors) = response.errors {
                for error in errors {
                    tracing::error!("GraphQL error: {:?}", error);
                }
                return Err(anyhow::anyhow!("GraphQL errors occurred"));
            }
        }
        Err(anyhow::anyhow!("Failed to create ruleset"))
    }

    pub async fn create_check_run(
        &self,
        owner: &str,
        repo: &str,
        sha: &str,
        status: octocrab::params::checks::CheckRunStatus,
        conclusion: octocrab::params::checks::CheckRunConclusion,
        installation_id: u64,
    ) -> anyhow::Result<()> {
        self.with_installation_async(installation_id, |client| async move {
            client
                .checks(owner, repo)
                .create_check_run(GITHUB_CHECK_SUITE_NAME, sha.to_string())
                .conclusion(conclusion)
                .status(status)
                .send()
                .await
                .map_err(|e| {
                    tracing::error!("Failed to create check run: {:?}", e);
                    anyhow!("Failed to create check run: {}", e)
                })
        })
        .await
        .map_err(|e| {
            tracing::error!("Error creating check run: {}", e);
            anyhow!("Error creating check run: {}", e)
        })?;

        Ok(())
    }
}
