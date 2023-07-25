use core::fmt;

use serde::{Deserialize, Serialize};

use crate::state::GatewayConnectivity;

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

#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(rename_all = "lowercase"))]
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub enum GatewayConnectionStatusKind {
    Good,
    Bad,
    VeryBad,
}

impl From<GatewayConnectivity> for GatewayConnectionStatusKind {
    fn from(conn: GatewayConnectivity) -> Self {
        match conn {
            GatewayConnectivity::Good => GatewayConnectionStatusKind::Good,
            GatewayConnectivity::Bad { .. } => GatewayConnectionStatusKind::Bad,
            GatewayConnectivity::VeryBad { .. } => GatewayConnectionStatusKind::VeryBad,
        }
    }
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(rename_all = "lowercase"))]
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConnectivityTestResult {
    NotAvailable,
    Success,
    Fail,
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
#[derive(Clone, Serialize)]
pub struct AppEventConnectionStatusChangedPayload {
    pub status: ConnectionStatusKind,
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DirectoryService {
    pub id: String,
    pub description: String,
    pub items: Vec<DirectoryServiceProvider>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HarbourMasterService {
    pub service_provider_client_id: String,
    pub ip_address: String,
    pub last_successful_ping_utc: String,
    pub last_updated_utc: String,
    pub routing_score: f32,
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DirectoryServiceProvider {
    pub id: String,
    pub description: String,
    /// Address of the network requester in the form "<client_id>.<client_enc>@<gateway_id>"
    /// e.g. DpB3cHAchJiNBQi5FrZx2csXb1mrHkpYh9Wzf8Rjsuko.ANNWrvHqMYuertHGHUrZdBntQhpzfbWekB39qez9U2Vx@2BuMSfMW3zpeAjKXyKLhmY4QW1DXurrtSPEJ6CjX3SEh
    pub address: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PagedResult<T> {
    pub page: u32,
    pub size: u32,
    pub total: i32,
    pub items: Vec<T>,
}

#[cfg_attr(test, derive(ts_rs::TS))]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Gateway {
    pub identity: String,
}
