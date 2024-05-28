use nym_crypto::asymmetric::identity;
use nym_sphinx::addressing::clients::Recipient;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{make_bincode_serializer, IpPair};

use super::{
    signature::{SignatureError, SignedRequest},
    VERSION,
};

fn generate_random() -> u64 {
    use rand::RngCore;
    let mut rng = rand::rngs::OsRng;
    rng.next_u64()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IpPacketRequest {
    pub version: u8,
    pub data: IpPacketRequestData,
}

impl IpPacketRequest {
    pub fn new_static_connect_request(
        ips: IpPair,
        reply_to: Recipient,
        reply_to_hops: Option<u8>,
        reply_to_avg_mix_delays: Option<f64>,
        buffer_timeout: Option<u64>,
    ) -> (Self, u64) {
        let request_id = generate_random();
        (
            Self {
                version: VERSION,
                data: IpPacketRequestData::StaticConnect(SignedStaticConnectRequest {
                    request: StaticConnectRequest {
                        request_id,
                        ips,
                        reply_to,
                        reply_to_hops,
                        reply_to_avg_mix_delays,
                        buffer_timeout,
                        timestamp: OffsetDateTime::now_utc(),
                    },
                    signature: None,
                }),
            },
            request_id,
        )
    }

    pub fn new_dynamic_connect_request(
        reply_to: Recipient,
        reply_to_hops: Option<u8>,
        reply_to_avg_mix_delays: Option<f64>,
        buffer_timeout: Option<u64>,
    ) -> (Self, u64) {
        let request_id = generate_random();
        (
            Self {
                version: VERSION,
                data: IpPacketRequestData::DynamicConnect(SignedDynamicConnectRequest {
                    request: DynamicConnectRequest {
                        request_id,
                        reply_to,
                        reply_to_hops,
                        reply_to_avg_mix_delays,
                        buffer_timeout,
                        timestamp: OffsetDateTime::now_utc(),
                    },
                    signature: None,
                }),
            },
            request_id,
        )
    }

    pub fn new_disconnect_request(reply_to: Recipient) -> (Self, u64) {
        let request_id = generate_random();
        (
            Self {
                version: VERSION,
                data: IpPacketRequestData::Disconnect(SignedDisconnectRequest {
                    request: DisconnectRequest {
                        request_id,
                        reply_to,
                        timestamp: OffsetDateTime::now_utc(),
                    },
                    signature: None,
                }),
            },
            request_id,
        )
    }

    pub fn new_data_request(ip_packets: bytes::Bytes) -> Self {
        Self {
            version: VERSION,
            data: IpPacketRequestData::Data(DataRequest { ip_packets }),
        }
    }

    pub fn new_ping(reply_to: Recipient) -> (Self, u64) {
        let request_id = generate_random();
        (
            Self {
                version: VERSION,
                data: IpPacketRequestData::Ping(PingRequest {
                    request_id,
                    reply_to,
                    timestamp: OffsetDateTime::now_utc(),
                }),
            },
            request_id,
        )
    }

    pub fn new_health_request(reply_to: Recipient) -> (Self, u64) {
        let request_id = generate_random();
        (
            Self {
                version: VERSION,
                data: IpPacketRequestData::Health(HealthRequest {
                    request_id,
                    reply_to,
                    timestamp: OffsetDateTime::now_utc(),
                }),
            },
            request_id,
        )
    }

    pub fn id(&self) -> Option<u64> {
        match &self.data {
            IpPacketRequestData::StaticConnect(request) => Some(request.request.request_id),
            IpPacketRequestData::DynamicConnect(request) => Some(request.request.request_id),
            IpPacketRequestData::Disconnect(request) => Some(request.request.request_id),
            IpPacketRequestData::Data(_) => None,
            IpPacketRequestData::Ping(request) => Some(request.request_id),
            IpPacketRequestData::Health(request) => Some(request.request_id),
        }
    }

    pub fn recipient(&self) -> Option<&Recipient> {
        match &self.data {
            IpPacketRequestData::StaticConnect(request) => Some(&request.request.reply_to),
            IpPacketRequestData::DynamicConnect(request) => Some(&request.request.reply_to),
            IpPacketRequestData::Disconnect(request) => Some(&request.request.reply_to),
            IpPacketRequestData::Data(_) => None,
            IpPacketRequestData::Ping(request) => Some(&request.reply_to),
            IpPacketRequestData::Health(request) => Some(&request.reply_to),
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
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum IpPacketRequestData {
    StaticConnect(SignedStaticConnectRequest),
    DynamicConnect(SignedDynamicConnectRequest),
    Disconnect(SignedDisconnectRequest),
    Data(DataRequest),
    Ping(PingRequest),
    Health(HealthRequest),
}

impl IpPacketRequestData {
    pub fn add_signature(&mut self, signature: Vec<u8>) -> Option<Vec<u8>> {
        match self {
            IpPacketRequestData::StaticConnect(request) => {
                request.signature = Some(signature);
                request.signature.clone()
            }
            IpPacketRequestData::DynamicConnect(request) => {
                request.signature = Some(signature);
                request.signature.clone()
            }
            IpPacketRequestData::Disconnect(request) => {
                request.signature = Some(signature);
                request.signature.clone()
            }
            IpPacketRequestData::Data(_)
            | IpPacketRequestData::Ping(_)
            | IpPacketRequestData::Health(_) => None,
        }
    }
}

// A static connect request is when the client provides the internal IP address it will use on the
// ip packet router.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct StaticConnectRequest {
    pub request_id: u64,

    pub ips: IpPair,

    // The nym-address the response should be sent back to
    pub reply_to: Recipient,

    // The number of mix node hops that responses should take, in addition to the entry and exit
    // node. Zero means only client -> entry -> exit -> client.
    pub reply_to_hops: Option<u8>,

    // The average delay at each mix node, in milliseconds. Currently this is not supported by the
    // ip packet router.
    pub reply_to_avg_mix_delays: Option<f64>,

    // The maximum time in milliseconds the IPR should wait when filling up a mix packet
    // with ip packets.
    pub buffer_timeout: Option<u64>,

    // Timestamp of when the request was sent by the client.
    pub timestamp: OffsetDateTime,
}

impl StaticConnectRequest {
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        use bincode::Options;
        make_bincode_serializer().serialize(self)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SignedStaticConnectRequest {
    pub request: StaticConnectRequest,
    pub signature: Option<Vec<u8>>,
}

impl SignedRequest for SignedStaticConnectRequest {
    fn identity(&self) -> &identity::PublicKey {
        self.request.reply_to.identity()
    }

    fn request(&self) -> Result<Vec<u8>, SignatureError> {
        self.request
            .to_bytes()
            .map_err(|error| SignatureError::RequestSerializationError {
                message: "failed to serialize request to binary".to_string(),
                error,
            })
    }

    fn signature(&self) -> Option<&Vec<u8>> {
        self.signature.as_ref()
    }

    fn timestamp(&self) -> OffsetDateTime {
        self.request.timestamp
    }
}

// A dynamic connect request is when the client does not provide the internal IP address it will use
// on the ip packet router, and instead requests one to be assigned to it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DynamicConnectRequest {
    pub request_id: u64,

    // The nym-address the response should be sent back to
    pub reply_to: Recipient,

    // The number of mix node hops that responses should take, in addition to the entry and exit
    // node. Zero means only client -> entry -> exit -> client.
    pub reply_to_hops: Option<u8>,

    // The average delay at each mix node, in milliseconds. Currently this is not supported by the
    // ip packet router.
    pub reply_to_avg_mix_delays: Option<f64>,

    // The maximum time in milliseconds the IPR should wait when filling up a mix packet
    // with ip packets.
    pub buffer_timeout: Option<u64>,

    // Timestamp of when the request was sent by the client.
    pub timestamp: OffsetDateTime,
}

impl DynamicConnectRequest {
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        use bincode::Options;
        make_bincode_serializer().serialize(self)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SignedDynamicConnectRequest {
    pub request: DynamicConnectRequest,
    pub signature: Option<Vec<u8>>,
}

impl SignedRequest for SignedDynamicConnectRequest {
    fn identity(&self) -> &identity::PublicKey {
        self.request.reply_to.identity()
    }

    fn request(&self) -> Result<Vec<u8>, SignatureError> {
        self.request
            .to_bytes()
            .map_err(|error| SignatureError::RequestSerializationError {
                message: "failed to serialize request to binary".to_string(),
                error,
            })
    }

    fn signature(&self) -> Option<&Vec<u8>> {
        self.signature.as_ref()
    }

    fn timestamp(&self) -> OffsetDateTime {
        self.request.timestamp
    }
}

// A disconnect request is when the client wants to disconnect from the ip packet router and free
// up the allocated IP address.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DisconnectRequest {
    pub request_id: u64,

    // The nym-address the response should be sent back to
    pub reply_to: Recipient,

    // Timestamp of when the request was sent by the client.
    pub timestamp: OffsetDateTime,
}

impl DisconnectRequest {
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        use bincode::Options;
        make_bincode_serializer().serialize(self)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SignedDisconnectRequest {
    pub request: DisconnectRequest,
    pub signature: Option<Vec<u8>>,
}

impl SignedRequest for SignedDisconnectRequest {
    fn identity(&self) -> &identity::PublicKey {
        self.request.reply_to.identity()
    }

    fn request(&self) -> Result<Vec<u8>, SignatureError> {
        self.request
            .to_bytes()
            .map_err(|error| SignatureError::RequestSerializationError {
                message: "failed to serialize request to binary".to_string(),
                error,
            })
    }

    fn signature(&self) -> Option<&Vec<u8>> {
        self.signature.as_ref()
    }

    fn timestamp(&self) -> OffsetDateTime {
        self.request.timestamp
    }
}

// A data request is when the client wants to send an IP packet to a destination.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DataRequest {
    pub ip_packets: bytes::Bytes,
}

// A ping request is when the client wants to check if the ip packet router is still alive.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PingRequest {
    pub request_id: u64,

    // The nym-address the response should be sent back to
    pub reply_to: Recipient,

    // Timestamp of when the request was sent by the client.
    pub timestamp: OffsetDateTime,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct HealthRequest {
    pub request_id: u64,

    // The nym-address the response should be sent back to
    pub reply_to: Recipient,

    // Timestamp of when the request was sent by the client.
    pub timestamp: OffsetDateTime,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};
    use std::str::FromStr;

    #[test]
    fn check_size_of_request() {
        let connect = IpPacketRequest {
            version: 4,
            data: IpPacketRequestData::StaticConnect(
                SignedStaticConnectRequest {
                    request: StaticConnectRequest {
                        request_id: 123,
                        ips: IpPair::new(Ipv4Addr::from_str("10.0.0.1").unwrap(), Ipv6Addr::from_str("2001:db8:a160::1").unwrap()),
                        reply_to: Recipient::try_from_base58_string("D1rrpsysCGCYXy9saP8y3kmNpGtJZUXN9SvFoUcqAsM9.9Ssso1ea5NfkbMASdiseDSjTN1fSWda5SgEVjdSN4CvV@GJqd3ZxpXWSNxTfx7B1pPtswpetH4LnJdFeLeuY5KUuN").unwrap(),
                        reply_to_hops: None,
                        reply_to_avg_mix_delays: None,
                        buffer_timeout: None,
                        timestamp: OffsetDateTime::now_utc(),
                    },
                    signature: None,
                }
            ),
        };
        assert_eq!(connect.to_bytes().unwrap().len(), 139);
    }

    #[test]
    fn check_size_of_data() {
        let data = IpPacketRequest {
            version: 4,
            data: IpPacketRequestData::Data(DataRequest {
                ip_packets: bytes::Bytes::from(vec![1u8; 32]),
            }),
        };
        assert_eq!(data.to_bytes().unwrap().len(), 35);
    }

    #[test]
    fn serialize_and_deserialize_data_request() {
        let data = IpPacketRequest {
            version: 4,
            data: IpPacketRequestData::Data(DataRequest {
                ip_packets: bytes::Bytes::from(vec![1, 2, 4, 2, 5]),
            }),
        };

        let serialized = data.to_bytes().unwrap();
        let deserialized = IpPacketRequest::from_reconstructed_message(
            &nym_sphinx::receiver::ReconstructedMessage {
                message: serialized,
                sender_tag: None,
            },
        )
        .unwrap();

        assert_eq!(deserialized.version, 4);
        assert_eq!(
            deserialized.data,
            IpPacketRequestData::Data(DataRequest {
                ip_packets: bytes::Bytes::from(vec![1, 2, 4, 2, 5]),
            })
        );
    }
}
