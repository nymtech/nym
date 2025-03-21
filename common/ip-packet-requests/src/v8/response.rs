use nym_bin_common::build_information::BinaryBuildInformationOwned;
use serde::{Deserialize, Serialize};

use crate::{make_bincode_serializer, IpPair};

use super::VERSION;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IpPacketResponse {
    pub version: u8,
    pub data: IpPacketResponseData,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum IpPacketResponseData {
    Data(DataResponse),
    Control(Box<ControlResponse>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataResponse {
    pub ip_packet: bytes::Bytes,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ControlResponse {
    // Response for a connect request
    Connect(ConnectResponse),

    // Response for a disconnect initiqated by the client
    Disconnect(DisconnectResponse),

    // Message from the server that the client got disconnected without the client initiating it
    UnrequestedDisconnect(UnrequestedDisconnect),

    // Response to ping request
    Pong(PongResponse),

    // Response for a health request
    Health(Box<HealthResponse>),

    // Info response. This can be anything from informative messages to errors
    Info(InfoResponse),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConnectResponse {
    pub request_id: u64,
    pub reply: ConnectResponseReply,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConnectResponseReply {
    Success(ConnectSuccess),
    Failure(ConnectFailureReason),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConnectSuccess {
    pub ips: IpPair,
}

#[derive(Clone, Debug, Serialize, Deserialize, thiserror::Error)]
pub enum ConnectFailureReason {
    #[error("client is already connected")]
    ClientAlreadyConnected,

    #[error("no available ip address")]
    NoAvailableIp,

    #[error("{0}")]
    Other(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DisconnectResponse {
    pub request_id: u64,
    pub reply: DisconnectResponseReply,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DisconnectResponseReply {
    Success,
    Failure(DisconnectFailureReason),
}

#[derive(Clone, Debug, Serialize, Deserialize, thiserror::Error)]
pub enum DisconnectFailureReason {
    #[error("client is not connected")]
    ClientNotConnected,

    #[error("{0}")]
    Other(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnrequestedDisconnect {
    pub reason: UnrequestedDisconnectReason,
}

#[derive(Clone, Debug, Serialize, Deserialize, thiserror::Error)]
pub enum UnrequestedDisconnectReason {
    #[error("client mixnet traffic timeout")]
    ClientMixnetTrafficTimeout,

    #[error("client tun traffic timeout")]
    ClientTunTrafficTimeout,

    #[error("{0}")]
    Other(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PongResponse {
    pub request_id: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub request_id: u64,
    pub reply: HealthResponseReply,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthResponseReply {
    // Return the binary build information of the IPR
    pub build_info: BinaryBuildInformationOwned,

    // Return if the IPR has performed a successful routing test.
    pub routable: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InfoResponse {
    pub request_id: u64,
    pub reply: InfoResponseReply,
    pub level: InfoLevel,
}

#[derive(Clone, Debug, Serialize, Deserialize, thiserror::Error)]
pub enum InfoResponseReply {
    #[error("{msg}")]
    Generic { msg: String },

    #[error(
        "version mismatch: response is v{request_version} and response is v{response_version}"
    )]
    VersionMismatch {
        request_version: u8,
        response_version: u8,
    },

    #[error("destination failed exit policy filter check: {dst}")]
    ExitPolicyFilterCheckFailed { dst: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum InfoLevel {
    Info,
    Warn,
    Error,
}

impl IpPacketResponse {
    pub fn new_ip_packet(ip_packet: bytes::Bytes) -> Self {
        Self {
            version: VERSION,
            data: IpPacketResponseData::Data(DataResponse { ip_packet }),
        }
    }

    pub fn id(&self) -> Option<u64> {
        match &self.data {
            IpPacketResponseData::Data(_) => None,
            IpPacketResponseData::Control(response) => response.id(),
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        use bincode::Options;
        make_bincode_serializer().serialize(self)
    }

    pub fn from_reconstructed_message(
        message: &nym_sphinx::receiver::ReconstructedMessage,
    ) -> Result<Self, bincode::Error> {
        use bincode::Options;
        make_bincode_serializer().deserialize(&message.message)
    }
}

impl IpPacketResponseData {
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        use bincode::Options;
        make_bincode_serializer().serialize(self)
    }
}

impl ControlResponse {
    fn id(&self) -> Option<u64> {
        match self {
            ControlResponse::Connect(response) => Some(response.request_id),
            ControlResponse::Disconnect(response) => Some(response.request_id),
            ControlResponse::UnrequestedDisconnect(_) => None,
            ControlResponse::Pong(response) => Some(response.request_id),
            ControlResponse::Health(response) => Some(response.request_id),
            ControlResponse::Info(response) => Some(response.request_id),
        }
    }
}

impl ConnectResponseReply {
    pub fn is_success(&self) -> bool {
        match self {
            ConnectResponseReply::Success(_) => true,
            ConnectResponseReply::Failure(_) => false,
        }
    }
}
