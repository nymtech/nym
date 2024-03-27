// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    make_bincode_serializer, InsufficientSocketDataError, SocketData, Socks5ProtocolVersion,
    Socks5RequestError, Socks5Response,
};
use nym_service_providers_common::interface::{Serializable, ServiceProviderRequest};
use nym_sphinx_addressing::clients::{Recipient, RecipientFormattingError};
use serde::{Deserialize, Serialize};

use std::fmt::{Debug, Formatter};
use tap::TapFallible;
use thiserror::Error;

pub type ConnectionId = u64;
pub type RemoteAddress = String;

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum RequestFlag {
    Connect = 0,
    Send = 1,
    Query = 2,
}

impl TryFrom<u8> for RequestFlag {
    type Error = RequestDeserializationError;

    fn try_from(value: u8) -> Result<RequestFlag, RequestDeserializationError> {
        match value {
            _ if value == (RequestFlag::Connect as u8) => Ok(Self::Connect),
            _ if value == (RequestFlag::Send as u8) => Ok(Self::Send),
            _ if value == (RequestFlag::Query as u8) => Ok(Self::Query),
            value => Err(RequestDeserializationError::UnknownRequestFlag { value }),
        }
    }
}

#[derive(Debug, Error)]
pub enum RequestDeserializationError {
    #[error("not enough bytes to recover the length of the address")]
    AddressLengthTooShort,

    #[error("not enough bytes to recover the address")]
    AddressTooShort,

    #[error("not enough bytes to recover the connection id")]
    ConnectionIdTooShort,

    #[error("no data provided")]
    NoData,

    #[error("{value} is not a valid request flag")]
    UnknownRequestFlag { value: u8 },

    #[error("too short return address")]
    ReturnAddressTooShort,

    #[error("malformed return address - {0}")]
    MalformedReturnAddress(RecipientFormattingError),

    #[error("failed to deserialize query request: {source}")]
    QueryDeserializationError {
        #[from]
        source: bincode::Error,
    },

