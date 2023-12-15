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
}
