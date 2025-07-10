use serde::{Deserialize, Serialize};

pub mod repository;

#[derive(Deserialize, Serialize, Debug)]
pub struct GraphQLError {
    message: String,
}
