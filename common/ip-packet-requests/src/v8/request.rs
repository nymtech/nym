use std::{fmt, sync::Arc};

use nym_crypto::asymmetric::ed25519;
use nym_sphinx::addressing::Recipient;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{
    sign::{SignatureError, SignedRequest},
    IpPair,
};

use super::VERSION;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct IpPacketRequest {
    pub version: u8,
    pub data: IpPacketRequestData,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum IpPacketRequestData {
    Data(DataRequest),
    Control(Box<ControlRequest>),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ControlRequest {
    StaticConnect(SignedStaticConnectRequest),
    DynamicConnect(SignedDynamicConnectRequest),
    Disconnect(SignedDisconnectRequest),
    Ping(PingRequest),
    Health(HealthRequest),
}

// A data request is when the client wants to send an IP packet to a destination.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DataRequest {
    pub ip_packets: bytes::Bytes,
}

// A static connect request is when the client provides the internal IP address it will use on the
// ip packet router.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct StaticConnectRequest {
    pub request_id: u64,

    // The requested internal IP addresses.
    pub ips: IpPair,

    // The maximum time in milliseconds the IPR should wait when filling up a mix packet
    // with ip packets.
    pub buffer_timeout: Option<u64>,

    // Timestamp of when the request was sent by the client.
    pub timestamp: OffsetDateTime,

    pub sender: SentBy,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SignedStaticConnectRequest {
    pub request: StaticConnectRequest,
    pub signature: Option<ed25519::Signature>,
}

// A dynamic connect request is when the client does not provide the internal IP address it will use
// on the ip packet router, and instead requests one to be assigned to it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DynamicConnectRequest {
    pub request_id: u64,

    // The maximum time in milliseconds the IPR should wait when filling up a mix packet
    // with ip packets.
    pub buffer_timeout: Option<u64>,

    // Timestamp of when the request was sent by the client.
    pub timestamp: OffsetDateTime,

    pub sender: SentBy,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SignedDynamicConnectRequest {
    pub request: DynamicConnectRequest,
    pub signature: Option<ed25519::Signature>,
}

// A disconnect request is when the client wants to disconnect from the ip packet router and free
// up the allocated IP address.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DisconnectRequest {
    pub request_id: u64,

    // Timestamp of when the request was sent by the client.
    pub timestamp: OffsetDateTime,

    pub sender: SentBy,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SignedDisconnectRequest {
    pub request: DisconnectRequest,
    pub signature: Option<ed25519::Signature>,
}

// A ping request is when the client wants to check if the ip packet router is still alive.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PingRequest {
    pub request_id: u64,

    pub sender: SentBy,

    // Timestamp of when the request was sent by the client.
    pub timestamp: OffsetDateTime,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct HealthRequest {
    pub request_id: u64,

    pub sender: SentBy,

    // Timestamp of when the request was sent by the client.
    pub timestamp: OffsetDateTime,
}

// When constructing the request, use this return address. It has the keypair to be able to sign
// the request if we reveal the sender.
#[derive(Clone, Debug)]
pub enum ReturnAddress {
    AnonymousSenderTag,
    NymAddress {
        reply_to: Box<Recipient>,
        signing_keypair: Arc<ed25519::KeyPair>,
    },
}

// The serialized sender field in the request, that does not contain the keypair.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SentBy {
    AnonymousSenderTag,
    NymAddress(Box<Recipient>),
}

impl IpPacketRequest {
    pub fn new_connect_request(
        ips: Option<IpPair>,
        buffer_timeout: Option<u64>,
        return_address: ReturnAddress,
    ) -> Result<(Self, u64), SignatureError> {
        let request_id = rand::random();
        let timestamp = OffsetDateTime::now_utc();
        let sender = return_address.clone().into();
        let request = if let Some(ips) = ips {
            let request = StaticConnectRequest {
                request_id,
                ips,
                buffer_timeout,
                timestamp,
                sender,
            };
            let signature = return_address
                .signing_key()
                .map(|keypair| {
                    request
                        .to_bytes()
                        .map(|bytes| keypair.private_key().sign(bytes))
                })
                .transpose()?;
            ControlRequest::StaticConnect(SignedStaticConnectRequest { request, signature })
        } else {
            let request = DynamicConnectRequest {
                request_id,
                buffer_timeout,
                timestamp,
                sender,
            };
            let signature = return_address
                .signing_key()
                .map(|keypair| {
                    request
                        .to_bytes()
                        .map(|bytes| keypair.private_key().sign(bytes))
                })
                .transpose()?;
            ControlRequest::DynamicConnect(SignedDynamicConnectRequest { request, signature })
        };
        let request = Self {
            version: VERSION,
            data: IpPacketRequestData::Control(Box::new(request)),
        };
        Ok((request, request_id))
    }

