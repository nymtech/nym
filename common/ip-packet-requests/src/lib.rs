use std::net::IpAddr;

use nym_sphinx::addressing::clients::Recipient;
use serde::{Deserialize, Serialize};

pub const CURRENT_VERSION: u8 = 1;

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
        ip: IpAddr,
        reply_to: Recipient,
        reply_to_hops: Option<u8>,
        reply_to_avg_mix_delays: Option<f64>,
    ) -> (Self, u64) {
        let request_id = generate_random();
        (
            Self {
                version: CURRENT_VERSION,
                data: IpPacketRequestData::StaticConnect(StaticConnectRequest {
                    request_id,
                    ip,
                    reply_to,
                    reply_to_hops,
                    reply_to_avg_mix_delays,
                }),
            },
            request_id,
        )
    }

    pub fn new_dynamic_connect_request(
        reply_to: Recipient,
        reply_to_hops: Option<u8>,
        reply_to_avg_mix_delays: Option<f64>,
    ) -> (Self, u64) {
        let request_id = generate_random();
        (
            Self {
                version: CURRENT_VERSION,
                data: IpPacketRequestData::DynamicConnect(DynamicConnectRequest {
                    request_id,
                    reply_to,
                    reply_to_hops,
                    reply_to_avg_mix_delays,
                }),
            },
            request_id,
        )
    }

    pub fn new_ip_packet(ip_packet: bytes::Bytes) -> Self {
        Self {
            version: CURRENT_VERSION,
            data: IpPacketRequestData::Data(DataRequest { ip_packet }),
        }
    }

    pub fn id(&self) -> Option<u64> {
        match &self.data {
            IpPacketRequestData::StaticConnect(request) => Some(request.request_id),
            IpPacketRequestData::DynamicConnect(request) => Some(request.request_id),
            IpPacketRequestData::Data(_) => None,
        }
    }

    pub fn recipient(&self) -> Option<&Recipient> {
        match &self.data {
            IpPacketRequestData::StaticConnect(request) => Some(&request.reply_to),
            IpPacketRequestData::DynamicConnect(request) => Some(&request.reply_to),
            IpPacketRequestData::Data(_) => None,
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
    StaticConnect(StaticConnectRequest),
    DynamicConnect(DynamicConnectRequest),
    Data(DataRequest),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct StaticConnectRequest {
    pub request_id: u64,
    pub ip: IpAddr,
    // The nym-address the response should be sent back to
    pub reply_to: Recipient,
    // The number of mix node hops that responses should take, in addition to the entry and exit
    // node. Zero means only client -> entry -> exit -> client.
    pub reply_to_hops: Option<u8>,
    // The average delay at each mix node, in milliseconds. Currently this is not supported by the
    // ip packet router.
    pub reply_to_avg_mix_delays: Option<f64>,
}

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
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DataRequest {
    pub ip_packet: bytes::Bytes,
}

// ---

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

    pub fn id(&self) -> Option<u64> {
        match &self.data {
            IpPacketResponseData::StaticConnect(response) => Some(response.request_id),
            IpPacketResponseData::DynamicConnect(response) => Some(response.request_id),
            IpPacketResponseData::Data(_) => None,
        }
    }

    pub fn recipient(&self) -> Option<&Recipient> {
        match &self.data {
            IpPacketResponseData::StaticConnect(response) => Some(&response.reply_to),
            IpPacketResponseData::DynamicConnect(response) => Some(&response.reply_to),
            IpPacketResponseData::Data(_) => None,
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
    Data(DataResponse),
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
pub struct DataResponse {
    pub ip_packet: bytes::Bytes,
}

fn make_bincode_serializer() -> impl bincode::Options {
    use bincode::Options;
    bincode::DefaultOptions::new()
        .with_big_endian()
        .with_varint_encoding()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_size_of_request() {
        let connect = IpPacketRequest {
            version: 4,
            data: IpPacketRequestData::StaticConnect(
                StaticConnectRequest {
                    request_id: 123,
                    ip: IpAddr::from([10, 0, 0, 1]),
                    reply_to: Recipient::try_from_base58_string("D1rrpsysCGCYXy9saP8y3kmNpGtJZUXN9SvFoUcqAsM9.9Ssso1ea5NfkbMASdiseDSjTN1fSWda5SgEVjdSN4CvV@GJqd3ZxpXWSNxTfx7B1pPtswpetH4LnJdFeLeuY5KUuN").unwrap(),
                    reply_to_hops: None,
                    reply_to_avg_mix_delays: None,
                },
            )
        };
        assert_eq!(connect.to_bytes().unwrap().len(), 107);
    }

    #[test]
    fn check_size_of_data() {
        let data = IpPacketRequest {
            version: 4,
            data: IpPacketRequestData::Data(DataRequest {
                ip_packet: bytes::Bytes::from(vec![1u8; 32]),
            }),
        };
        assert_eq!(data.to_bytes().unwrap().len(), 35);
    }

    #[test]
    fn serialize_and_deserialize_data_request() {
        let data = IpPacketRequest {
            version: 4,
            data: IpPacketRequestData::Data(DataRequest {
                ip_packet: bytes::Bytes::from(vec![1, 2, 4, 2, 5]),
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
                ip_packet: bytes::Bytes::from(vec![1, 2, 4, 2, 5]),
            })
        );
    }
}
