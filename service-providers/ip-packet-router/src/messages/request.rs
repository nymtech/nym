// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_ip_packet_requests::{v7, v8, IpPair};
use nym_sdk::mixnet::AnonymousSenderTag;

use crate::clients::ConnectedClientId;

use super::DeserializedIpPacketRequest;

// The internal representation of the request after deserialization, valid for all versions
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct IpPacketRequest {
    pub(crate) version: u8,
    pub(crate) data: IpPacketRequestData,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum IpPacketRequestData {
    StaticConnect(StaticConnectRequest),
    DynamicConnect(DynamicConnectRequest),
    Disconnect(DisconnectRequest),
    Data(DataRequest),
    Ping(PingRequest),
    Health(HealthRequest),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct StaticConnectRequest {
    pub(crate) request_id: u64,
    pub(crate) sent_by: ConnectedClientId,
    pub(crate) ips: IpPair,
    pub(crate) buffer_timeout: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DynamicConnectRequest {
    pub(crate) request_id: u64,
    pub(crate) sent_by: ConnectedClientId,
    pub(crate) buffer_timeout: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DisconnectRequest {
    pub(crate) request_id: u64,
    pub(crate) sent_by: ConnectedClientId,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DataRequest {
    pub(crate) ip_packets: bytes::Bytes,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PingRequest {
    pub(crate) request_id: u64,
    pub(crate) sent_by: ConnectedClientId,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct HealthRequest {
    pub(crate) request_id: u64,
    pub(crate) sent_by: ConnectedClientId,
}

impl From<v7::request::IpPacketRequest> for IpPacketRequest {
    fn from(request: v7::request::IpPacketRequest) -> Self {
        Self {
            version: 7,
            data: match request.data {
                v7::request::IpPacketRequestData::StaticConnect(inner) => {
                    IpPacketRequestData::StaticConnect(StaticConnectRequest {
                        request_id: inner.request.request_id,
                        sent_by: ConnectedClientId::NymAddress(Box::new(inner.request.reply_to)),
                        ips: inner.request.ips,
                        buffer_timeout: inner.request.buffer_timeout,
                    })
                }
                v7::request::IpPacketRequestData::DynamicConnect(inner) => {
                    IpPacketRequestData::DynamicConnect(DynamicConnectRequest {
                        request_id: inner.request.request_id,
                        sent_by: ConnectedClientId::NymAddress(Box::new(inner.request.reply_to)),
                        buffer_timeout: inner.request.buffer_timeout,
                    })
                }
                v7::request::IpPacketRequestData::Disconnect(inner) => {
                    IpPacketRequestData::Disconnect(DisconnectRequest {
                        request_id: inner.request.request_id,
                        sent_by: ConnectedClientId::NymAddress(Box::new(inner.request.reply_to)),
                    })
                }
                v7::request::IpPacketRequestData::Data(inner) => {
                    IpPacketRequestData::Data(DataRequest {
                        ip_packets: inner.ip_packets,
                    })
                }
                v7::request::IpPacketRequestData::Ping(inner) => {
                    IpPacketRequestData::Ping(PingRequest {
                        request_id: inner.request_id,
                        sent_by: ConnectedClientId::NymAddress(Box::new(inner.reply_to)),
                    })
                }
                v7::request::IpPacketRequestData::Health(inner) => {
                    IpPacketRequestData::Health(HealthRequest {
                        request_id: inner.request_id,
                        sent_by: ConnectedClientId::NymAddress(Box::new(inner.reply_to)),
                    })
                }
            },
        }
    }
}

impl From<(v8::request::IpPacketRequest, AnonymousSenderTag)> for IpPacketRequest {
    fn from((request, sender_tag): (v8::request::IpPacketRequest, AnonymousSenderTag)) -> Self {
        Self {
            version: 8,
            data: match request.data {
                v8::request::IpPacketRequestData::StaticConnect(inner) => {
                    IpPacketRequestData::StaticConnect(StaticConnectRequest {
                        request_id: inner.request.request_id,
                        sent_by: ConnectedClientId::SenderTag(sender_tag),
                        ips: inner.request.ips,
                        buffer_timeout: inner.request.buffer_timeout,
                    })
                }
                v8::request::IpPacketRequestData::DynamicConnect(inner) => {
                    IpPacketRequestData::DynamicConnect(DynamicConnectRequest {
                        request_id: inner.request.request_id,
                        sent_by: ConnectedClientId::SenderTag(sender_tag),
                        buffer_timeout: inner.request.buffer_timeout,
                    })
                }
                v8::request::IpPacketRequestData::Disconnect(inner) => {
                    IpPacketRequestData::Disconnect(DisconnectRequest {
                        request_id: inner.request.request_id,
                        sent_by: ConnectedClientId::SenderTag(sender_tag),
                    })
                }
                v8::request::IpPacketRequestData::Data(inner) => {
                    IpPacketRequestData::Data(DataRequest {
                        ip_packets: inner.ip_packets,
                    })
                }
                v8::request::IpPacketRequestData::Ping(inner) => {
                    IpPacketRequestData::Ping(PingRequest {
                        request_id: inner.request_id,
                        sent_by: ConnectedClientId::SenderTag(sender_tag),
                    })
                }
                v8::request::IpPacketRequestData::Health(inner) => {
                    IpPacketRequestData::Health(HealthRequest {
                        request_id: inner.request_id,
                        sent_by: ConnectedClientId::SenderTag(sender_tag),
                    })
                }
            },
        }
    }
}

impl From<DeserializedIpPacketRequest> for IpPacketRequest {
    fn from(request: DeserializedIpPacketRequest) -> Self {
        match request {
            DeserializedIpPacketRequest::V7(request) => request.into(),
            DeserializedIpPacketRequest::V8(request) => request.into(),
        }
    }
}
