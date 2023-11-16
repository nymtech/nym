use std::fmt::Display;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Location {
    pub code: String,
    pub country: String,
    pub country_code: String,
    pub city: String,
    pub city_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_code: Option<String>,
}

impl Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.state {
            Some(state) => {
                write!(f, "{}, {}, {}", self.city, state, self.country)
            }
            None => {
                write!(f, "{}, {}", self.city, self.country)
            }
        }
    }
}
