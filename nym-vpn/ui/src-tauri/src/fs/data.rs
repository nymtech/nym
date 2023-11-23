use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::states::app::{NodeConfig, PrivacyMode};

#[derive(Default, Serialize, Deserialize, Debug, Clone, TS)]
#[ts(export)]
pub enum UiMode {
    Dark,
    #[default]
    Light,
}

#[derive(Default, Serialize, Deserialize, Debug, Clone, TS)]
#[ts(export)]
pub struct AppData {
    pub monitoring: Option<bool>,
    pub autoconnect: Option<bool>,
    pub killswitch: Option<bool>,
    pub ui_mode: Option<UiMode>,
    pub privacy_mode: Option<PrivacyMode>,
    pub entry_node: Option<NodeConfig>,
    pub exit_node: Option<NodeConfig>,
}
