use serde::{Deserialize, Serialize};

use crate::states::app::{NodeConfig, PrivacyMode};

#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub struct AppData {
    pub monitoring: Option<bool>,
    pub autoconnect: Option<bool>,
    pub killswitch: Option<bool>,
    pub privacy_mode: Option<PrivacyMode>,
    pub entry_node: Option<NodeConfig>,
    pub exit_node: Option<NodeConfig>,
}
