use serde::{Deserialize, Serialize};

#[allow(non_snake_case)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export, export_to = "nym-wallet/src/types/rust/AppEnv.ts")
)]
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct AppEnv {
    pub ADMIN_ADDRESS: Option<String>,
    pub SHOW_TERMINAL: Option<String>,
}
