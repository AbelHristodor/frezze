use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct RespositoryResponse {
    pub data: Option<Data>,
    pub errors: Option<Vec<super::GraphQLError>>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Data {
    pub repository: Option<Repository>,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Repository {
    pub id: String,
    created_at: String,
    database_id: i64,
    pub rulesets: RulesetConnection,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RulesetConnection {
    pub nodes: Vec<Ruleset>,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Ruleset {
    pub id: String,
    pub name: String,
    pub enforcement: String,
    pub target: String,
    pub conditions: RulesetConditions,
    pub rules: RuleConnection,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RulesetConditions {
    pub ref_name: RefName,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RefName {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RuleConnection {
    pub nodes: Vec<Rule>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Rule {
    pub id: String,
    #[serde(rename = "type")]
    pub rule_type: String,
    pub parameters: Option<RuleParameters>,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)] // Allows deserializing different rule parameter structures
pub enum RuleParameters {
    BranchNamePattern(BranchNamePatternParameters),
    CommitMessagePattern(CommitMessagePatternParameters),
    RequiredStatusChecks(RequiredStatusChecksParameters),
    PullRequest(PullRequestParameters),
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BranchNamePatternParameters {
    name: Option<String>,
    negate: bool,
    operator: String,
    pattern: String,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CommitMessagePatternParameters {
    name: Option<String>,
    negate: bool,
    operator: String,
    pattern: String,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RequiredStatusChecksParameters {
    strict_required_status_checks_policy: bool,
    required_status_checks: Vec<StatusCheck>,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StatusCheck {
    context: String,
    integration_id: Option<i64>,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PullRequestParameters {
    dismiss_stale_reviews_on_push: bool,
    require_code_owner_review: bool,
    required_approving_review_count: i32,
    required_review_thread_resolution: bool,
    require_last_push_approval: bool,
}
