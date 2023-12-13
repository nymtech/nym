use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    /// Path pointing to an env configuration file describing the network
    pub env_config_file: Option<PathBuf>,
    /// Mixnet public ID of the entry gateway
    pub entry_gateway: String,
    /// Mixnet recipient address
    pub exit_router: String,
    /// NYM API URL
    /// ⚠ use the same as the one provided by the env_config_file
    /// TODO use the one provided by the env_config_file
    pub nym_api: String,
}
