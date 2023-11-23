use std::net::IpAddr;

use nym_sphinx::addressing::clients::Recipient;
use serde::{Deserialize, Serialize};

pub const CURRENT_VERSION: u8 = 1;

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
    ) -> Self {
        Self {
            version: CURRENT_VERSION,
            data: IpPacketRequestData::StaticConnect(StaticConnectRequest {
                ip,
                reply_to,
                reply_to_hops,
                reply_to_avg_mix_delays,
            }),
        }
    }

    pub fn new_dynamic_connect_request(
        reply_to: Recipient,
        reply_to_hops: Option<u8>,
        reply_to_avg_mix_delays: Option<f64>,
    ) -> Self {
        Self {
            version: CURRENT_VERSION,
            data: IpPacketRequestData::DynamicConnect(DynamicConnectRequest {
                reply_to,
                reply_to_hops,
                reply_to_avg_mix_delays,
            }),
        }
    }

    pub fn new_ip_packet(ip_packet: bytes::Bytes) -> Self {
        Self {
            version: CURRENT_VERSION,
            data: IpPacketRequestData::Data(DataRequest { ip_packet }),
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
pub enum IpPacketRequestData {
    StaticConnect(StaticConnectRequest),
    DynamicConnect(DynamicConnectRequest),
    Data(DataRequest),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StaticConnectRequest {
    pub ip: IpAddr,
    pub reply_to: Recipient,
    pub reply_to_hops: Option<u8>,
    pub reply_to_avg_mix_delays: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DynamicConnectRequest {
    pub reply_to: Recipient,
    pub reply_to_hops: Option<u8>,
    pub reply_to_avg_mix_delays: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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
    pub fn new_static_connect_success() -> Self {
        Self {
            version: CURRENT_VERSION,
            data: IpPacketResponseData::StaticConnect(StaticConnectResponse::Success),
        }
    }

    pub fn new_static_connect_failure() -> Self {
        Self {
            version: CURRENT_VERSION,
            data: IpPacketResponseData::StaticConnect(StaticConnectResponse::Failure),
        }
    }

    pub fn new_dynamic_connect_success(ip: IpAddr) -> Self {
        Self {
            version: CURRENT_VERSION,
            data: IpPacketResponseData::DynamicConnect(DynamicConnectResponse::Success(
                DynamicConnectSuccess { ip },
            )),
        }
    }

    pub fn new_dynamic_connect_failure() -> Self {
        Self {
            version: CURRENT_VERSION,
            data: IpPacketResponseData::DynamicConnect(DynamicConnectResponse::Failure),
        }
    }

    pub fn new_ip_packet(ip_packet: bytes::Bytes) -> Self {
        Self {
            version: CURRENT_VERSION,
            data: IpPacketResponseData::Data(DataResponse { ip_packet }),
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
pub enum StaticConnectResponse {
    Success,
    Failure,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DynamicConnectResponse {
    Success(DynamicConnectSuccess),
    Failure,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DynamicConnectSuccess {
    pub ip: IpAddr,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataResponse {
    pub ip_packet: bytes::Bytes,
}

// ---

// #[derive(serde::Serialize, serde::Deserialize)]
// pub struct TaggedIpPacket {
//     pub packet: bytes::Bytes,
//     pub return_address: Recipient,
//     pub return_mix_hops: Option<u8>,
//     // pub return_mix_delays: Option<f64>,
// }
//
// impl TaggedIpPacket {
//     pub fn new(
//         packet: bytes::Bytes,
//         return_address: Recipient,
//         return_mix_hops: Option<u8>,
//     ) -> Self {
//         TaggedIpPacket {
//             packet,
//             return_address,
//             return_mix_hops,
//         }
//     }
//
//     pub fn from_reconstructed_message(
//         message: &nym_sphinx::receiver::ReconstructedMessage,
//     ) -> Result<Self, bincode::Error> {
//         use bincode::Options;
//         make_bincode_serializer().deserialize(&message.message)
//     }
//
//     pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
//         use bincode::Options;
//         make_bincode_serializer().serialize(self)
//     }
// }

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
                    ip: IpAddr::from([10, 0, 0, 1]),
                    reply_to: Recipient::try_from_base58_string("D1rrpsysCGCYXy9saP8y3kmNpGtJZUXN9SvFoUcqAsM9.9Ssso1ea5NfkbMASdiseDSjTN1fSWda5SgEVjdSN4CvV@GJqd3ZxpXWSNxTfx7B1pPtswpetH4LnJdFeLeuY5KUuN").unwrap(),
                    reply_to_hops: None,
                    reply_to_avg_mix_delays: None,
                },
            )
        };

        // dbg!(&connect);
        // dbg!(&connect.to_bytes().unwrap());
        // dbg!(&connect.to_bytes().unwrap().len());
        assert_eq!(connect.to_bytes().unwrap().len(), 106);
    }

    #[test]
    fn check_size_of_data() {
        let data = IpPacketRequest {
            version: 4,
            data: IpPacketRequestData::Data(DataRequest {
                ip_packet: bytes::Bytes::from(vec![1u8; 32]),
            }),
        };

        // dbg!(&data);
        // dbg!(&data.to_bytes().unwrap());
        // dbg!(&data.to_bytes().unwrap().len());
        assert_eq!(data.to_bytes().unwrap().len(), 35);
    }
}
