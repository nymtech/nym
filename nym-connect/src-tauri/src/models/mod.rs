use serde::{Deserialize, Serialize};

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ConnectResult {
    pub address: String,
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct DisconnectResult {
    pub success: bool,
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(rename_all = "lowercase"))]
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionStatusKind {
    Disconnected,
    Disconnecting,
    Connected,
    Connecting,
}

pub const APP_EVENT_CONNECTION_STATUS_CHANGED: &str = "app:connection-status-changed";

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Clone, serde::Serialize)]
pub struct AppEventConnectionStatusChangedPayload {
    pub status: ConnectionStatusKind,
}
