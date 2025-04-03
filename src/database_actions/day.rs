use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};


#[derive(Debug, Serialize, Deserialize)]
pub struct Day {
    pub date: DateTime,
    pub votes_yes: Vec<i64>,
    pub votes_no: Vec<i64>,
}
