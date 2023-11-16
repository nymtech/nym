use std::{
    fmt::Display,
    net::{Ipv4Addr, SocketAddr},
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Types for GRPC
#[derive(Debug, Clone)]
pub struct NewSession {
    pub request_id: Uuid,
    pub device_unique_id: Uuid,
    pub location_code: String,
}

#[derive(Debug, Clone)]
pub struct EndSession {
    pub request_id: Uuid,
    pub device_unique_id: Uuid,
    pub vpn_session_uuid: Uuid,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClientConnected {
    pub request_id: Uuid,
    pub device_unique_id: Uuid,
    pub vpn_session_uuid: Uuid,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Accepted {
    pub request_id: Uuid,
    pub vpn_session_uuid: Uuid,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServerCreated {
    pub request_id: Uuid,
    pub vpn_session_uuid: Uuid,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Failed {
    pub request_id: Uuid,
    pub vpn_session_uuid: Uuid,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServerRunning {
    pub request_id: Uuid,
    pub vpn_session_uuid: Uuid,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServerReady {
    pub request_id: Uuid,
    pub vpn_session_uuid: Uuid,
    pub public_key: String,
    pub ipv4_endpoint: SocketAddr,
    pub private_ipv4: Ipv4Addr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ended {
    pub request_id: Uuid,
    pub vpn_session_uuid: Uuid,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct VpnSessionStatusRequest {
    pub request_id: Uuid,
    pub device_unique_id: Uuid,
    pub vpn_session_uuid: Uuid,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VpnSessionStatus {
    Accepted(Accepted),
    Failed(Failed),
    ServerCreated(ServerCreated),
    ServerRunning(ServerRunning),
    ServerReady(ServerReady),
    ClientConnected(ClientConnected),
    Ended(Ended),
}

#[derive(Clone, Debug)]
pub struct UserCredentials {
    pub email: String,
    pub password: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DeviceType {
    Linux,
    MacOS,
    Windows,
    IOS,
    Android,
}

#[derive(Debug)]
pub struct DeviceInfo {
    pub name: String,
    pub version: String,
    pub arch: String,
    pub public_key: String,
    pub unique_id: uuid::Uuid,
    pub device_type: DeviceType,
}

#[derive(Debug)]
pub struct AddDeviceRequest {
    pub user_creds: UserCredentials,
    pub device_info: DeviceInfo,
}

#[derive(Debug)]
pub struct AddDeviceResponse {
    pub token: String,
    pub device_addresses: DeviceAddresses,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceAddresses {
    pub ipv4_address: Ipv4Addr,
}

impl Display for DeviceAddresses {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DeviceAddresses(ipv4_address = {})", self.ipv4_address)
    }
}

// for protobuf
impl TryFrom<i32> for DeviceType {
    type Error = String;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => DeviceType::Linux,
            1 => DeviceType::MacOS,
            2 => DeviceType::Windows,
            3 => DeviceType::Android,
            4 => DeviceType::IOS,
            _ => Err("invalid device type")?,
        })
    }
}

impl From<DeviceType> for i32 {
    fn from(value: DeviceType) -> Self {
        match value {
            DeviceType::Linux => 0,
            DeviceType::MacOS => 1,
            DeviceType::Windows => 2,
            DeviceType::Android => 3,
            DeviceType::IOS => 4,
        }
    }
}

impl Display for VpnSessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "VpnSessionStatus: {}",
            match self {
                VpnSessionStatus::Accepted(accepted) => format!("{accepted}"),
                VpnSessionStatus::Failed(failed) => format!("{failed}"),
                VpnSessionStatus::ServerCreated(server_created) => format!("{server_created}"),
                VpnSessionStatus::ServerRunning(server_running) => format!("{server_running}"),
                VpnSessionStatus::ServerReady(server_ready) => format!("{server_ready}"),
                VpnSessionStatus::ClientConnected(client_connected) =>
                    format!("{client_connected}"),
                VpnSessionStatus::Ended(ended) => format!("{ended}"),
            }
        )
    }
}

impl Display for VpnSessionStatusRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "VpnSessionStatusRequest request_id: {}, device_unique_id: {}, vpn_session_uuid: {}",
            self.request_id, self.device_unique_id, self.vpn_session_uuid
        )
    }
}

impl Display for Accepted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Accepted request_id: {}, vpn_session_uuid: {}",
            self.request_id, self.vpn_session_uuid
        )
    }
}

impl Display for Failed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Failed request_id: {} vpn_session_uuid: {}",
            self.request_id, self.vpn_session_uuid
        )
    }
}

impl Display for ServerCreated {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ServerCreated request_id: {}, vpn_session_uuid: {}",
            self.request_id, self.vpn_session_uuid
        )
    }
}

impl Display for ServerRunning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ServerRunning request_id: {}, vpn_session_uuid: {}",
            self.request_id, self.vpn_session_uuid
        )
    }
}

impl Display for ServerReady {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ServerReady request_id: {}, vpn_session_uuid: {}, ipv4_endpoint: {}, private_ipv4: {}, public_key: {}",
            self.request_id,
            self.vpn_session_uuid,
            self.ipv4_endpoint,
            self.private_ipv4,
            self.public_key
        )
    }
}

impl Display for Ended {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Ended request_id: {}, vpn_session_uuid: {}, reason: {}",
            self.request_id, self.vpn_session_uuid, self.reason
        )
    }
}

impl Display for ClientConnected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ClientConnected request_id: {}, device_unique_id: {}, vpn_session_uuid: {}",
            self.request_id, self.device_unique_id, self.vpn_session_uuid
        )
    }
}

impl Display for EndSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "EndSession: request_id: {}, vpn_session_uuid: {}, device_unique_id: {}, reason: {} ",
            self.request_id, self.vpn_session_uuid, self.device_unique_id, self.reason
        )
    }
}
