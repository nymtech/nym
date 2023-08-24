// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_service_providers_common::interface;
use nym_service_providers_common::interface::ServiceProviderMessagingError;
use std::mem;
use thiserror::Error;

pub use request::*;
pub use response::*;
pub use version::*;

pub mod request;
pub mod response;
pub mod version;

pub type Socks5ProviderRequest = interface::Request<Socks5Request>;
pub type Socks5ProviderResponse = interface::Response<Socks5Request>;

#[derive(Debug, Error, PartialEq, Eq)]
#[error(
    "didn't receive enough data to recover socket data. got {received}, but expected at least {expected}"
)]
pub struct InsufficientSocketDataError {
    received: usize,
    expected: usize,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct SocketDataHeader {
    pub seq: u64,
    pub connection_id: ConnectionId,
    pub local_socket_closed: bool,
}

impl SocketDataHeader {
    const SERIALIZED_LEN: usize = mem::size_of::<ConnectionId>() + 1 + mem::size_of::<u64>();

    // we need to have two serialization methods for backwards compatibility,
    // since we serialized those fields differently depending on whether it was ingress vs egress...

    pub fn try_from_request_bytes(
        b: &[u8],
    ) -> Result<SocketDataHeader, InsufficientSocketDataError> {
        if b.len() != Self::SERIALIZED_LEN {
            return Err(InsufficientSocketDataError {
                received: b.len(),
                expected: Self::SERIALIZED_LEN,
            });
        }

        // the unwraps here are fine as we just ensured we have the exact amount of bytes we need
        let connection_id = ConnectionId::from_be_bytes(b[0..8].try_into().unwrap());
        let local_socket_closed = b[8] != 0;
        let seq = u64::from_be_bytes(b[9..].try_into().unwrap());

        Ok(SocketDataHeader {
            seq,
            connection_id,
            local_socket_closed,
        })
    }

    // the serialization of the header looks as follows:
    // (it's vital it's not modified as we need this exact structure for backwards compatibility)
    // CONNECTION_ID (8B) || SOCKET_CLOSED (1B) || SEQUENCE (8B)
    pub fn into_request_bytes(self) -> Vec<u8> {
        self.into_request_bytes_iter().collect()
    }

    pub fn into_request_bytes_iter(self) -> impl Iterator<Item = u8> {
        self.connection_id
            .to_be_bytes()
            .into_iter()
            .chain(std::iter::once(self.local_socket_closed as u8))
            .chain(self.seq.to_be_bytes())
    }

    pub fn try_from_response_bytes(
        b: &[u8],
    ) -> Result<SocketDataHeader, InsufficientSocketDataError> {
        if b.len() != Self::SERIALIZED_LEN {
            return Err(InsufficientSocketDataError {
                received: b.len(),
                expected: Self::SERIALIZED_LEN,
            });
        }

        // the unwraps here are fine as we just ensured we have the exact amount of bytes we need
        let local_socket_closed = b[0] != 0;
        let connection_id = ConnectionId::from_be_bytes(b[1..9].try_into().unwrap());
        let seq = u64::from_be_bytes(b[9..].try_into().unwrap());

        Ok(SocketDataHeader {
            seq,
            connection_id,
            local_socket_closed,
        })
    }

    // the serialization of the header looks as follows:
    // (it's vital it's not modified as we need this exact structure for backwards compatibility)
    // SOCKET_CLOSED (1B) || CONNECTION_ID (8B) || SEQUENCE (8B)
    pub fn into_response_bytes(self) -> Vec<u8> {
        self.into_response_bytes_iter().collect()
    }

