use serde::{Deserialize, Serialize};

use crate::{IpPair, make_bincode_serializer};

use super::VERSION;

// Wire-format rule (see the module comment in `super`): redefine only the types
// on the path to a field that changed, and reuse the rest. v10 adds `mtu` to
// `ConnectSuccess`, so every type from `IpPacketResponse` down to
// `ConnectSuccess` is redefined below. bincode is positional and rejects
// trailing bytes, so this path cannot be a v8 type with a field appended; it has
// to be its own tree.
//
// The non-connect branches (disconnect, pong, health, info) are byte-identical
// to v8 and stay re-exported. Editing one of these v8 types therefore also moves
// v10's wire format for that branch, so treat them as frozen.
pub use crate::v8::response::{
    ConnectFailureReason, DataResponse, DisconnectResponse, HealthResponse, InfoLevel,
    InfoResponse, PongResponse, UnrequestedDisconnect,
};

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
pub enum ControlResponse {
    Connect(ConnectResponse),
    Disconnect(DisconnectResponse),
    UnrequestedDisconnect(UnrequestedDisconnect),
    Pong(PongResponse),
    Health(Box<HealthResponse>),
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

/// `mtu`: the largest IP packet (bytes) the IPR accepts on its egress TUN. The
/// client sizes its MTU to this so its advertised TCP MSS fits what the IPR forwards.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConnectSuccess {
    pub ips: IpPair,
    pub mtu: u16,
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

    pub fn from_bytes(data: &[u8]) -> Result<Self, bincode::Error> {
        use bincode::Options;
        make_bincode_serializer().deserialize(data)
    }

    pub fn from_reconstructed_message(
        message: &nym_sphinx::receiver::ReconstructedMessage,
    ) -> Result<Self, bincode::Error> {
        Self::from_bytes(&message.message)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    fn sample_ips() -> IpPair {
        IpPair::new(Ipv4Addr::new(10, 0, 0, 2), Ipv6Addr::LOCALHOST)
    }

    fn connect_success(mtu: u16) -> IpPacketResponse {
        IpPacketResponse {
            version: VERSION,
            data: IpPacketResponseData::Control(Box::new(ControlResponse::Connect(
                ConnectResponse {
                    request_id: 42,
                    reply: ConnectResponseReply::Success(ConnectSuccess {
                        ips: sample_ips(),
                        mtu,
                    }),
                },
            ))),
        }
    }

    #[test]
    fn connect_success_round_trips_with_mtu() {
        let bytes = connect_success(1500).to_bytes().unwrap();
        let decoded = IpPacketResponse::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.version, VERSION);
        let IpPacketResponseData::Control(control) = decoded.data else {
            panic!("expected control response");
        };
        let ControlResponse::Connect(connect) = *control else {
            panic!("expected connect response");
        };
        let ConnectResponseReply::Success(success) = connect.reply else {
            panic!("expected success");
        };
        assert_eq!(success.ips, sample_ips());
        assert_eq!(success.mtu, 1500);
    }

    // A v10 decoder cannot read a v8 connect response (EOF on the absent mtu),
    // which is why the field needed a new version rather than extending v8 in place.
    #[test]
    fn v10_decoder_rejects_v8_connect_response() {
        use crate::v8;

        let v8_response = v8::response::IpPacketResponse {
            version: v8::VERSION,
            data: v8::response::IpPacketResponseData::Control(Box::new(
                v8::response::ControlResponse::Connect(v8::response::ConnectResponse {
                    request_id: 42,
                    reply: v8::response::ConnectResponseReply::Success(
                        v8::response::ConnectSuccess { ips: sample_ips() },
                    ),
                }),
            )),
        };
        let v8_bytes = v8_response.to_bytes().unwrap();

        assert!(
            IpPacketResponse::from_bytes(&v8_bytes).is_err(),
            "v10 decoder must not silently accept a v8 connect response"
        );
    }
}