    pub fn new_disconnect_request(
        return_address: ReturnAddress,
    ) -> Result<(Self, u64), SignatureError> {
        let request_id = rand::random();
        let timestamp = OffsetDateTime::now_utc();
        let sender = return_address.clone().into();
        let request = DisconnectRequest {
            request_id,
            timestamp,
            sender,
        };
        let signature = return_address
            .signing_key()
            .map(|keypair| {
                request
                    .to_bytes()
                    .map(|bytes| keypair.private_key().sign(bytes))
            })
            .transpose()?;
        let request = Self {
            version: VERSION,
            data: IpPacketRequestData::Control(Box::new(ControlRequest::Disconnect(
                SignedDisconnectRequest { request, signature },
            ))),
        };
        Ok((request, request_id))
    }

    pub fn new_data_request(ip_packets: bytes::Bytes) -> Self {
        Self {
            version: VERSION,
            data: IpPacketRequestData::Data(DataRequest { ip_packets }),
        }
    }

    pub fn new_ping(return_address: ReturnAddress) -> (Self, u64) {
        let request_id = rand::random();
        let timestamp = OffsetDateTime::now_utc();
        let sender = return_address.into();
        let ping_request = PingRequest {
            request_id,
            sender,
            timestamp,
        };
        let request = Self {
            version: VERSION,
            data: IpPacketRequestData::Control(Box::new(ControlRequest::Ping(ping_request))),
        };
        (request, request_id)
    }

    pub fn new_health_request(return_address: ReturnAddress) -> (Self, u64) {
        let request_id = rand::random();
        let timestamp = OffsetDateTime::now_utc();
        let sender = return_address.into();
        let health_request = HealthRequest {
            request_id,
            sender,
            timestamp,
        };
        let request = Self {
            version: VERSION,
            data: IpPacketRequestData::Control(Box::new(ControlRequest::Health(health_request))),
        };
        (request, request_id)
    }

    pub fn id(&self) -> Option<u64> {
        match self.data {
            IpPacketRequestData::Control(ref c) => Some(c.id()),
            IpPacketRequestData::Data(_) => None,
        }
    }

    pub fn verify(&self) -> Result<(), SignatureError> {
        match &self.data {
            IpPacketRequestData::Control(c) => c.verify(),
            IpPacketRequestData::Data(_) => Ok(()),
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        use bincode::Options;
        crate::make_bincode_serializer().serialize(self)
    }

    pub fn from_reconstructed_message(
        message: &nym_sphinx::receiver::ReconstructedMessage,
    ) -> Result<Self, bincode::Error> {
        use bincode::Options;
        crate::make_bincode_serializer().deserialize(&message.message)
    }
}

impl fmt::Display for IpPacketRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "IpPacketRequest {{ version: {}, data: {} }}",
            self.version, self.data
        )
    }
}

impl fmt::Display for IpPacketRequestData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IpPacketRequestData::Data(_) => write!(f, "Data"),
            IpPacketRequestData::Control(c) => write!(f, "Control({})", c),
        }
    }
}

impl ControlRequest {
    fn id(&self) -> u64 {
        match self {
            ControlRequest::StaticConnect(request) => request.request.request_id,
            ControlRequest::DynamicConnect(request) => request.request.request_id,
            ControlRequest::Disconnect(request) => request.request.request_id,
            ControlRequest::Ping(request) => request.request_id,
            ControlRequest::Health(request) => request.request_id,
        }
    }

    fn verify(&self) -> Result<(), SignatureError> {
        match self {
            ControlRequest::StaticConnect(request) => request.verify(),
            ControlRequest::DynamicConnect(request) => request.verify(),
            ControlRequest::Disconnect(request) => request.verify(),
            ControlRequest::Ping(_) => Ok(()),
            ControlRequest::Health(_) => Ok(()),
        }
    }
}

impl fmt::Display for ControlRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ControlRequest::StaticConnect(_) => write!(f, "StaticConnect"),
            ControlRequest::DynamicConnect(_) => write!(f, "DynamicConnect"),
            ControlRequest::Disconnect(_) => write!(f, "Disconnect"),
            ControlRequest::Ping(_) => write!(f, "Ping"),
            ControlRequest::Health(_) => write!(f, "Health"),
        }
    }
}

