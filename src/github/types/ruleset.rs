use serde::{Deserialize, Serialize};

use crate::github::types::repository::Ruleset;

#[derive(Deserialize, Serialize, Debug)]
pub struct CreateRulesetResponse {
    pub data: Option<Data>,
    pub errors: Option<Vec<super::GraphQLError>>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Data {
    #[serde(rename = "createRepositoryRuleset")]
    pub create_repository_ruleset: Option<Ruleset>,
}

pub mod input {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CreateRepositoryRulesetInput {
        #[serde(rename = "sourceId")]
        pub source_id: String,
        pub conditions: RulesetConditions,
        pub name: String,
        pub enforcement: RulesetEnforcement,
        pub target: RulesetTarget,
        pub rules: Vec<RulesetRule>,
        #[serde(rename = "clientMutationId", skip_serializing_if = "Option::is_none")]
        pub client_mutation_id: Option<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct RulesetConditions {
        #[serde(rename = "refName")]
        pub ref_name: RefNameCondition,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct RefNameCondition {
        pub include: Vec<String>,
        pub exclude: Vec<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum RulesetEnforcement {
        #[serde(rename = "ACTIVE")]
        Active,
        #[serde(rename = "DISABLED")]
        Disabled,
        #[serde(rename = "EVALUATE")]
        Evaluate,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum RulesetTarget {
        #[serde(rename = "BRANCH")]
        Branch,
        #[serde(rename = "TAG")]
        Tag,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct RulesetRule {
        #[serde(rename = "type")]
        pub rule_type: RuleType,
        pub parameters: RuleParameters,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum RuleType {
        #[serde(rename = "REQUIRED_STATUS_CHECKS")]
        RequiredStatusChecks,
        #[serde(rename = "REQUIRED_SIGNATURES")]
        RequiredSignatures,
        #[serde(rename = "PULL_REQUEST")]
        PullRequest,
        #[serde(rename = "REQUIRED_DEPLOYMENTS")]
        RequiredDeployments,
        #[serde(rename = "DELETION")]
        Deletion,
        #[serde(rename = "NON_FAST_FORWARD")]
        NonFastForward,
        #[serde(rename = "CREATION")]
        Creation,
        #[serde(rename = "UPDATE")]
        Update,
        #[serde(rename = "REQUIRED_LINEAR_HISTORY")]
        RequiredLinearHistory,
        #[serde(rename = "FORCE_PUSH")]
        ForcePush,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(untagged)]
    pub enum RuleParameters {
        RequiredStatusChecks(RequiredStatusChecksParameters),
        // Add other parameter types as needed
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct RequiredStatusChecksParameters {
        #[serde(rename = "requiredStatusChecks")]
        pub required_status_checks: RequiredStatusChecksConfig,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct RequiredStatusChecksConfig {
        #[serde(rename = "strictRequiredStatusChecksPolicy")]
        pub strict_required_status_checks_policy: bool,
        #[serde(rename = "requiredStatusChecks")]
        pub required_status_checks: Vec<StatusCheck>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct StatusCheck {
        pub context: String,
        #[serde(rename = "integrationId")]
        pub integration_id: u64,
    }

    // Example implementation
    impl CreateRepositoryRulesetInput {
        pub fn new(
            source_id: String,
            name: String,
            ref_patterns: Vec<String>,
            status_checks: Vec<StatusCheck>,
        ) -> Self {
            Self {
                source_id,
                conditions: RulesetConditions {
                    ref_name: RefNameCondition {
                        include: ref_patterns,
                        exclude: vec![],
                    },
                },
                name,
                enforcement: RulesetEnforcement::Active,
                target: RulesetTarget::Branch,
                rules: vec![RulesetRule {
                    rule_type: RuleType::RequiredStatusChecks,
                    parameters: RuleParameters::RequiredStatusChecks(
                        RequiredStatusChecksParameters {
                            required_status_checks: RequiredStatusChecksConfig {
                                strict_required_status_checks_policy: true,
                                required_status_checks: status_checks,
                            },
                        },
                    ),
                }],
                client_mutation_id: None,
            }
        }
    }
}
