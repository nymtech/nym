use std::fmt;

use nym_ip_packet_requests::{v7, v8, IpPair};
use nym_sdk::mixnet::{AnonymousSenderTag, Recipient};

use crate::error::{IpPacketRouterError, Result};

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum RequestSender {
    NymAddress(Box<Recipient>),
    SenderTag(AnonymousSenderTag),
}
impl RequestSender {
    pub(crate) fn into_nym_address(self) -> Option<Recipient> {
        match self {
            RequestSender::NymAddress(nym_address) => Some(*nym_address),
            RequestSender::SenderTag(_) => None,
        }
    }

    fn into_sender_tag(self) -> Option<AnonymousSenderTag> {
        match self {
            RequestSender::NymAddress(_) => None,
            RequestSender::SenderTag(tag) => Some(tag),
        }
    }
}

impl fmt::Display for RequestSender {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestSender::NymAddress(nym_address) => write!(f, "{nym_address}"),
            RequestSender::SenderTag(tag) => write!(f, "{tag}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct IpPacketRequest2 {
    pub version: u8,
    pub data: IpPacketRequestData2,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum IpPacketRequestData2 {
    StaticConnect(StaticConnectRequest2),
    DynamicConnect(DynamicConnectRequest2),
    Disconnect(DisconnectRequest2),
    Data(DataRequest2),
    Ping(PingRequest2),
    Health(HealthRequest2),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct StaticConnectRequest2 {
    pub(crate) request_id: u64,
    pub(crate) sent_by: RequestSender,
    pub(crate) ips: IpPair,
    pub(crate) buffer_timeout: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DynamicConnectRequest2 {
    pub(crate) request_id: u64,
    pub(crate) sent_by: RequestSender,
    pub(crate) buffer_timeout: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DisconnectRequest2 {
    pub(crate) request_id: u64,
    pub(crate) sent_by: RequestSender,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DataRequest2 {
    pub(crate) ip_packets: bytes::Bytes,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PingRequest2 {
    pub(crate) request_id: u64,
    pub(crate) sent_by: RequestSender,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct HealthRequest2 {
    pub(crate) request_id: u64,
    pub(crate) sent_by: RequestSender,
}

impl From<v7::request::IpPacketRequest> for IpPacketRequest2 {
    fn from(request: v7::request::IpPacketRequest) -> Self {
        Self {
            version: 7,
            data: match request.data {
                v7::request::IpPacketRequestData::StaticConnect(inner) => {
                    IpPacketRequestData2::StaticConnect(StaticConnectRequest2 {
                        request_id: inner.request.request_id,
                        sent_by: RequestSender::NymAddress(Box::new(inner.request.reply_to)),
                        ips: inner.request.ips,
                        buffer_timeout: inner.request.buffer_timeout,
                    })
                }
                v7::request::IpPacketRequestData::DynamicConnect(_) => {
                    todo!();
                }
                v7::request::IpPacketRequestData::Disconnect(_) => {
                    todo!();
                }
                v7::request::IpPacketRequestData::Data(inner) => {
                    IpPacketRequestData2::Data(DataRequest2 {
                        ip_packets: inner.ip_packets,
                    })
                }
                v7::request::IpPacketRequestData::Ping(_) => {
                    todo!();
                }
                v7::request::IpPacketRequestData::Health(_) => {
                    todo!();
                }
            },
        }
    }
}

impl From<(v8::request::IpPacketRequest, AnonymousSenderTag)> for IpPacketRequest2 {
    fn from((request, sender_tag): (v8::request::IpPacketRequest, AnonymousSenderTag)) -> Self {
        Self {
            version: 8,
            data: match request.data {
                v8::request::IpPacketRequestData::StaticConnect(inner) => {
                    IpPacketRequestData2::StaticConnect(StaticConnectRequest2 {
                        request_id: inner.request.request_id,
                        sent_by: RequestSender::SenderTag(sender_tag),
                        ips: inner.request.ips,
                        buffer_timeout: inner.request.buffer_timeout,
                    })
                }
                v8::request::IpPacketRequestData::DynamicConnect(_) => {
                    todo!();
                }
                v8::request::IpPacketRequestData::Disconnect(_) => {
                    todo!();
                }
                v8::request::IpPacketRequestData::Data(inner) => {
                    IpPacketRequestData2::Data(DataRequest2 {
                        ip_packets: inner.ip_packets,
                    })
                }
                v8::request::IpPacketRequestData::Ping(_) => {
                    todo!();
                }
                v8::request::IpPacketRequestData::Health(_) => {
                    todo!();
                }
            },
        }
    }
}

impl From<IpPacketRequest> for IpPacketRequest2 {
    fn from(request: IpPacketRequest) -> Self {
        match request {
            IpPacketRequest::V7(request) => request.into(),
            IpPacketRequest::V8(request) => request.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum IpPacketRequest {
    V7(v7::request::IpPacketRequest),
    V8((v8::request::IpPacketRequest, AnonymousSenderTag)),
}

impl IpPacketRequest {
    pub(crate) fn version(&self) -> u8 {
        match self {
            IpPacketRequest::V7(_) => 7,
            IpPacketRequest::V8(_) => 8,
        }
    }

    pub(crate) fn verify(&self) -> Result<()> {
        match self {
            IpPacketRequest::V7(request) => request.verify(),
            IpPacketRequest::V8(request) => request.0.verify(),
        }
        .map_err(|err| IpPacketRouterError::FailedToVerifyRequest { source: err })
    }
}

impl fmt::Display for IpPacketRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IpPacketRequest::V7(request) => write!(f, "{request}"),
            IpPacketRequest::V8((request, _)) => write!(f, "{request}"),
        }
    }
}

impl From<v7::request::IpPacketRequest> for IpPacketRequest {
    fn from(request: v7::request::IpPacketRequest) -> Self {
        IpPacketRequest::V7(request)
    }
}

impl From<(v8::request::IpPacketRequest, AnonymousSenderTag)> for IpPacketRequest {
    fn from(request: (v8::request::IpPacketRequest, AnonymousSenderTag)) -> Self {
        IpPacketRequest::V8(request)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SupportedClientVersion {
    V7,
    V8,
}

impl SupportedClientVersion {
    pub(crate) fn new(request_version: u8) -> Option<Self> {
        match request_version {
            7 => Some(SupportedClientVersion::V7),
            8 => Some(SupportedClientVersion::V8),
            _ => None,
        }
    }

    pub(crate) fn into_u8(self) -> u8 {
        match self {
            SupportedClientVersion::V7 => 7,
            SupportedClientVersion::V8 => 8,
        }
    }
}

pub(crate) type PacketHandleResult = Result<Option<VersionedResponse>>;

pub(crate) struct VersionedResponse {
    pub(crate) version: SupportedClientVersion,
    pub(crate) request_id: Option<u64>,
    pub(crate) reply_to: RequestSender,
    pub(crate) response: Response2,
}

#[derive(Debug, Clone)]
pub(crate) enum Response2 {
    StaticConnect(StaticConnectResponse),
    DynamicConnect(DynamicConnectResponse),
    Disconnect,
    Data,
    Pong,
    Health,
    Info(InfoResponse),
}

#[derive(Debug, Clone)]
pub(crate) enum StaticConnectResponse {
    Success,
    Failure(StaticConnectFailureReason),
}

#[derive(thiserror::Error, Debug, Clone)]
pub(crate) enum StaticConnectFailureReason {
    #[error("requested ip address is already in use")]
    RequestedIpAlreadyInUse,
    #[error("client already connected")]
    ClientAlreadyConnected,
    #[error("request timestamp is out of date")]
    OutOfDateTimestamp,
    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Clone)]
pub(crate) enum DynamicConnectResponse {
    Success,
    Failure(DynamicConnectFailureReason),
}

#[derive(Clone, Debug, thiserror::Error)]
pub(crate) enum DynamicConnectFailureReason {
    #[error("client already connected")]
    ClientAlreadyConnected,
    #[error("no available ip address")]
    NoAvailableIp,
    #[error("{0}")]
    Other(String),
}

impl From<VersionedResponse> for v7::response::IpPacketResponse {
    fn from(response: VersionedResponse) -> Self {
        match response.response {
            Response2::StaticConnect(inner) => v7::response::IpPacketResponse {
                version: response.version.into_u8(),
                data: v7::response::IpPacketResponseData::StaticConnect(
                    v7::response::StaticConnectResponse {
                        request_id: response.request_id.unwrap(),
                        reply_to: response.reply_to.into_nym_address().unwrap(),
                        reply: match inner {
                            StaticConnectResponse::Success => {
                                v7::response::StaticConnectResponseReply::Success
                            }
                            StaticConnectResponse::Failure(err) => {
                                v7::response::StaticConnectResponseReply::Failure(err.into())
                            }
                        },
                    },
                ),
            },
            Response2::DynamicConnect(_) => {
                todo!();
            }
            Response2::Disconnect => todo!(),
            Response2::Data => todo!(),
            Response2::Pong => todo!(),
            Response2::Health => todo!(),
            Response2::Info(inner) => v7::response::IpPacketResponse {
                version: response.version.into_u8(),
                data: v7::response::IpPacketResponseData::Info(v7::response::InfoResponse {
                    request_id: response.request_id.unwrap(),
                    reply_to: response.reply_to.into_nym_address().unwrap(),
                    reply: inner.reply.into(),
                    level: inner.level.into(),
                }),
            },
        }
    }
}

impl From<VersionedResponse> for v8::response::IpPacketResponse {
    fn from(response: VersionedResponse) -> Self {
        match response.response {
            Response2::StaticConnect(inner) => v8::response::IpPacketResponse {
                version: response.version.into_u8(),
                data: v8::response::IpPacketResponseData::StaticConnect(
                    v8::response::StaticConnectResponse {
                        request_id: response.request_id.unwrap(),
                        reply: match inner {
                            StaticConnectResponse::Success => {
                                v8::response::StaticConnectResponseReply::Success
                            }
                            StaticConnectResponse::Failure(err) => {
                                v8::response::StaticConnectResponseReply::Failure(err.into())
                            }
                        },
                    },
                ),
            },
            Response2::DynamicConnect(_) => {
                todo!();
            }
            Response2::Disconnect => todo!(),
            Response2::Data => todo!(),
            Response2::Pong => todo!(),
            Response2::Health => todo!(),
            Response2::Info(inner) => v8::response::IpPacketResponse {
                version: response.version.into_u8(),
                data: v8::response::IpPacketResponseData::Info(v8::response::InfoResponse {
                    request_id: response.request_id.unwrap(),
                    reply: inner.reply.into(),
                    level: inner.level.into(),
                }),
            },
        }
    }
}

impl From<StaticConnectFailureReason> for v7::response::StaticConnectFailureReason {
    fn from(reason: StaticConnectFailureReason) -> Self {
        match reason {
            StaticConnectFailureReason::RequestedIpAlreadyInUse => {
                v7::response::StaticConnectFailureReason::RequestedIpAlreadyInUse
            }
            StaticConnectFailureReason::ClientAlreadyConnected => {
                v7::response::StaticConnectFailureReason::RequestedNymAddressAlreadyInUse
            }
            StaticConnectFailureReason::OutOfDateTimestamp => {
                v7::response::StaticConnectFailureReason::OutOfDateTimestamp
            }
            StaticConnectFailureReason::Other(err) => {
                v7::response::StaticConnectFailureReason::Other(err)
            }
        }
    }
}

impl From<StaticConnectFailureReason> for v8::response::StaticConnectFailureReason {
    fn from(reason: StaticConnectFailureReason) -> Self {
        match reason {
            StaticConnectFailureReason::RequestedIpAlreadyInUse => {
                v8::response::StaticConnectFailureReason::RequestedIpAlreadyInUse
            }
            StaticConnectFailureReason::ClientAlreadyConnected => {
                v8::response::StaticConnectFailureReason::ClientAlreadyConnected
            }
            StaticConnectFailureReason::OutOfDateTimestamp => {
                v8::response::StaticConnectFailureReason::OutOfDateTimestamp
            }
            StaticConnectFailureReason::Other(err) => {
                v8::response::StaticConnectFailureReason::Other(err)
            }
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct InfoResponse {
    pub(crate) request_id: Option<u64>,
    pub(crate) reply: InfoResponseReply,
    pub(crate) level: InfoLevel,
}

#[derive(Clone, Debug, thiserror::Error)]
pub(crate) enum InfoResponseReply {
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

impl From<InfoResponseReply> for v7::response::InfoResponseReply {
    fn from(reply: InfoResponseReply) -> Self {
        match reply {
            InfoResponseReply::Generic { msg } => v7::response::InfoResponseReply::Generic { msg },
            InfoResponseReply::VersionMismatch {
                request_version,
                response_version,
            } => v7::response::InfoResponseReply::VersionMismatch {
                request_version,
                response_version,
            },
            InfoResponseReply::ExitPolicyFilterCheckFailed { dst } => {
                v7::response::InfoResponseReply::ExitPolicyFilterCheckFailed { dst }
            }
        }
    }
}

impl From<InfoResponseReply> for v8::response::InfoResponseReply {
    fn from(reply: InfoResponseReply) -> Self {
        match reply {
            InfoResponseReply::Generic { msg } => v8::response::InfoResponseReply::Generic { msg },
            InfoResponseReply::VersionMismatch {
                request_version,
                response_version,
            } => v8::response::InfoResponseReply::VersionMismatch {
                request_version,
                response_version,
            },
            InfoResponseReply::ExitPolicyFilterCheckFailed { dst } => {
                v8::response::InfoResponseReply::ExitPolicyFilterCheckFailed { dst }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum InfoLevel {
    Info,
    Warn,
    Error,
}

impl From<InfoLevel> for v7::response::InfoLevel {
    fn from(level: InfoLevel) -> Self {
        match level {
            InfoLevel::Info => v7::response::InfoLevel::Info,
            InfoLevel::Warn => v7::response::InfoLevel::Warn,
            InfoLevel::Error => v7::response::InfoLevel::Error,
        }
    }
}

impl From<InfoLevel> for v8::response::InfoLevel {
    fn from(level: InfoLevel) -> Self {
        match level {
            InfoLevel::Info => v8::response::InfoLevel::Info,
            InfoLevel::Warn => v8::response::InfoLevel::Warn,
            InfoLevel::Error => v8::response::InfoLevel::Error,
        }
    }
}