impl StaticConnectRequest {
    pub fn to_bytes(&self) -> Result<Vec<u8>, SignatureError> {
        use bincode::Options;
        crate::make_bincode_serializer()
            .serialize(self)
            .map_err(|error| SignatureError::RequestSerializationError {
                message: "failed to serialize request to binary".to_string(),
                error,
            })
    }
}

impl SignedRequest for SignedStaticConnectRequest {
    fn request_as_bytes(&self) -> Result<Vec<u8>, SignatureError> {
        self.request.to_bytes()
    }

    fn timestamp(&self) -> OffsetDateTime {
        self.request.timestamp
    }

    fn identity(&self) -> Option<&ed25519::PublicKey> {
        self.request.sender.identity()
    }

    fn signature(&self) -> Option<&ed25519::Signature> {
        self.signature.as_ref()
    }
}

impl DynamicConnectRequest {
    pub fn to_bytes(&self) -> Result<Vec<u8>, SignatureError> {
        use bincode::Options;
        crate::make_bincode_serializer()
            .serialize(self)
            .map_err(|error| SignatureError::RequestSerializationError {
                message: "failed to serialize request to binary".to_string(),
                error,
            })
    }
}

impl SignedRequest for SignedDynamicConnectRequest {
    fn request_as_bytes(&self) -> Result<Vec<u8>, SignatureError> {
        self.request.to_bytes()
    }

    fn timestamp(&self) -> OffsetDateTime {
        self.request.timestamp
    }

    fn identity(&self) -> Option<&ed25519::PublicKey> {
        self.request.sender.identity()
    }

    fn signature(&self) -> Option<&ed25519::Signature> {
        self.signature.as_ref()
    }
}

impl DisconnectRequest {
    pub fn to_bytes(&self) -> Result<Vec<u8>, SignatureError> {
        use bincode::Options;
        crate::make_bincode_serializer()
            .serialize(self)
            .map_err(|error| SignatureError::RequestSerializationError {
                message: "failed to serialize request to binary".to_string(),
                error,
            })
    }
}

impl SignedRequest for SignedDisconnectRequest {
    fn request_as_bytes(&self) -> Result<Vec<u8>, SignatureError> {
        self.request.to_bytes()
    }

    fn timestamp(&self) -> OffsetDateTime {
        self.request.timestamp
    }

    fn identity(&self) -> Option<&ed25519::PublicKey> {
        self.request.sender.identity()
    }

    fn signature(&self) -> Option<&ed25519::Signature> {
        self.signature.as_ref()
    }
}

impl SentBy {
    fn identity(&self) -> Option<&ed25519::PublicKey> {
        match self {
            SentBy::AnonymousSenderTag => None,
            SentBy::NymAddress(recipient) => Some(recipient.identity()),
        }
    }
}

impl From<Recipient> for SentBy {
    fn from(recipient: Recipient) -> Self {
        SentBy::NymAddress(Box::new(recipient))
    }
}

impl ReturnAddress {
    fn signing_key(&self) -> Option<&ed25519::KeyPair> {
        match self {
            ReturnAddress::AnonymousSenderTag => None,
            ReturnAddress::NymAddress {
                signing_keypair, ..
            } => Some(signing_keypair.as_ref()),
        }
    }
}

impl From<ReturnAddress> for SentBy {
    fn from(return_address: ReturnAddress) -> Self {
        match return_address {
            ReturnAddress::AnonymousSenderTag => SentBy::AnonymousSenderTag,
            ReturnAddress::NymAddress { reply_to, .. } => SentBy::NymAddress(reply_to),
        }
    }
}

#[cfg(test)]
mod tests {
    use time::macros::datetime;

    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};
    use std::str::FromStr;

    #[test]
    fn check_size_of_request() {
        let connect = IpPacketRequest {
            version: 4,
            data: IpPacketRequestData::Control(Box::new(ControlRequest::StaticConnect(
                SignedStaticConnectRequest {
                    request: StaticConnectRequest {
                        request_id: 123,
                        ips: IpPair::new(
                            Ipv4Addr::from_str("10.0.0.1").unwrap(),
                            Ipv6Addr::from_str("fc00::1").unwrap(),
                        ),
                        buffer_timeout: None,
                        timestamp: datetime!(2024-01-01 12:59:59.5 UTC),
                        sender: SentBy::AnonymousSenderTag,
                    },
                    signature: None,
                },
            ))),
        };
        assert_eq!(connect.to_bytes().unwrap().len(), 42);
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
