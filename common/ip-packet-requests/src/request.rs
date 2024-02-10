use std::net::IpAddr;

use nym_sphinx::addressing::clients::Recipient;
use serde::{Deserialize, Serialize};

use crate::{make_bincode_serializer, CURRENT_VERSION};

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
        buffer_timeout: Option<u64>,
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
                    buffer_timeout,
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
                version: CURRENT_VERSION,
                data: IpPacketRequestData::DynamicConnect(DynamicConnectRequest {
                    request_id,
                    reply_to,
                    reply_to_hops,
                    reply_to_avg_mix_delays,
                    buffer_timeout,
                }),
            },
            request_id,
        )
    }

    pub fn new_disconnect_request(reply_to: Recipient) -> (Self, u64) {
        let request_id = generate_random();
        (
            Self {
                version: CURRENT_VERSION,
                data: IpPacketRequestData::Disconnect(DisconnectRequest {
                    request_id,
                    reply_to,
                }),
            },
            request_id,
        )
    }

    pub fn new_data_request(ip_packets: bytes::Bytes) -> Self {
        Self {
            version: CURRENT_VERSION,
            data: IpPacketRequestData::Data(DataRequest { ip_packets }),
        }
    }

    pub fn id(&self) -> Option<u64> {
        match &self.data {
            IpPacketRequestData::StaticConnect(request) => Some(request.request_id),
            IpPacketRequestData::DynamicConnect(request) => Some(request.request_id),
            IpPacketRequestData::Disconnect(request) => Some(request.request_id),
            IpPacketRequestData::Data(_) => None,
            IpPacketRequestData::Ping(request) => Some(request.request_id),
            IpPacketRequestData::Health(request) => Some(request.request_id),
        }
    }

    pub fn recipient(&self) -> Option<&Recipient> {
        match &self.data {
            IpPacketRequestData::StaticConnect(request) => Some(&request.reply_to),
            IpPacketRequestData::DynamicConnect(request) => Some(&request.reply_to),
            IpPacketRequestData::Disconnect(request) => Some(&request.reply_to),
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
    StaticConnect(StaticConnectRequest),
    DynamicConnect(DynamicConnectRequest),
    Disconnect(DisconnectRequest),
    Data(DataRequest),
    Ping(PingRequest),
    Health(HealthRequest),
}

// A static connect request is when the client provides the internal IP address it will use on the
// ip packet router.
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

    // The maximum time in milliseconds the IPR should wait when filling up a mix packet
    // with ip packets.
    pub buffer_timeout: Option<u64>,
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
}

// A disconnect request is when the client wants to disconnect from the ip packet router and free
// up the allocated IP address.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DisconnectRequest {
    pub request_id: u64,
    // The nym-address the response should be sent back to
    pub reply_to: Recipient,
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
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct HealthRequest {
    pub request_id: u64,
    // The nym-address the response should be sent back to
    pub reply_to: Recipient,
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
                    buffer_timeout: None,
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
