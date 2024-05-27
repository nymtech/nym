use nym_sphinx::addressing::clients::Recipient;
use serde::{Deserialize, Serialize};

use crate::{make_bincode_serializer, IpPair};

use super::VERSION;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IpPacketResponse {
    pub version: u8,
    pub data: IpPacketResponseData,
}

impl IpPacketResponse {
    pub fn new_static_connect_success(request_id: u64, reply_to: Recipient) -> Self {
        Self {
            version: VERSION,
            data: IpPacketResponseData::StaticConnect(StaticConnectResponse {
                request_id,
                reply_to,
                reply: StaticConnectResponseReply::Success,
            }),
        }
    }

    pub fn new_static_connect_failure(
        request_id: u64,
        reply_to: Recipient,
        reason: StaticConnectFailureReason,
    ) -> Self {
        Self {
            version: VERSION,
            data: IpPacketResponseData::StaticConnect(StaticConnectResponse {
                request_id,
                reply_to,
                reply: StaticConnectResponseReply::Failure(reason),
            }),
        }
    }

    pub fn new_dynamic_connect_success(request_id: u64, reply_to: Recipient, ips: IpPair) -> Self {
        Self {
            version: VERSION,
            data: IpPacketResponseData::DynamicConnect(DynamicConnectResponse {
                request_id,
                reply_to,
                reply: DynamicConnectResponseReply::Success(DynamicConnectSuccess { ips }),
            }),
        }
    }

    pub fn new_dynamic_connect_failure(
        request_id: u64,
        reply_to: Recipient,
        reason: DynamicConnectFailureReason,
    ) -> Self {
        Self {
            version: VERSION,
            data: IpPacketResponseData::DynamicConnect(DynamicConnectResponse {
                request_id,
                reply_to,
                reply: DynamicConnectResponseReply::Failure(reason),
            }),
        }
    }

    pub fn new_disconnect_success(request_id: u64, reply_to: Recipient) -> Self {
        Self {
            version: VERSION,
            data: IpPacketResponseData::Disconnect(DisconnectResponse {
                request_id,
                reply_to,
                reply: DisconnectResponseReply::Success,
            }),
        }
    }

    pub fn new_disconnect_failure(
        request_id: u64,
        reply_to: Recipient,
        reason: DisconnectFailureReason,
    ) -> Self {
        Self {
            version: VERSION,
            data: IpPacketResponseData::Disconnect(DisconnectResponse {
                request_id,
                reply_to,
                reply: DisconnectResponseReply::Failure(reason),
            }),
        }
    }

    pub fn new_unrequested_disconnect(
        reply_to: Recipient,
        reason: UnrequestedDisconnectReason,
    ) -> Self {
        Self {
            version: VERSION,
            data: IpPacketResponseData::UnrequestedDisconnect(UnrequestedDisconnect {
                reply_to,
                reason,
            }),
        }
    }

    pub fn new_ip_packet(ip_packet: bytes::Bytes) -> Self {
        Self {
            version: VERSION,
            data: IpPacketResponseData::Data(DataResponse { ip_packet }),
        }
    }

    pub fn new_version_mismatch(
        request_id: u64,
        reply_to: Recipient,
        request_version: u8,
        our_version: u8,
    ) -> Self {
        Self {
            version: VERSION,
            data: IpPacketResponseData::Info(InfoResponse {
                request_id,
                reply_to,
                reply: InfoResponseReply::VersionMismatch {
                    request_version,
                    response_version: our_version,
                },
                level: InfoLevel::Error,
            }),
        }
    }

    pub fn new_data_info_response(
        reply_to: Recipient,
        reply: InfoResponseReply,
        level: InfoLevel,
    ) -> Self {
        Self {
            version: VERSION,
            data: IpPacketResponseData::Info(InfoResponse {
                request_id: 0,
                reply_to,
                reply,
                level,
            }),
        }
    }

    pub fn new_pong(request_id: u64, reply_to: Recipient) -> Self {
        Self {
            version: VERSION,
            data: IpPacketResponseData::Pong(PongResponse {
                request_id,
                reply_to,
            }),
        }
    }

    pub fn new_health_response(
        request_id: u64,
        reply_to: Recipient,
        build_info: nym_bin_common::build_information::BinaryBuildInformationOwned,
        routable: Option<bool>,
    ) -> Self {
        Self {
            version: VERSION,
            data: IpPacketResponseData::Health(HealthResponse {
                request_id,
                reply_to,
                reply: HealthResponseReply {
                    build_info,
                    routable,
                },
            }),
        }
    }

