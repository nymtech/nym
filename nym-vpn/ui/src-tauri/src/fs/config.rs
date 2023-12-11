use std::{net::Ipv4Addr, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    /// Path pointing to an env file describing the network
    pub config_env_file: Option<PathBuf>,
    /// Mixnet public ID of the entry gateway
    pub entry_gateway: String,
    /// Mixnet recipient address
    pub exit_router: String,
    /// NYM API URL
    pub nym_api: String,
}
