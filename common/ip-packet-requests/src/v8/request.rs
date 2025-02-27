use std::fmt;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::sign::SignatureError;

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
    Connect(ConnectRequest),
    Disconnect(DisconnectRequest),
    Ping(PingRequest),
    Health(HealthRequest),
}

// A data request is when the client wants to send an IP packet to a destination.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DataRequest {
    pub ip_packets: bytes::Bytes,
}

// A dynamic connect request is when the client does not provide the internal IP address it will use
// on the ip packet router, and instead requests one to be assigned to it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ConnectRequest {
    pub request_id: u64,

    // The maximum time in milliseconds the IPR should wait when filling up a mix packet
    // with ip packets.
    pub buffer_timeout: Option<u64>,

    // Timestamp of when the request was sent by the client.
    pub timestamp: OffsetDateTime,
}

// A disconnect request is when the client wants to disconnect from the ip packet router and free
// up the allocated IP address.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DisconnectRequest {
    pub request_id: u64,

    // Timestamp of when the request was sent by the client.
    pub timestamp: OffsetDateTime,
}

// A ping request is when the client wants to check if the ip packet router is still alive.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PingRequest {
    pub request_id: u64,

    // Timestamp of when the request was sent by the client.
    pub timestamp: OffsetDateTime,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct HealthRequest {
    pub request_id: u64,

    // Timestamp of when the request was sent by the client.
    pub timestamp: OffsetDateTime,
}

impl IpPacketRequest {
    pub fn new_connect_request(buffer_timeout: Option<u64>) -> Result<(Self, u64), SignatureError> {
        let request_id = rand::random();
        let timestamp = OffsetDateTime::now_utc();
        let connect = ConnectRequest {
            request_id,
            buffer_timeout,
            timestamp,
        };
        let request = Self {
            version: VERSION,
            data: IpPacketRequestData::Control(Box::new(ControlRequest::Connect(connect))),
        };
        Ok((request, request_id))
    }

    pub fn new_disconnect_request() -> Result<(Self, u64), SignatureError> {
        let request_id = rand::random();
        let timestamp = OffsetDateTime::now_utc();
        let disconnect = DisconnectRequest {
            request_id,
            timestamp,
        };
        let request = Self {
            version: VERSION,
            data: IpPacketRequestData::Control(Box::new(ControlRequest::Disconnect(disconnect))),
        };
        Ok((request, request_id))
    }

    pub fn new_data_request(ip_packets: bytes::Bytes) -> Self {
        Self {
            version: VERSION,
            data: IpPacketRequestData::Data(DataRequest { ip_packets }),
        }
    }

    pub fn new_ping() -> (Self, u64) {
        let request_id = rand::random();
        let timestamp = OffsetDateTime::now_utc();
        let ping_request = PingRequest {
            request_id,
            timestamp,
        };
        let request = Self {
            version: VERSION,
            data: IpPacketRequestData::Control(Box::new(ControlRequest::Ping(ping_request))),
        };
        (request, request_id)
    }

    pub fn new_health_request() -> (Self, u64) {
        let request_id = rand::random();
        let timestamp = OffsetDateTime::now_utc();
        let health_request = HealthRequest {
            request_id,
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
            ControlRequest::Connect(request) => request.request_id,
            ControlRequest::Disconnect(request) => request.request_id,
            ControlRequest::Ping(request) => request.request_id,
            ControlRequest::Health(request) => request.request_id,
        }
    }
}

impl fmt::Display for ControlRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ControlRequest::Connect(_) => write!(f, "Connect"),
            ControlRequest::Disconnect(_) => write!(f, "Disconnect"),
            ControlRequest::Ping(_) => write!(f, "Ping"),
            ControlRequest::Health(_) => write!(f, "Health"),
        }
    }
}

impl ConnectRequest {
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

#[cfg(test)]
mod tests {
    use time::macros::datetime;

    use super::*;

    #[test]
    fn check_size_of_request() {
        let connect = IpPacketRequest {
            version: 4,
            data: IpPacketRequestData::Control(Box::new(ControlRequest::Connect(ConnectRequest {
                request_id: 123,
                buffer_timeout: None,
                timestamp: datetime!(2024-01-01 12:59:59.5 UTC),
            }))),
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
