use core::fmt;

use serde::{Deserialize, Serialize};

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct ConnectResult {
    pub address: String,
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct DisconnectResult {
    pub success: bool,
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(rename_all = "lowercase"))]
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionStatusKind {
    Disconnected,
    Disconnecting,
    Connected,
    Connecting,
}

impl fmt::Display for ConnectionStatusKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ConnectionStatusKind::Disconnected => write!(f, "Disconnected"),
            ConnectionStatusKind::Disconnecting => write!(f, "Disconnecting"),
            ConnectionStatusKind::Connected => write!(f, "Connected"),
            ConnectionStatusKind::Connecting => write!(f, "Connecting"),
        }
    }
}

pub const APP_EVENT_CONNECTION_STATUS_CHANGED: &str = "app:connection-status-changed";

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Clone, serde::Serialize)]
pub struct AppEventConnectionStatusChangedPayload {
    pub status: ConnectionStatusKind,
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct DirectoryService {
    pub id: String,
    pub description: String,
    pub items: Vec<DirectoryServiceProvider>,
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct DirectoryServiceProvider {
    pub id: String,
    pub description: String,
    /// Address of the network requester in the form "<gateway_id>.<service_provider_id>"
    /// e.g. DpB3cHAchJiNBQi5FrZx2csXb1mrHkpYh9Wzf8Rjsuko.ANNWrvHqMYuertHGHUrZdBntQhpzfbWekB39qez9U2Vx@2BuMSfMW3zpeAjKXyKLhmY4QW1DXurrtSPEJ6CjX3SEh
    pub address: String,
    /// Address of the gateway, e.g. 2BuMSfMW3zpeAjKXyKLhmY4QW1DXurrtSPEJ6CjX3SEh
    pub gateway: String,
}