    #[error(transparent)]
    InvalidSocketData(#[from] InsufficientSocketDataError),
}

impl RequestDeserializationError {
    pub fn is_malformed_return(&self) -> bool {
        matches!(self, RequestDeserializationError::MalformedReturnAddress(_))
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ConnectRequest {
    // TODO: is connection_id redundant now?
    pub conn_id: ConnectionId,
    pub remote_addr: RemoteAddress,
    pub return_address: Option<Recipient>,
}

impl Debug for ConnectRequest {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectRequest")
            .field("conn_id", &self.conn_id)
            .field("remote_addr", &self.remote_addr)
            .field(
                "return_address",
                &self.return_address.map(|r| r.to_string()),
            )
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendRequest {
    pub data: SocketData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum QueryRequest {
    OpenProxy,
    Description,
    ExitPolicy,
}

#[derive(Debug, Clone)]
pub struct Socks5Request {
    pub protocol_version: Socks5ProtocolVersion,
    pub content: Socks5RequestContent,
}

impl Serializable for Socks5Request {
    type Error = Socks5RequestError;

    // legacy requests had the format of
    // 0 (Message::REQUEST_FLAG) || 0 (RequestFlag::Connect) || <data> for connect requests
    // 0 (Message::REQUEST_FLAG) || 1 (RequestFlag::Send) || <data> for send requests
    // the updated formats use
    // 3 (Socks5ProtocolVersion) || 0 (RequestFlag::Connect) || <data> for connect requests
    // 3 (Socks5ProtocolVersion) || 1 (RequestFlag::Send) || <data> for send requests
    // in both cases, the actual data is serialized the same way, so the process is quite straight forward
    fn into_bytes(self) -> Vec<u8> {
        if let Some(version) = self.protocol_version.as_u8() {
            std::iter::once(version)
                .chain(self.content.into_bytes())
                .collect()
        } else {
            std::iter::once(Self::LEGACY_TYPE_TAG)
                .chain(self.content.into_bytes())
                .collect()
        }
    }

    fn try_from_bytes(b: &[u8]) -> Result<Self, Self::Error> {
        if b.is_empty() {
            return Err(RequestDeserializationError::NoData.into());
        }

        let protocol_version = Socks5ProtocolVersion::from(b[0]);
        if protocol_version > Self::max_supported_version() {
            return Err(Socks5RequestError::UnsupportedProtocolVersion { protocol_version });
        }

        // TODO: handle the case then protocol version if less then the current one. Then we should
        // make sure to only respond with the same version

        Ok(Socks5Request {
            protocol_version,
            content: Socks5RequestContent::try_from_bytes(&b[1..])?,
        })
    }
}

impl ServiceProviderRequest for Socks5Request {
    type ProtocolVersion = Socks5ProtocolVersion;
    type Response = Socks5Response;
    type Error = Socks5RequestError;

    fn provider_specific_version(&self) -> Self::ProtocolVersion {
        self.protocol_version
    }

    fn max_supported_version() -> Self::ProtocolVersion {
        Socks5ProtocolVersion::new_current()
    }
}

impl Socks5Request {
    // type tag that used to be prepended to all request messages
    const LEGACY_TYPE_TAG: u8 = 0x00;

    pub fn new(
        protocol_version: Socks5ProtocolVersion,
        content: Socks5RequestContent,
    ) -> Socks5Request {
        Socks5Request {
            protocol_version,
            content,
        }
    }

    pub fn new_connect(
        protocol_version: Socks5ProtocolVersion,
        conn_id: ConnectionId,
        remote_addr: RemoteAddress,
        return_address: Option<Recipient>,
    ) -> Socks5Request {
        Socks5Request {
            protocol_version,
            content: Socks5RequestContent::new_connect(conn_id, remote_addr, return_address),
        }
    }

    pub fn new_send(protocol_version: Socks5ProtocolVersion, data: SocketData) -> Socks5Request {
        Socks5Request {
            protocol_version,
            content: Socks5RequestContent::new_send(data),
        }
    }

    pub fn new_query(
        protocol_version: Socks5ProtocolVersion,
        query: QueryRequest,
    ) -> Socks5Request {
        Socks5Request {
            protocol_version,
            content: Socks5RequestContent::Query(query),
        }
    }
}

/// A request from a SOCKS5 client that a Nym Socks5 service provider should
/// take an action for an application using a (probably local) Nym Socks5 proxy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Socks5RequestContent {
    /// Start a new TCP connection to the specified `RemoteAddress` and send
    /// the request data up the connection.
    /// All responses produced on this `ConnectionId` should come back to the specified `Recipient`
    Connect(Box<ConnectRequest>),

    /// Re-use an existing TCP connection, sending more request data up it.
    Send(SendRequest),

    Query(QueryRequest),
}

impl Socks5RequestContent {
    /// Construct a new Request::Connect instance
    pub fn new_connect(
        conn_id: ConnectionId,
        remote_addr: RemoteAddress,
        return_address: Option<Recipient>,
    ) -> Socks5RequestContent {
        Socks5RequestContent::Connect(Box::new(ConnectRequest {
            conn_id,
            remote_addr,
            return_address,
        }))
    }

    /// Construct a new Request::Send instance
    pub fn new_send(data: SocketData) -> Socks5RequestContent {
        Socks5RequestContent::Send(SendRequest { data })
    }

    /// Deserialize the request type, connection id, destination address and port,
    /// and the request body from bytes.
    ///
    // TODO: this was already inaccurate
    // /// Serialized bytes looks like this:
    // ///
    // /// --------------------------------------------------------------------------------------
    // ///  request_flag | connection_id | address_length | remote_address_bytes | request_data |
    // ///        1      |       8       |      2         |    address_length    |    ...       |
    // /// --------------------------------------------------------------------------------------
    ///
    /// The request_flag tells us whether this is a new connection request (`new_connect`),
    /// an already-established connection we should send up (`new_send`), or
    /// a request to close an established connection (`new_close`).

    // connect:
    // RequestFlag::Connect || CONN_ID || ADDR_LEN || ADDR || <RETURN_ADDR>
    //
    // send:
    // RequestFlag::Send || CONN_ID || LOCAL_CLOSED || DATA
    // where DATA: SEQ || TRUE_DATA

    pub fn try_from_bytes(b: &[u8]) -> Result<Socks5RequestContent, RequestDeserializationError> {
        // each request needs to at least contain flag and ConnectionId
        if b.is_empty() {
            return Err(RequestDeserializationError::NoData);
        }

        match RequestFlag::try_from(b[0])? {
            RequestFlag::Connect => {
                if b.len() < 9 {
                    return Err(RequestDeserializationError::ConnectionIdTooShort);
                }
                let conn_id = u64::from_be_bytes([b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8]]);

                let connect_request_bytes = &b[9..];

                // we need to be able to read at least 2 bytes that specify address length
                if connect_request_bytes.len() < 2 {
                    return Err(RequestDeserializationError::AddressLengthTooShort);
                }

                let address_length =
                    u16::from_be_bytes([connect_request_bytes[0], connect_request_bytes[1]])
                        as usize;

                if connect_request_bytes.len() < 2 + address_length {
                    return Err(RequestDeserializationError::AddressTooShort);
                }

                let address_start = 2;
                let address_end = address_start + address_length;
                let address_bytes = &connect_request_bytes[address_start..address_end];
                let remote_address = String::from_utf8_lossy(address_bytes).to_string();

                // just a temporary reference to mid-slice for ease of use
                let recipient_data_bytes = &connect_request_bytes[address_end..];

                let return_address = if recipient_data_bytes.is_empty() {
                    None
                } else {
                    if recipient_data_bytes.len() != Recipient::LEN {
                        return Err(RequestDeserializationError::ReturnAddressTooShort);
                    }

                    let mut return_bytes = [0u8; Recipient::LEN];
                    return_bytes.copy_from_slice(&recipient_data_bytes[..Recipient::LEN]);
                    Some(
                        Recipient::try_from_bytes(return_bytes)
                            .map_err(RequestDeserializationError::MalformedReturnAddress)?,
                    )
                };

                Ok(Socks5RequestContent::new_connect(
                    conn_id,
                    remote_address,
                    return_address,
                ))
            }
            RequestFlag::Send => Ok(Socks5RequestContent::Send(SendRequest {
                data: SocketData::try_from_request_bytes(&b[1..])?,
            })),
            RequestFlag::Query => {
                use bincode::Options;
                let query = make_bincode_serializer().deserialize(&b[1..])?;
                Ok(Socks5RequestContent::Query(query))
            }
        }
    }

    /// Serialize a Socks5 request into bytes, so that it can be packed inside
    /// a Sphinx packet, sent through the mixnet, and deserialized by the Socks5
    /// service provider which will make the request.
    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            // connect is: CONN_FLAG || CONN_ID || REMOTE_LEN || REMOTE || RETURN
            Socks5RequestContent::Connect(req) => {
                let remote_address_bytes = req.remote_addr.into_bytes();
                let remote_address_bytes_len = remote_address_bytes.len() as u16;

                let iter = std::iter::once(RequestFlag::Connect as u8)
                    .chain(req.conn_id.to_be_bytes())
                    .chain(remote_address_bytes_len.to_be_bytes())
                    .chain(remote_address_bytes);

                if let Some(return_address) = req.return_address {
                    iter.chain(return_address.to_bytes()).collect()
                } else {
                    iter.collect()
                }
            }
            Socks5RequestContent::Send(req) => std::iter::once(RequestFlag::Send as u8)
                .chain(req.data.into_request_bytes_iter())
                .collect(),

            Socks5RequestContent::Query(query) => {
                use bincode::Options;
                let query_bytes: Vec<u8> = make_bincode_serializer()
                    .serialize(&query)
                    .tap_err(|err| {
                        log::error!("Failed to serialize query request: {:?}: {err}", query);
                    })
                    .unwrap_or_default();
                std::iter::once(RequestFlag::Query as u8)
                    .chain(query_bytes)
                    .collect()
            }
        }
    }
}

#[cfg(test)]
mod request_deserialization_tests {
    use super::*;

