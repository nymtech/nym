use crate::{
    v7::request::{
        DataRequest as DataRequestV7, DisconnectRequest as DisconnectRequestV7,
        DynamicConnectRequest as DynamicConnectRequestV7, HealthRequest as HealthRequestV7,
        IpPacketRequest as IpPacketRequestV7, IpPacketRequestData as IpPacketRequestDataV7,
        PingRequest as PingRequestV7, SignedDisconnectRequest as SignedDisconnectRequestV7,
        SignedDynamicConnectRequest as SignedDynamicConnectRequestV7,
        SignedStaticConnectRequest as SignedStaticConnectRequestV7,
        StaticConnectRequest as StaticConnectRequestV7,
    },
    v8::request::{
        ControlRequest as ControlRequestV8, DataRequest as DataRequestV8,
        DisconnectRequest as DisconnectRequestV8, DynamicConnectRequest as DynamicConnectRequestV8,
        HealthRequest as HealthRequestV8, IpPacketRequest as IpPacketRequestV8,
        IpPacketRequestData as IpPacketRequestDataV8, PingRequest as PingRequestV8,
        SignedDisconnectRequest as SignedDisconnectRequestV8,
        SignedDynamicConnectRequest as SignedDynamicConnectRequestV8,
        SignedStaticConnectRequest as SignedStaticConnectRequestV8,
        StaticConnectRequest as StaticConnectRequestV8,
    },
};

impl From<IpPacketRequestV7> for IpPacketRequestV8 {
    fn from(request: IpPacketRequestV7) -> Self {
        Self {
            version: request.version,
            data: request.data.into(),
        }
    }
}

impl From<IpPacketRequestDataV7> for IpPacketRequestDataV8 {
    fn from(request: IpPacketRequestDataV7) -> Self {
        match request {
            IpPacketRequestDataV7::StaticConnect(r) => {
                IpPacketRequestDataV8::Control(Box::new(ControlRequestV8::StaticConnect(r.into())))
            }
            IpPacketRequestDataV7::DynamicConnect(r) => {
                IpPacketRequestDataV8::Control(Box::new(ControlRequestV8::DynamicConnect(r.into())))
            }
            IpPacketRequestDataV7::Disconnect(r) => {
                IpPacketRequestDataV8::Control(Box::new(ControlRequestV8::Disconnect(r.into())))
            }
            IpPacketRequestDataV7::Data(r) => IpPacketRequestDataV8::Data(r.into()),
            IpPacketRequestDataV7::Ping(r) => {
                IpPacketRequestDataV8::Control(Box::new(ControlRequestV8::Ping(r.into())))
            }
            IpPacketRequestDataV7::Health(r) => {
                IpPacketRequestDataV8::Control(Box::new(ControlRequestV8::Health(r.into())))
            }
        }
    }
}

impl From<SignedStaticConnectRequestV7> for SignedStaticConnectRequestV8 {
    fn from(signed_static_connect_request: SignedStaticConnectRequestV7) -> Self {
        Self {
            request: signed_static_connect_request.request.into(),
            signature: signed_static_connect_request.signature,
        }
    }
}

impl From<StaticConnectRequestV7> for StaticConnectRequestV8 {
    fn from(static_connect_request: StaticConnectRequestV7) -> Self {
        Self {
            request_id: static_connect_request.request_id,
            ips: static_connect_request.ips,
            buffer_timeout: static_connect_request.buffer_timeout,
            timestamp: static_connect_request.timestamp,
            sender: static_connect_request.reply_to.into(),
        }
    }
}

impl From<SignedDynamicConnectRequestV7> for SignedDynamicConnectRequestV8 {
    fn from(signed_dynamic_connect_request: SignedDynamicConnectRequestV7) -> Self {
        Self {
            request: signed_dynamic_connect_request.request.into(),
            signature: signed_dynamic_connect_request.signature,
        }
    }
}

impl From<DynamicConnectRequestV7> for DynamicConnectRequestV8 {
    fn from(dynamic_connect_request: DynamicConnectRequestV7) -> Self {
        Self {
            request_id: dynamic_connect_request.request_id,
            buffer_timeout: dynamic_connect_request.buffer_timeout,
            timestamp: dynamic_connect_request.timestamp,
            sender: dynamic_connect_request.reply_to.into(),
        }
    }
}

impl From<SignedDisconnectRequestV7> for SignedDisconnectRequestV8 {
    fn from(signed_disconnect_request: SignedDisconnectRequestV7) -> Self {
        Self {
            request: signed_disconnect_request.request.into(),
            signature: signed_disconnect_request.signature,
        }
    }
}

impl From<DisconnectRequestV7> for DisconnectRequestV8 {
    fn from(disconnect_request: DisconnectRequestV7) -> Self {
        Self {
            request_id: disconnect_request.request_id,
            timestamp: disconnect_request.timestamp,
            sender: disconnect_request.reply_to.into(),
        }
    }
}

impl From<DataRequestV7> for DataRequestV8 {
    fn from(data_request: DataRequestV7) -> Self {
        Self {
            ip_packets: data_request.ip_packets,
        }
    }
}

impl From<PingRequestV7> for PingRequestV8 {
    fn from(ping_request: PingRequestV7) -> Self {
        Self {
            request_id: ping_request.request_id,
            sender: ping_request.reply_to.into(),
            timestamp: ping_request.timestamp,
        }
    }
}

impl From<HealthRequestV7> for HealthRequestV8 {
    fn from(health_request: HealthRequestV7) -> Self {
        Self {
            request_id: health_request.request_id,
            sender: health_request.reply_to.into(),
            timestamp: health_request.timestamp,
        }
    }
}