    pub fn into_response_bytes_iter(self) -> impl Iterator<Item = u8> {
        std::iter::once(self.local_socket_closed as u8)
            .chain(self.connection_id.to_be_bytes())
            .chain(self.seq.to_be_bytes())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SocketData {
    pub header: SocketDataHeader,
    pub data: Vec<u8>,
}

impl SocketData {
    pub fn new(
        seq: u64,
        connection_id: ConnectionId,
        local_socket_closed: bool,
        data: Vec<u8>,
    ) -> Self {
        SocketData {
            header: SocketDataHeader {
                seq,
                connection_id,
                local_socket_closed,
            },
            data,
        }
    }

    fn verify_deserialization_len(b: &[u8]) -> Result<(), InsufficientSocketDataError> {
        if b.is_empty() {
            return Err(InsufficientSocketDataError {
                received: 0,
                expected: SocketDataHeader::SERIALIZED_LEN,
            });
        }

        if b.len() < SocketDataHeader::SERIALIZED_LEN {
            return Err(InsufficientSocketDataError {
                received: b.len(),
                expected: SocketDataHeader::SERIALIZED_LEN,
            });
        }
        Ok(())
    }

    // we need to have two serialization methods for backwards compatibility,
    // since we serialized those fields differently depending on whether it was ingress vs egress...
    pub fn try_from_request_bytes(b: &[u8]) -> Result<SocketData, InsufficientSocketDataError> {
        Self::verify_deserialization_len(b)?;
        let header =
            SocketDataHeader::try_from_request_bytes(&b[..SocketDataHeader::SERIALIZED_LEN])?;
        let data = b[SocketDataHeader::SERIALIZED_LEN..].to_vec();

        Ok(SocketData { header, data })
    }

    // the serialization of the socket data looks as follows:
    // HEADER || DATA
    pub fn into_request_bytes(self) -> Vec<u8> {
        self.into_request_bytes_iter().collect()
    }

    pub fn into_request_bytes_iter(self) -> impl Iterator<Item = u8> {
        self.header.into_request_bytes_iter().chain(self.data)
    }

    pub fn try_from_response_bytes(b: &[u8]) -> Result<SocketData, InsufficientSocketDataError> {
        Self::verify_deserialization_len(b)?;

        let header =
            SocketDataHeader::try_from_response_bytes(&b[..SocketDataHeader::SERIALIZED_LEN])?;
        let data = b[SocketDataHeader::SERIALIZED_LEN..].to_vec();

        Ok(SocketData { header, data })
    }

    pub fn into_response_bytes(self) -> Vec<u8> {
        self.into_response_bytes_iter().collect()
    }

    pub fn into_response_bytes_iter(self) -> impl Iterator<Item = u8> {
        self.header.into_response_bytes_iter().chain(self.data)
    }
}

#[derive(Debug, Error)]
pub enum Socks5RequestError {
    #[error("failed to deserialize received request: {source}")]
    RequestDeserialization {
        #[from]
        source: RequestDeserializationError,
    },

    #[error("failed to deserialize received response: {source}")]
    ResponseDeserialization {
        #[from]
        source: ResponseDeserializationError,
    },

    #[error(transparent)]
    ProviderInterfaceError(#[from] ServiceProviderMessagingError),

    #[error("received unsupported request protocol version: {protocol_version}")]
    UnsupportedProtocolVersion {
        protocol_version: <Socks5Request as interface::ServiceProviderRequest>::ProtocolVersion,
    },
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
    use nym_service_providers_common::interface::RequestContent;

    #[cfg(test)]
    mod socket_data_serialization {
        use super::*;

        #[test]
        fn for_requests() {
            assert_eq!(
                InsufficientSocketDataError {
                    received: 0,
                    expected: SocketDataHeader::SERIALIZED_LEN
                },
                SocketData::try_from_request_bytes(&[]).unwrap_err()
            );

            assert_eq!(
                InsufficientSocketDataError {
                    received: 10,
                    expected: SocketDataHeader::SERIALIZED_LEN
                },
                SocketData::try_from_request_bytes(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]).unwrap_err()
            );

            let good_data = SocketData::new(42, 12345, false, vec![2, 3]);
            let serialized = good_data.clone().into_request_bytes();

            assert_eq!(
                good_data,
                SocketData::try_from_request_bytes(&serialized).unwrap()
            );
            assert_ne!(
                good_data,
                SocketData::try_from_response_bytes(&serialized).unwrap()
            );

            let raw_bytes = [
                6, 6, 6, 6, 6, 6, 6, 6, 0, 0, 1, 2, 3, 4, 5, 6, 7, 255, 255, 255,
            ];
            assert_eq!(
                SocketData {
                    header: SocketDataHeader {
                        seq: u64::from_be_bytes([0, 1, 2, 3, 4, 5, 6, 7]),
                        connection_id: ConnectionId::from_be_bytes([6, 6, 6, 6, 6, 6, 6, 6]),
                        local_socket_closed: false,
                    },
                    data: vec![255, 255, 255],
                },
                SocketData::try_from_request_bytes(&raw_bytes).unwrap()
            )
        }

        #[test]
        fn for_responses() {
            assert_eq!(
                InsufficientSocketDataError {
                    received: 0,
                    expected: SocketDataHeader::SERIALIZED_LEN
                },
                SocketData::try_from_response_bytes(&[]).unwrap_err()
            );

            assert_eq!(
                InsufficientSocketDataError {
                    received: 10,
                    expected: SocketDataHeader::SERIALIZED_LEN
                },
                SocketData::try_from_response_bytes(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]).unwrap_err()
            );

            let good_data = SocketData::new(42, 12345, false, vec![2, 3]);
            let serialized = good_data.clone().into_response_bytes();

            assert_eq!(
                good_data,
                SocketData::try_from_response_bytes(&serialized).unwrap()
            );
            assert_ne!(
                good_data,
                SocketData::try_from_request_bytes(&serialized).unwrap()
            );

            let raw_bytes = [
                0, 6, 6, 6, 6, 6, 6, 6, 6, 0, 1, 2, 3, 4, 5, 6, 7, 255, 255, 255,
            ];
            assert_eq!(
                SocketData {
                    header: SocketDataHeader {
                        seq: u64::from_be_bytes([0, 1, 2, 3, 4, 5, 6, 7]),
                        connection_id: ConnectionId::from_be_bytes([6, 6, 6, 6, 6, 6, 6, 6]),
                        local_socket_closed: false,
                    },
                    data: vec![255, 255, 255],
                },
                SocketData::try_from_response_bytes(&raw_bytes).unwrap()
            )
        }
    }

    #[cfg(test)]
    mod interface_backwards_compatibility {
        use super::*;
        use nym_service_providers_common::interface::ProviderInterfaceVersion;

        #[test]
        fn old_client_vs_new_service_provider() {
            let old_serialized_connect = vec![
                0, 0, 2, 254, 34, 100, 192, 20, 13, 171, 0, 16, 56, 48, 46, 50, 52, 57, 46, 57, 57,
                46, 49, 52, 56, 58, 56, 48, 34, 112, 17, 182, 225, 6, 174, 216, 160, 41, 72, 236,
                160, 90, 156, 3, 250, 41, 243, 53, 191, 178, 218, 53, 170, 14, 185, 33, 94, 153,
                25, 41, 6, 82, 169, 187, 88, 246, 211, 57, 68, 225, 228, 231, 116, 29, 119, 235,
                160, 14, 156, 205, 66, 1, 75, 204, 204, 220, 14, 150, 191, 203, 174, 88, 121, 173,
                83, 219, 188, 164, 194, 212, 238, 228, 4, 128, 48, 105, 224, 83, 17, 246, 233, 16,
                235, 223, 68, 87, 13, 40, 34, 186, 218, 204, 126, 145,
            ];

            let new_deserialized =
                Socks5ProviderRequest::try_from_bytes(&old_serialized_connect).unwrap();

            match new_deserialized.content {
                RequestContent::ProviderData(req) => match req.content {
                    Socks5RequestContent::Connect(connect_req) => {
                        assert_eq!(connect_req.remote_addr, "80.249.99.148:80".to_string());
                        assert_eq!(connect_req.conn_id, 215647648274976171);
                        assert_eq!(connect_req.return_address, Some("3KRydEpanwjFhq5GAraVjRUF1Tno7w7oc4EwJYTGNo5J.RgZ7uMJHruBQqD5hC9Ghi3sqiTn6NycfM5qCfJz6yoM@9Byd9VAtyYMnbVAcqdoQxJnq76XEg2dbxbiF5Aa5Jj9J".parse().unwrap()));
                    }
                    _ => panic!("unexpected request"),
                },
                _ => panic!("unexpected request"),
            }

            let old_serialized_send = vec![
                0, 1, 108, 102, 28, 19, 50, 178, 37, 241, 0, 0, 0, 0, 0, 0, 0, 0, 0, 71, 69, 84,
                32, 47, 49, 77, 66, 46, 122, 105, 112, 32, 72, 84, 84, 80, 47, 49, 46, 49, 13, 10,
                72, 111, 115, 116, 58, 32, 105, 112, 118, 52, 46, 100, 111, 119, 110, 108, 111, 97,
                100, 46, 116, 104, 105, 110, 107, 98, 114, 111, 97, 100, 98, 97, 110, 100, 46, 99,
                111, 109, 13, 10, 85, 115, 101, 114, 45, 65, 103, 101, 110, 116, 58, 32, 99, 117,
                114, 108, 47, 55, 46, 54, 56, 46, 48, 13, 10, 65, 99, 99, 101, 112, 116, 58, 32,
                42, 47, 42, 13, 10, 13, 10,
            ];

            let new_deserialized =
                Socks5ProviderRequest::try_from_bytes(&old_serialized_send).unwrap();

            match new_deserialized.content {
                RequestContent::ProviderData(req) => match req.content {
                    Socks5RequestContent::Send(send_req) => {
                        assert_eq!(send_req.data.header.connection_id, 7810961472501196273);
                        assert_eq!(send_req.data.header.seq, 0);
                        assert_eq!(send_req.data.data.len(), 103);
                        assert!(!send_req.data.header.local_socket_closed);
                    }
                    _ => panic!("unexpected request"),
                },
                _ => panic!("unexpected request"),
            }
        }

        #[test]
        fn new_client_vs_old_service_provider() {
            let return_address = "3KRydEpanwjFhq5GAraVjRUF1Tno7w7oc4EwJYTGNo5J.RgZ7uMJHruBQqD5hC9Ghi3sqiTn6NycfM5qCfJz6yoM@9Byd9VAtyYMnbVAcqdoQxJnq76XEg2dbxbiF5Aa5Jj9J".parse().unwrap();

            let new_connect = Socks5ProviderRequest::new_provider_data(
                ProviderInterfaceVersion::Legacy,
                Socks5Request::new_connect(
                    Socks5ProtocolVersion::Legacy,
                    215647648274976171,
                    "80.249.99.148:80".to_string(),
                    Some(return_address),
                ),
            );

            let legacy_serialised = new_connect.into_bytes();
            let old_serialized_connect = vec![
                0, 0, 2, 254, 34, 100, 192, 20, 13, 171, 0, 16, 56, 48, 46, 50, 52, 57, 46, 57, 57,
                46, 49, 52, 56, 58, 56, 48, 34, 112, 17, 182, 225, 6, 174, 216, 160, 41, 72, 236,
                160, 90, 156, 3, 250, 41, 243, 53, 191, 178, 218, 53, 170, 14, 185, 33, 94, 153,
                25, 41, 6, 82, 169, 187, 88, 246, 211, 57, 68, 225, 228, 231, 116, 29, 119, 235,
                160, 14, 156, 205, 66, 1, 75, 204, 204, 220, 14, 150, 191, 203, 174, 88, 121, 173,
                83, 219, 188, 164, 194, 212, 238, 228, 4, 128, 48, 105, 224, 83, 17, 246, 233, 16,
                235, 223, 68, 87, 13, 40, 34, 186, 218, 204, 126, 145,
            ];

            assert_eq!(legacy_serialised, old_serialized_connect);
        }
    }
}