    mod all_request_types {
        use super::*;

        #[test]
        fn returns_error_when_zero_bytes() {
            let request_bytes = Vec::new();
            match Socks5RequestContent::try_from_bytes(&request_bytes).unwrap_err() {
                RequestDeserializationError::NoData => {}
                _ => unreachable!(),
            }
        }

        #[test]
        fn returns_error_when_connection_id_too_short() {
            let request_bytes = [RequestFlag::Connect as u8, 1, 2, 3, 4, 5, 6, 7].to_vec(); // 7 bytes connection id
            match Socks5RequestContent::try_from_bytes(&request_bytes).unwrap_err() {
                RequestDeserializationError::ConnectionIdTooShort => {}
                _ => unreachable!(),
            }
        }
    }

    #[cfg(test)]
    mod sending_data_over_a_new_connection {
        use super::*;

        #[test]
        fn returns_error_when_address_length_is_too_short() {
            let request_bytes1 = [RequestFlag::Connect as u8, 1, 2, 3, 4, 5, 6, 7, 8].to_vec(); // 8 bytes connection id, 0 bytes address length (2 were expected)
            let request_bytes2 = [RequestFlag::Connect as u8, 1, 2, 3, 4, 5, 6, 7, 8, 0].to_vec(); // 8 bytes connection id, 1 bytes address length (2 were expected)

            match Socks5RequestContent::try_from_bytes(&request_bytes1).unwrap_err() {
                RequestDeserializationError::AddressLengthTooShort => {}
                _ => unreachable!(),
            }

            match Socks5RequestContent::try_from_bytes(&request_bytes2).unwrap_err() {
                RequestDeserializationError::AddressLengthTooShort => {}
                _ => unreachable!(),
            }
        }

