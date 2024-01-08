use futures::channel::mpsc::UnboundedSender;
use nym_vpn_lib::NymVpnCtrlMessage;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use ts_rs::TS;

use crate::fs::data::AppData;

#[derive(Debug, Serialize, Deserialize, Clone, TS)]
#[ts(export)]
pub struct NodeConfig {
    pub id: String,
    pub country: Country,
}

#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub enum ConnectionState {
    Connected,
    #[default]
    Disconnected,
    Connecting,
    Disconnecting,
    Unknown,
}

#[derive(Default, Debug, Serialize, Deserialize, TS, Clone, PartialEq, Eq)]
#[ts(export)]
pub enum VpnMode {
    Mixnet,
    #[default]
    TwoHop,
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
    pub error: Option<String>,
    pub vpn_mode: VpnMode,
    pub entry_node: Option<NodeConfig>,
    pub exit_node: Option<NodeConfig>,
    pub entry_node_location: Option<Country>,
    pub exit_node_location: Option<Country>,
    pub tunnel: Option<TunnelConfig>,
    pub connection_start_time: Option<OffsetDateTime>,
    pub vpn_ctrl_tx: Option<UnboundedSender<NymVpnCtrlMessage>>,
}

impl From<&AppData> for AppState {
    fn from(app_data: &AppData) -> Self {
        AppState {
            entry_node_location: app_data.entry_node_location.clone(),
            exit_node_location: app_data.exit_node_location.clone(),
            vpn_mode: app_data.vpn_mode.clone().unwrap_or_default(),
            ..Default::default()
        }
    }
}

#[derive(Default, Serialize, Deserialize, Debug, Clone, TS)]
#[ts(export)]
pub struct Country {
    pub name: String,
    pub code: String,
}
