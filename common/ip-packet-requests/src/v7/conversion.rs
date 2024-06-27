use time::OffsetDateTime;

use crate::{v6, v7};

impl From<v6::request::IpPacketRequest> for v7::request::IpPacketRequest {
    fn from(ip_packet_request: v6::request::IpPacketRequest) -> Self {
        Self {
            version: 7,
            data: ip_packet_request.data.into(),
        }
    }
}

impl From<v6::request::IpPacketRequestData> for v7::request::IpPacketRequestData {
    fn from(ip_packet_request_data: v6::request::IpPacketRequestData) -> Self {
        match ip_packet_request_data {
            v6::request::IpPacketRequestData::StaticConnect(r) => {
                v7::request::IpPacketRequestData::StaticConnect(
                    v7::request::SignedStaticConnectRequest {
                        request: r.into(),
                        signature: None,
                    },
                )
            }
            v6::request::IpPacketRequestData::DynamicConnect(r) => {
                v7::request::IpPacketRequestData::DynamicConnect(
                    v7::request::SignedDynamicConnectRequest {
                        request: r.into(),
                        signature: None,
                    },
                )
            }
            v6::request::IpPacketRequestData::Disconnect(r) => {
                v7::request::IpPacketRequestData::Disconnect(v7::request::SignedDisconnectRequest {
                    request: r.into(),
                    signature: None,
                })
            }
            v6::request::IpPacketRequestData::Data(r) => {
                v7::request::IpPacketRequestData::Data(r.into())
            }
            v6::request::IpPacketRequestData::Ping(r) => {
                v7::request::IpPacketRequestData::Ping(r.into())
            }
            v6::request::IpPacketRequestData::Health(r) => {
                v7::request::IpPacketRequestData::Health(r.into())
            }
        }
    }
}

impl From<v6::request::StaticConnectRequest> for v7::request::StaticConnectRequest {
    fn from(static_connect_request: v6::request::StaticConnectRequest) -> Self {
        Self {
            request_id: static_connect_request.request_id,
            ips: static_connect_request.ips,
            reply_to: static_connect_request.reply_to,
            reply_to_hops: static_connect_request.reply_to_hops,
            reply_to_avg_mix_delays: static_connect_request.reply_to_avg_mix_delays,
            buffer_timeout: static_connect_request.buffer_timeout,
            timestamp: OffsetDateTime::now_utc(),
        }
    }
}

impl From<v6::request::DynamicConnectRequest> for v7::request::DynamicConnectRequest {
    fn from(dynamic_connect_request: v6::request::DynamicConnectRequest) -> Self {
        Self {
            request_id: dynamic_connect_request.request_id,
            reply_to: dynamic_connect_request.reply_to,
            reply_to_hops: dynamic_connect_request.reply_to_hops,
            reply_to_avg_mix_delays: dynamic_connect_request.reply_to_avg_mix_delays,
            buffer_timeout: dynamic_connect_request.buffer_timeout,
            timestamp: OffsetDateTime::now_utc(),
        }
    }
}

impl From<v6::request::DisconnectRequest> for v7::request::SignedDisconnectRequest {
    fn from(disconnect_request: v6::request::DisconnectRequest) -> Self {
        Self {
            request: disconnect_request.into(),
            signature: None,
        }
    }
}

impl From<v6::request::DisconnectRequest> for v7::request::DisconnectRequest {
    fn from(disconnect_request: v6::request::DisconnectRequest) -> Self {
        Self {
            request_id: disconnect_request.request_id,
            reply_to: disconnect_request.reply_to,
            timestamp: OffsetDateTime::now_utc(),
        }
    }
}

impl From<v6::request::DataRequest> for v7::request::DataRequest {
    fn from(data_request: v6::request::DataRequest) -> Self {
        Self {
            ip_packets: data_request.ip_packets,
        }
    }
}

impl From<v6::request::PingRequest> for v7::request::PingRequest {
    fn from(ping_request: v6::request::PingRequest) -> Self {
        Self {
            request_id: ping_request.request_id,
            reply_to: ping_request.reply_to,
            timestamp: OffsetDateTime::now_utc(),
        }
    }
}

impl From<v6::request::HealthRequest> for v7::request::HealthRequest {
    fn from(health_request: v6::request::HealthRequest) -> Self {
        Self {
            request_id: health_request.request_id,
            reply_to: health_request.reply_to,
            timestamp: OffsetDateTime::now_utc(),
        }
    }
}