        #[test]
        fn returns_error_when_address_too_short_for_given_address_length() {
            let request_bytes = [RequestFlag::Connect as u8, 1, 2, 3, 4, 5, 6, 7, 8, 0, 1].to_vec(); // 8 bytes connection id, 2 bytes address length, missing address
            match Socks5RequestContent::try_from_bytes(&request_bytes).unwrap_err() {
                RequestDeserializationError::AddressTooShort => {}
                _ => unreachable!(),
            }
        }

        #[test]
        fn returns_error_for_when_return_address_is_too_short() {
            // this one has "foo.com" remote address and correct 8 bytes of connection_id
            let request_bytes_prefix = [
                RequestFlag::Connect as u8,
                1,
                2,
                3,
                4,
                5,
                6,
                7,
                8,
                0,
                7,
                102,
                111,
                111,
                46,
                99,
                111,
                109,
            ];

            let recipient = Recipient::try_from_base58_string("CytBseW6yFXUMzz4SGAKdNLGR7q3sJLLYxyBGvutNEQV.4QXYyEVc5fUDjmmi8PrHN9tdUFV4PCvSJE1278cHyvoe@4sBbL1ngf1vtNqykydQKTFh26sQCw888GpUqvPvyNB4f").unwrap();
            let recipient_bytes = recipient.to_bytes();

            // take only part of actual recipient
            let request_bytes: Vec<_> = request_bytes_prefix
                .iter()
                .cloned()
                .chain(recipient_bytes.iter().take(40).cloned())
                .collect();

            match Socks5RequestContent::try_from_bytes(&request_bytes).unwrap_err() {
                RequestDeserializationError::ReturnAddressTooShort => {}
                _ => unreachable!(),
            }
        }

        #[test]
        fn returns_error_for_when_return_address_is_malformed() {
            // this one has "foo.com" remote address and correct 8 bytes of connection_id
            let request_bytes_prefix = [
                RequestFlag::Connect as u8,
                1,
                2,
                3,
                4,
                5,
                6,
                7,
                8,
                0,
                7,
                102,
                111,
                111,
                46,
                99,
                111,
                109,
            ];

            let recipient = Recipient::try_from_base58_string("CytBseW6yFXUMzz4SGAKdNLGR7q3sJLLYxyBGvutNEQV.4QXYyEVc5fUDjmmi8PrHN9tdUFV4PCvSJE1278cHyvoe@4sBbL1ngf1vtNqykydQKTFh26sQCw888GpUqvPvyNB4f").unwrap();
            let mut recipient_bytes = recipient.to_bytes();

            // mess up few bytes
            recipient_bytes[0] = 255;
            recipient_bytes[15] ^= 1;
            recipient_bytes[31] ^= 1;

            let request_bytes: Vec<_> = request_bytes_prefix
                .iter()
                .cloned()
                .chain(recipient_bytes)
                .collect();
            assert!(Socks5RequestContent::try_from_bytes(&request_bytes)
                .unwrap_err()
                .is_malformed_return());
        }

        #[test]
        fn works_when_request_is_sized_properly_even_without_data() {
            // this one has "foo.com" remote address, correct 8 bytes of connection_id, and 0 bytes request data
            let request_bytes = [
                RequestFlag::Connect as u8,
                1,
                2,
                3,
                4,
                5,
                6,
                7,
                8,
                0,
                7,
                102,
                111,
                111,
                46,
                99,
                111,
                109,
            ]
            .to_vec();

            let recipient = Recipient::try_from_base58_string("CytBseW6yFXUMzz4SGAKdNLGR7q3sJLLYxyBGvutNEQV.4QXYyEVc5fUDjmmi8PrHN9tdUFV4PCvSJE1278cHyvoe@4sBbL1ngf1vtNqykydQKTFh26sQCw888GpUqvPvyNB4f").unwrap();
            let recipient_bytes = recipient.to_bytes();

            let request_bytes: Vec<_> = request_bytes.into_iter().chain(recipient_bytes).collect();

            let request = Socks5RequestContent::try_from_bytes(&request_bytes).unwrap();
            match request {
                Socks5RequestContent::Connect(req) => {
                    assert_eq!("foo.com".to_string(), req.remote_addr);
                    assert_eq!(u64::from_be_bytes([1, 2, 3, 4, 5, 6, 7, 8]), req.conn_id);
                    assert_eq!(
                        req.return_address.unwrap().to_bytes().to_vec(),
                        recipient.to_bytes().to_vec()
                    );
                }
                _ => unreachable!(),
            }
        }

