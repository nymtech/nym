// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    make_bincode_serializer, ConnectionId, InsufficientSocketDataError, SocketData,
    Socks5ProtocolVersion, Socks5RequestError,
};
use nym_exit_policy::ExitPolicy;
use nym_service_providers_common::interface::{Serializable, ServiceProviderResponse};
use serde::{Deserialize, Serialize};
use tap::TapFallible;
use thiserror::Error;

// don't start tags from 0 for easier backwards compatibility since `NetworkData`
// used to be a `Response` with tag 1
// and `ConnectionError` used to be `NetworkRequesterResponse` with tag 2
#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum ResponseFlag {
    NetworkData = 1,
    ConnectionError = 2,
    Query = 3,
}

impl TryFrom<u8> for ResponseFlag {
    type Error = ResponseDeserializationError;

    fn try_from(value: u8) -> Result<ResponseFlag, ResponseDeserializationError> {
        match value {
            _ if value == (ResponseFlag::NetworkData as u8) => Ok(Self::NetworkData),
            _ if value == (ResponseFlag::ConnectionError as u8) => Ok(Self::ConnectionError),
            _ if value == (ResponseFlag::Query as u8) => Ok(Self::Query),
            value => Err(ResponseDeserializationError::UnknownResponseFlag { value }),
        }
    }
}

#[derive(Debug, Error)]
pub enum ResponseDeserializationError {
    #[error("the network data was malformed: {source}")]
    MalformedNetworkData {
        #[from]
        source: InsufficientSocketDataError,
    },

    #[error("not enough bytes to recover the connection id")]
    ConnectionIdTooShort,

    #[error("{value} is not a valid response flag")]
    UnknownResponseFlag { value: u8 },

    #[error("no data provided")]
    NoData,

    #[error("message is not utf8 encoded: {source}")]
    MalformedErrorMessage {
        #[from]
        source: std::string::FromUtf8Error,
    },

    #[error("failed to deserialize query response: {source}")]
    QueryDeserializationError {
        #[from]
        source: bincode::Error,
    },
}

#[derive(Debug, Clone)]
pub struct Socks5Response {
    pub protocol_version: Socks5ProtocolVersion,
    pub content: Socks5ResponseContent,
}

impl Serializable for Socks5Response {
    type Error = Socks5RequestError;

    // legacy responses had the format of
    // 1 (Message::RESPONSE_FLAG) || <data> for data responses
    // 2 (Message::NR_RESPONSE_FLAG) || <data> for error responses
    // the updated formats use
    // 3 (Socks5ProtocolVersion) || 0 (ResponseFlag::NetworkData) || <data> for data responses
    // 3 (Socks5ProtocolVersion) || 1 (ResponseFlag::ConnectionError) || <data> for error responses
    // so for serialization an optional version tag is prepended
    // and in deserialization it's just the case of shifting the buffer in case of non-legacy response payload
    fn into_bytes(self) -> Vec<u8> {
        if let Some(version) = self.protocol_version.as_u8() {
            std::iter::once(version)
                .chain(self.content.into_bytes())
                .collect()
        } else {
            self.content.into_bytes()
        }
    }

    fn try_from_bytes(b: &[u8]) -> Result<Self, Self::Error> {
        if b.is_empty() {
            return Err(ResponseDeserializationError::NoData.into());
        }

        let protocol_version = Socks5ProtocolVersion::from(b[0]);
        let content = if protocol_version.is_legacy() {
            Socks5ResponseContent::try_from_bytes(b)
        } else {
            Socks5ResponseContent::try_from_bytes(&b[1..])
        }?;
        Ok(Socks5Response {
            protocol_version,
            content,
        })
    }
}

impl ServiceProviderResponse for Socks5Response {}

impl Socks5Response {
    pub fn new(
        protocol_version: Socks5ProtocolVersion,
        content: Socks5ResponseContent,
    ) -> Socks5Response {
        Socks5Response {
            protocol_version,
            content,
        }
    }

    pub fn new_network_data(
        protocol_version: Socks5ProtocolVersion,
        seq: u64,
        connection_id: ConnectionId,
        data: Vec<u8>,
        is_closed: bool,
    ) -> Socks5Response {
        Socks5Response {
            protocol_version,
            content: Socks5ResponseContent::new_network_data(seq, connection_id, data, is_closed),
        }
    }

