use serde::{Deserialize, Serialize};

#[allow(non_snake_case)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export, export_to = "nym-wallet/src/types/rust/AppEnv.ts")
)]
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct AppEnv {
    pub ADMIN_ADDRESS: Option<String>,
    pub SHOW_TERMINAL: Option<String>,
    pub ENABLE_QA_MODE: Option<String>,
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export, export_to = "nym-wallet/src/types/rust/AppVersion.ts")
)]
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct AppVersion {
    pub current_version: String,
    pub latest_version: String,
    pub is_update_available: bool,
}