    pub fn id(&self) -> Option<u64> {
        match &self.data {
            IpPacketResponseData::StaticConnect(response) => Some(response.request_id),
            IpPacketResponseData::DynamicConnect(response) => Some(response.request_id),
            IpPacketResponseData::Disconnect(response) => Some(response.request_id),
            IpPacketResponseData::UnrequestedDisconnect(_) => None,
            IpPacketResponseData::Data(_) => None,
            IpPacketResponseData::Pong(response) => Some(response.request_id),
            IpPacketResponseData::Health(response) => Some(response.request_id),
            IpPacketResponseData::Info(response) => Some(response.request_id),
        }
    }

    pub fn recipient(&self) -> Option<&Recipient> {
        match &self.data {
            IpPacketResponseData::StaticConnect(response) => Some(&response.reply_to),
            IpPacketResponseData::DynamicConnect(response) => Some(&response.reply_to),
            IpPacketResponseData::Disconnect(response) => Some(&response.reply_to),
            IpPacketResponseData::UnrequestedDisconnect(response) => Some(&response.reply_to),
            IpPacketResponseData::Data(_) => None,
            IpPacketResponseData::Pong(response) => Some(&response.reply_to),
            IpPacketResponseData::Health(response) => Some(&response.reply_to),
            IpPacketResponseData::Info(response) => Some(&response.reply_to),
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

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum IpPacketResponseData {
    // Response for a static connect request
    StaticConnect(StaticConnectResponse),

    // Response for a dynamic connect request
    DynamicConnect(DynamicConnectResponse),

    // Response for a disconnect initiqated by the client
    Disconnect(DisconnectResponse),

    // Message from the server that the client got disconnected without the client initiating it
    UnrequestedDisconnect(UnrequestedDisconnect),

    // Response to a data request
    Data(DataResponse),

    // Response to ping request
    Pong(PongResponse),

    // Response for a health request
    Health(HealthResponse),

    // Info response. This can be anything from informative messages to errors
    Info(InfoResponse),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StaticConnectResponse {
    pub request_id: u64,
    pub reply_to: Recipient,
    pub reply: StaticConnectResponseReply,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum StaticConnectResponseReply {
    Success,
    Failure(StaticConnectFailureReason),
}

impl StaticConnectResponseReply {
    pub fn is_success(&self) -> bool {
        match self {
            StaticConnectResponseReply::Success => true,
            StaticConnectResponseReply::Failure(_) => false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, thiserror::Error)]
pub enum StaticConnectFailureReason {
    #[error("requested ip address is already in use")]
    RequestedIpAlreadyInUse,
    #[error("requested nym-address is already in use")]
    RequestedNymAddressAlreadyInUse,
    #[error("{0}")]
    Other(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DynamicConnectResponse {
    pub request_id: u64,
    pub reply_to: Recipient,
    pub reply: DynamicConnectResponseReply,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DynamicConnectResponseReply {
    Success(DynamicConnectSuccess),
    Failure(DynamicConnectFailureReason),
}

impl DynamicConnectResponseReply {
    pub fn is_success(&self) -> bool {
        match self {
            DynamicConnectResponseReply::Success(_) => true,
            DynamicConnectResponseReply::Failure(_) => false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DynamicConnectSuccess {
    pub ips: IpPair,
}

#[derive(Clone, Debug, Serialize, Deserialize, thiserror::Error)]
pub enum DynamicConnectFailureReason {
    #[error("requested nym-address is already in use")]
    RequestedNymAddressAlreadyInUse,
    #[error("no available ip address")]
    NoAvailableIp,
    #[error("{0}")]
    Other(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DisconnectResponse {
    pub request_id: u64,
    pub reply_to: Recipient,
    pub reply: DisconnectResponseReply,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DisconnectResponseReply {
    Success,
    Failure(DisconnectFailureReason),
}

#[derive(Clone, Debug, Serialize, Deserialize, thiserror::Error)]
pub enum DisconnectFailureReason {
    #[error("requested nym-address is not currently connected")]
    RequestedNymAddressNotConnected,
    #[error("{0}")]
    Other(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnrequestedDisconnect {
    pub reply_to: Recipient,
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
pub struct DataResponse {
    pub ip_packet: bytes::Bytes,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PongResponse {
    pub request_id: u64,
    pub reply_to: Recipient,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub request_id: u64,
    pub reply_to: Recipient,
    pub reply: HealthResponseReply,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthResponseReply {
    // Return the binary build information of the IPR
    pub build_info: nym_bin_common::build_information::BinaryBuildInformationOwned,
    // Return if the IPR has performed a successful routing test.
    pub routable: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InfoResponse {
    pub request_id: u64,
    pub reply_to: Recipient,
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