    pub fn new_connection_error(
        protocol_version: Socks5ProtocolVersion,
        connection_id: ConnectionId,
        error_message: String,
    ) -> Socks5Response {
        Socks5Response {
            protocol_version,
            content: Socks5ResponseContent::new_connection_error(connection_id, error_message),
        }
    }

    pub fn new_query(
        protocol_version: Socks5ProtocolVersion,
        query_response: QueryResponse,
    ) -> Socks5Response {
        Socks5Response {
            protocol_version,
            content: Socks5ResponseContent::Query(query_response),
        }
    }

    pub fn new_query_error<S: Into<String>>(
        protocol_version: Socks5ProtocolVersion,
        message: S,
    ) -> Socks5Response {
        Socks5Response {
            protocol_version,
            content: Socks5ResponseContent::Query(QueryResponse::Error {
                message: message.into(),
            }),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Socks5ResponseContent {
    NetworkData { content: SocketData },
    ConnectionError(ConnectionError),
    Query(QueryResponse),
}

impl Socks5ResponseContent {
    pub fn new_network_data(
        seq: u64,
        connection_id: ConnectionId,
        data: Vec<u8>,
        is_closed: bool,
    ) -> Socks5ResponseContent {
        Socks5ResponseContent::NetworkData {
            content: SocketData::new(seq, connection_id, is_closed, data),
        }
    }

    pub fn new_connection_error(
        connection_id: ConnectionId,
        error_message: String,
    ) -> Socks5ResponseContent {
        Socks5ResponseContent::ConnectionError(ConnectionError::new(connection_id, error_message))
    }

    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            Socks5ResponseContent::NetworkData { content } => {
                std::iter::once(ResponseFlag::NetworkData as u8)
                    .chain(content.into_response_bytes_iter())
                    .collect()
            }
            Socks5ResponseContent::ConnectionError(res) => {
                std::iter::once(ResponseFlag::ConnectionError as u8)
                    .chain(res.into_bytes())
                    .collect()
            }
            Socks5ResponseContent::Query(query) => {
                use bincode::Options;
                let query_bytes: Vec<u8> = make_bincode_serializer()
                    .serialize(&query)
                    .tap_err(|err| {
                        log::error!("Failed to serialize query response: {:?}: {err}", query);
                    })
                    .unwrap_or_default();
                std::iter::once(ResponseFlag::Query as u8)
                    .chain(query_bytes)
                    .collect()
            }
        }
    }

    pub fn try_from_bytes(b: &[u8]) -> Result<Socks5ResponseContent, ResponseDeserializationError> {
        if b.is_empty() {
            // TODO: bad error type since this branch could be reached in the 'versioned' case
            // after reading 1 byte already
            return Err(ResponseDeserializationError::NoData);
        }

        let response_flag = ResponseFlag::try_from(b[0])?;
        match response_flag {
            ResponseFlag::NetworkData => Ok(Socks5ResponseContent::NetworkData {
                content: SocketData::try_from_response_bytes(&b[1..])?,
            }),
            ResponseFlag::ConnectionError => Ok(Socks5ResponseContent::ConnectionError(
                ConnectionError::try_from_bytes(&b[1..])?,
            )),
            ResponseFlag::Query => {
                use bincode::Options;
                let query = make_bincode_serializer().deserialize(&b[1..])?;
                Ok(Socks5ResponseContent::Query(query))
            }
        }
    }

    pub fn as_query(&self) -> Option<&QueryResponse> {
        match self {
            Socks5ResponseContent::Query(query) => Some(query),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectionError {
    pub connection_id: ConnectionId,
    pub network_requester_error: String,
}

impl ConnectionError {
    pub fn new(connection_id: ConnectionId, network_requester_error: String) -> Self {
        ConnectionError {
            connection_id,
            network_requester_error,
        }
    }

    pub fn try_from_bytes(b: &[u8]) -> Result<ConnectionError, ResponseDeserializationError> {
        if b.is_empty() {
            return Err(ResponseDeserializationError::NoData);
        }

        if b.len() < 8 {
            return Err(ResponseDeserializationError::ConnectionIdTooShort);
        }

        let mut connection_id_bytes = b.to_vec();
        let network_requester_error_bytes = connection_id_bytes.split_off(8);

        let connection_id = u64::from_be_bytes([
            connection_id_bytes[0],
            connection_id_bytes[1],
            connection_id_bytes[2],
            connection_id_bytes[3],
            connection_id_bytes[4],
            connection_id_bytes[5],
            connection_id_bytes[6],
            connection_id_bytes[7],
        ]);
        let network_requester_error = String::from_utf8(network_requester_error_bytes)?;

        Ok(ConnectionError {
            connection_id,
            network_requester_error,
        })
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.connection_id
            .to_be_bytes()
            .iter()
            .copied()
            .chain(self.network_requester_error.into_bytes())
            .collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum QueryResponse {
    OpenProxy(bool),
    Description(String),
    ExitPolicy {
        enabled: bool,
        upstream: String,
        policy: Option<ExitPolicy>,
    },
    Error {
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod connection_error_response_serde_tests {
        use super::*;

        #[test]
        fn simple_serde() {
            let conn_id = 42;
            let network_requester_error = String::from("This is a test msg");
            let response = ConnectionError::new(conn_id, network_requester_error.clone());
            let bytes = response.into_bytes();
            let deserialized_response = ConnectionError::try_from_bytes(&bytes).unwrap();

            assert_eq!(conn_id, deserialized_response.connection_id);
            assert_eq!(
                network_requester_error,
                deserialized_response.network_requester_error
            );
        }

        #[test]
        fn deserialization_errors() {
            let err = ConnectionError::try_from_bytes(&[]).err().unwrap();
            assert!(matches!(err, ResponseDeserializationError::NoData));

            let bytes: [u8; 5] = [1, 2, 3, 4, 5];
            let err = ConnectionError::try_from_bytes(&bytes).err().unwrap();
            assert!(matches!(
                err,
                ResponseDeserializationError::ConnectionIdTooShort
            ));

            let bytes: Vec<u8> = 42u64
                .to_be_bytes()
                .into_iter()
                .chain([0, 159, 146, 150])
                .collect();
            let err = ConnectionError::try_from_bytes(&bytes).err().unwrap();
            assert!(matches!(
                err,
                ResponseDeserializationError::MalformedErrorMessage { .. }
            ));
        }
    }

    #[cfg(test)]
    mod serialize_query_response {
        use super::*;

        #[test]
        fn serialize_there_and_back() {
            let open_proxy = Socks5ResponseContent::Query(QueryResponse::OpenProxy(true));
            let bytes_open_proxy = open_proxy.clone().into_bytes();
            assert_eq!(bytes_open_proxy, vec![3, 0, 1]);

            let description =
                Socks5ResponseContent::Query(QueryResponse::Description("foo".to_string()));
            let bytes_description = description.clone().into_bytes();
            assert_eq!(bytes_description, vec![3, 1, 3, 102, 111, 111]);

            let error = Socks5ResponseContent::Query(QueryResponse::Error {
                message: "this is an error".to_string(),
            });
            let bytes_error = error.clone().into_bytes();
            assert_eq!(
                bytes_error,
                vec![
                    3, 3, 16, 116, 104, 105, 115, 32, 105, 115, 32, 97, 110, 32, 101, 114, 114,
                    111, 114
                ]
            );

            let exit_policy = Socks5ResponseContent::Query(QueryResponse::ExitPolicy {
                enabled: false,
                upstream: "http://foo.bar".to_string(),
                policy: Some(ExitPolicy::new_open()),
            });
            let bytes_exit_policy = exit_policy.clone().into_bytes();
            assert_eq!(
                bytes_exit_policy,
                vec![
                    3, 2, 0, 14, 104, 116, 116, 112, 58, 47, 47, 102, 111, 111, 46, 98, 97, 114, 1,
                    1, 0, 1, 42, 1, 251, 255, 255
                ]
            );

            let open_proxy2 = Socks5ResponseContent::try_from_bytes(&bytes_open_proxy).unwrap();
            let description2 = Socks5ResponseContent::try_from_bytes(&bytes_description).unwrap();
            let error2 = Socks5ResponseContent::try_from_bytes(&bytes_error).unwrap();
            let exit_policy2 = Socks5ResponseContent::try_from_bytes(&bytes_exit_policy).unwrap();

            assert_eq!(open_proxy, open_proxy2);
            assert_eq!(description, description2);
            assert_eq!(error, error2);
            assert_eq!(exit_policy, exit_policy2);
        }
    }
}