        #[test]
        fn works_when_request_is_sized_properly_and_has_data() {
            // this one has a 1-byte remote address, correct 8 bytes of connection_id, and 3 bytes request data
            let request_bytes = [
                RequestFlag::Connect as u8,
                1,
                2,
                3,
                4,
                5,
                6,
                7,
                8,
                0,
                7,
                102,
                111,
                111,
                46,
                99,
                111,
                109,
            ]
            .to_vec();

            let recipient = Recipient::try_from_base58_string("CytBseW6yFXUMzz4SGAKdNLGR7q3sJLLYxyBGvutNEQV.4QXYyEVc5fUDjmmi8PrHN9tdUFV4PCvSJE1278cHyvoe@4sBbL1ngf1vtNqykydQKTFh26sQCw888GpUqvPvyNB4f").unwrap();
            let recipient_bytes = recipient.to_bytes();

            let request_bytes: Vec<_> = request_bytes.into_iter().chain(recipient_bytes).collect();

            let request = Socks5RequestContent::try_from_bytes(&request_bytes).unwrap();
            match request {
                Socks5RequestContent::Connect(req) => {
                    assert_eq!("foo.com".to_string(), req.remote_addr);
                    assert_eq!(u64::from_be_bytes([1, 2, 3, 4, 5, 6, 7, 8]), req.conn_id);
                    assert_eq!(
                        req.return_address.unwrap().to_bytes().to_vec(),
                        recipient.to_bytes().to_vec()
                    );
                }
                _ => unreachable!(),
            }
        }
    }

    #[cfg(test)]
    mod sending_additional_data_over_an_existing_connection {
        use super::*;

        #[test]
        fn works_when_request_is_sized_properly_even_without_data() {
            // correct 8 bytes of connection_id, 1 byte of local_closed, 8 bytes of sequence and 0 bytes request data
            let request_bytes = [
                RequestFlag::Send as u8,
                1,
                2,
                3,
                4,
                5,
                6,
                7,
                8,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                1,
            ]
            .to_vec();
            let request = Socks5RequestContent::try_from_bytes(&request_bytes).unwrap();
            match request {
                Socks5RequestContent::Send(SendRequest { data }) => {
                    assert_eq!(
                        u64::from_be_bytes([1, 2, 3, 4, 5, 6, 7, 8]),
                        data.header.connection_id
                    );
                    assert!(!data.header.local_socket_closed);
                    assert_eq!(1, data.header.seq);
                    assert!(data.data.is_empty());
                }
                _ => unreachable!(),
            }
        }

        #[test]
        fn works_when_request_is_sized_properly_and_has_data() {
            // correct 8 bytes of connection_id, 1 byte of local_closed, 8 bytes of sequence and 3 bytes request data (all 255)
            let request_bytes = [
                RequestFlag::Send as u8,
                1,
                2,
                3,
                4,
                5,
                6,
                7,
                8,
                1,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                1,
                255,
                255,
                255,
            ]
            .to_vec();

            let request = Socks5RequestContent::try_from_bytes(&request_bytes).unwrap();
            match request {
                Socks5RequestContent::Send(SendRequest { data }) => {
                    assert_eq!(
                        u64::from_be_bytes([1, 2, 3, 4, 5, 6, 7, 8]),
                        data.header.connection_id
                    );
                    assert!(data.header.local_socket_closed);
                    assert_eq!(1, data.header.seq);
                    assert_eq!(vec![255, 255, 255], data.data);
                }
                _ => unreachable!(),
            }
        }
    }

    #[cfg(test)]
    mod serialize_query_request {
        use super::*;

        #[test]
        fn serialize_there_and_back() {
            let open_proxy = Socks5RequestContent::Query(QueryRequest::OpenProxy);
            let bytes_open_proxy = open_proxy.clone().into_bytes();
            assert_eq!(bytes_open_proxy, vec![2, 0]);

            let description = Socks5RequestContent::Query(QueryRequest::Description);
            let bytes_description = description.clone().into_bytes();
            assert_eq!(bytes_description, vec![2, 1]);

            let open_proxy2 = Socks5RequestContent::try_from_bytes(&bytes_open_proxy).unwrap();
            let description2 = Socks5RequestContent::try_from_bytes(&bytes_description).unwrap();

            assert_eq!(open_proxy, open_proxy2);
            assert_eq!(description, description2);
        }
    }
}
