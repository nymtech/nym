use crate::{v7, v8};

impl From<v7::request::IpPacketRequest> for v8::request::IpPacketRequest {
    fn from(ip_packet_request: v7::request::IpPacketRequest) -> Self {
        Self {
            version: 8,
            data: ip_packet_request.data.into(),
        }
    }
}

impl From<v7::request::IpPacketRequestData> for v8::request::IpPacketRequestData {
    fn from(ip_packet_request_data: v7::request::IpPacketRequestData) -> Self {
        match ip_packet_request_data {
            v7::request::IpPacketRequestData::StaticConnect(r) => {
                v8::request::IpPacketRequestData::StaticConnect(r.into())
            }
            v7::request::IpPacketRequestData::DynamicConnect(r) => {
                v8::request::IpPacketRequestData::DynamicConnect(r.into())
            }
            v7::request::IpPacketRequestData::Disconnect(r) => {
                v8::request::IpPacketRequestData::Disconnect(r.into())
            }
            v7::request::IpPacketRequestData::Data(r) => {
                v8::request::IpPacketRequestData::Data(r.into())
            }
            v7::request::IpPacketRequestData::Ping(r) => {
                v8::request::IpPacketRequestData::Ping(r.into())
            }
            v7::request::IpPacketRequestData::Health(r) => {
                v8::request::IpPacketRequestData::Health(r.into())
            }
        }
    }
}

impl From<v7::request::SignedStaticConnectRequest> for v8::request::SignedStaticConnectRequest {
    fn from(signed_static_connect_request: v7::request::SignedStaticConnectRequest) -> Self {
        Self {
            request: signed_static_connect_request.request.into(),
            signature: signed_static_connect_request.signature,
        }
    }
}

impl From<v7::request::StaticConnectRequest> for v8::request::StaticConnectRequest {
    fn from(static_connect_request: v7::request::StaticConnectRequest) -> Self {
        Self {
            request_id: static_connect_request.request_id,
            ips: static_connect_request.ips,
            reply_to_avg_mix_delays: static_connect_request.reply_to_avg_mix_delays,
            buffer_timeout: static_connect_request.buffer_timeout,
            timestamp: static_connect_request.timestamp,
            signed_by: *static_connect_request.reply_to.identity(),
        }
    }
}

impl From<v7::request::SignedDynamicConnectRequest> for v8::request::SignedDynamicConnectRequest {
    fn from(signed_dynamic_connect_request: v7::request::SignedDynamicConnectRequest) -> Self {
        Self {
            request: signed_dynamic_connect_request.request.into(),
            signature: signed_dynamic_connect_request.signature,
        }
    }
}

impl From<v7::request::DynamicConnectRequest> for v8::request::DynamicConnectRequest {
    fn from(dynamic_connect_request: v7::request::DynamicConnectRequest) -> Self {
        Self {
            request_id: dynamic_connect_request.request_id,
            reply_to_avg_mix_delays: dynamic_connect_request.reply_to_avg_mix_delays,
            buffer_timeout: dynamic_connect_request.buffer_timeout,
            timestamp: dynamic_connect_request.timestamp,
            signed_by: *dynamic_connect_request.reply_to.identity(),
        }
    }
}

impl From<v7::request::SignedDisconnectRequest> for v8::request::SignedDisconnectRequest {
    fn from(signed_disconnect_request: v7::request::SignedDisconnectRequest) -> Self {
        Self {
            request: signed_disconnect_request.request.into(),
            signature: signed_disconnect_request.signature,
        }
    }
}

impl From<v7::request::DisconnectRequest> for v8::request::DisconnectRequest {
    fn from(disconnect_request: v7::request::DisconnectRequest) -> Self {
        Self {
            request_id: disconnect_request.request_id,
            timestamp: disconnect_request.timestamp,
            signed_by: *disconnect_request.reply_to.identity(),
        }
    }
}

impl From<v7::request::DataRequest> for v8::request::DataRequest {
    fn from(data_request: v7::request::DataRequest) -> Self {
        Self {
            ip_packets: data_request.ip_packets,
        }
    }
}

impl From<v7::request::PingRequest> for v8::request::PingRequest {
    fn from(ping_request: v7::request::PingRequest) -> Self {
        Self {
            request_id: ping_request.request_id,
            timestamp: ping_request.timestamp,
        }
    }
}

impl From<v7::request::HealthRequest> for v8::request::HealthRequest {
    fn from(health_request: v7::request::HealthRequest) -> Self {
        Self {
            request_id: health_request.request_id,
            timestamp: health_request.timestamp,
        }
    }
}
