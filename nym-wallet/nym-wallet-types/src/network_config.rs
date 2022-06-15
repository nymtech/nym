use serde::{Deserialize, Serialize};
use std::fmt;

// When the UI queries validator urls we use this type
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "nym-wallet/src/types/rust/ValidatorUrls.ts")
)]
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidatorUrls {
    pub urls: Vec<ValidatorUrl>,
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export, export_to = "nym-wallet/src/types/rust/ValidatorUrl.ts")
)]
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidatorUrl {
    pub url: String,
    pub name: Option<String>,
}

// The type used when adding or removing validators, effectively the input.
// NOTE: we should consider if we want to split this up
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export, export_to = "nym-wallet/src/types/rust/ValidatorUrls.ts")
)]
#[derive(Debug, Serialize, Deserialize)]
pub struct Validator {
    pub nymd_url: String,
    pub nymd_name: Option<String>,
    pub api_url: Option<String>,
}

impl fmt::Display for Validator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let nymd_url = format!("nymd_url: {}", self.nymd_url);
        let api_url = self
            .api_url
            .as_ref()
            .map(|api_url| format!(", api_url: {}", api_url))
            .unwrap_or_default();
        write!(f, "{nymd_url}{api_url}")
    }
}
