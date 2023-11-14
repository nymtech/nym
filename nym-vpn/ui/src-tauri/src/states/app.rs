use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NodeConfig {
    pub id: String,
    pub country: String,
}

#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum ConnectionState {
    Connected,
    #[default]
    Disconnected,
    Connecting,
    Disconnecting,
    Error,
}

#[derive(Default, Debug, Serialize, Deserialize, TS, Clone)]
#[ts(export)]
pub enum PrivacyMode {
    High,
    Medium,
    #[default]
    Low,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TunnelConfig {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Default)]
pub struct AppState {
    pub state: ConnectionState,
    pub privacy_mode: PrivacyMode,
    pub entry_node: Option<NodeConfig>,
    pub exit_node: Option<NodeConfig>,
    pub tunnel: Option<TunnelConfig>,
}
