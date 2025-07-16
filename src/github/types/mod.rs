use serde::{Deserialize, Serialize};

pub mod repository;
pub mod ruleset;

#[derive(Deserialize, Serialize, Debug)]
pub struct GraphQLError {
    message: String,
}
