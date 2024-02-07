use std::net::IpAddr;

use nym_sphinx::addressing::clients::Recipient;
use serde::{Deserialize, Serialize};

use crate::{make_bincode_serializer, CURRENT_VERSION};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IpPacketResponse {
    pub version: u8,
    pub data: IpPacketResponseData,
}

impl IpPacketResponse {
    pub fn new_static_connect_success(request_id: u64, reply_to: Recipient) -> Self {
        Self {
            version: CURRENT_VERSION,
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
            version: CURRENT_VERSION,
            data: IpPacketResponseData::StaticConnect(StaticConnectResponse {
                request_id,
                reply_to,
                reply: StaticConnectResponseReply::Failure(reason),
            }),
        }
    }

    pub fn new_dynamic_connect_success(request_id: u64, reply_to: Recipient, ip: IpAddr) -> Self {
        Self {
            version: CURRENT_VERSION,
            data: IpPacketResponseData::DynamicConnect(DynamicConnectResponse {
                request_id,
                reply_to,
                reply: DynamicConnectResponseReply::Success(DynamicConnectSuccess { ip }),
            }),
        }
    }

    pub fn new_dynamic_connect_failure(
        request_id: u64,
        reply_to: Recipient,
        reason: DynamicConnectFailureReason,
    ) -> Self {
        Self {
            version: CURRENT_VERSION,
            data: IpPacketResponseData::DynamicConnect(DynamicConnectResponse {
                request_id,
                reply_to,
                reply: DynamicConnectResponseReply::Failure(reason),
            }),
        }
    }

    pub fn new_ip_packet(ip_packet: bytes::Bytes) -> Self {
        Self {
            version: CURRENT_VERSION,
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
            version: CURRENT_VERSION,
            data: IpPacketResponseData::Error(ErrorResponse {
                request_id,
                reply_to,
                reply: ErrorResponseReply::VersionMismatch {
                    request_version,
                    response_version: our_version,
                },
            }),
        }
    }

    pub fn new_data_error_response(reply_to: Recipient, reply: ErrorResponseReply) -> Self {
        Self {
            version: CURRENT_VERSION,
            data: IpPacketResponseData::Error(ErrorResponse {
                request_id: 0,
                reply_to,
                reply,
            }),
        }
    }

    pub fn id(&self) -> Option<u64> {
        match &self.data {
            IpPacketResponseData::StaticConnect(response) => Some(response.request_id),
            IpPacketResponseData::DynamicConnect(response) => Some(response.request_id),
            IpPacketResponseData::Disconnect(response) => Some(response.request_id),
            IpPacketResponseData::Data(_) => None,
            IpPacketResponseData::Error(response) => Some(response.request_id),
        }
    }

    pub fn recipient(&self) -> Option<&Recipient> {
        match &self.data {
            IpPacketResponseData::StaticConnect(response) => Some(&response.reply_to),
            IpPacketResponseData::DynamicConnect(response) => Some(&response.reply_to),
            IpPacketResponseData::Disconnect(response) => Some(&response.reply_to),
            IpPacketResponseData::Data(_) => None,
            IpPacketResponseData::Error(response) => Some(&response.reply_to),
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
    StaticConnect(StaticConnectResponse),
    DynamicConnect(DynamicConnectResponse),
    Disconnect(DisconnectResponse),
    Data(DataResponse),
    Error(ErrorResponse),
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
    pub ip: IpAddr,
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
pub struct DataResponse {
    pub ip_packet: bytes::Bytes,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub request_id: u64,
    pub reply_to: Recipient,
    pub reply: ErrorResponseReply,
}

#[derive(Clone, Debug, Serialize, Deserialize, thiserror::Error)]
pub enum ErrorResponseReply {
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
