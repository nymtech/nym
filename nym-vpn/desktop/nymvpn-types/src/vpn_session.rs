use std::fmt::Display;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{location::Location, DateTimeUtc};

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type", content = "payload")]
pub enum VpnStatus {
    Accepted(Location),
    ServerCreated(Location),
    ServerRunning(Location),
    ServerReady(Location),
    Connecting(Location),
    Connected(Location, DateTimeUtc),
    Disconnecting(Location),
    Disconnected,
}

impl Display for VpnStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Status: {}",
            match self {
                VpnStatus::Accepted(location) => format!("Accepted, City: {}", location.city),
                VpnStatus::Connected(location, _) => format!("Connected, City: {}", location.city),
                VpnStatus::Connecting(location) => format!("Connecting, City: {}", location.city),
                VpnStatus::Disconnected => format!("Disconnected"),
                VpnStatus::Disconnecting(location) =>
                    format!("Disconnecting, City: {}", location.city),
                VpnStatus::ServerCreated(location) =>
                    format!("ServerCreated, City: {}", location.city),
                VpnStatus::ServerRunning(location) =>
                    format!("ServerRunning, City: {}", location.city),
                VpnStatus::ServerReady(location) => format!("ServerReady, City: {}", location.city),
            }
        )
    }
}
