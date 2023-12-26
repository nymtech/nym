use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    /// Path pointing to an env configuration file describing the network
    pub env_config_file: Option<PathBuf>,
    /// Country code (two letters format, eg. FR)
    pub entry_node_location: Option<String>,
}
