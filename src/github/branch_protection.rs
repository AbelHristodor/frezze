use crate::github::Github;
use anyhow::{Result, anyhow};

const BRANCH_PROTECTION_ENDPOINT: &str = "/repos/{owner}/{repo}/branches/{branch}/protection";

/*
* The flow is:
* - Get the repository id (node_id), either with Graphql or Rest API
* - Use the Graphql mutation to create new branch protection rules: https://docs.github.com/en/graphql/reference/objects#repository
* - check if the branch protection rule exists, if not, create it
* - update all PRs with a check run that fails/succedes based on the freeze period
* - build the webhook that listens to events from PRs
*
 */

impl Github {}
