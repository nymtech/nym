// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{ConnectionId, Socks5ProtocolVersion, Socks5RequestError};
use nym_service_providers_common::interface::{Serializable, ServiceProviderResponse};
use thiserror::Error;

// don't start tags from 0 for easier backwards compatibility since `NetworkData`
// used to be a `Response` with tag 1
// and `ConnectionError` used to be `NetworkRequesterResponse` with tag 2
#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum ResponseFlag {
    NetworkData = 1,
    ConnectionError = 2,
}

impl TryFrom<u8> for ResponseFlag {
    type Error = ResponseDeserializationError;

    fn try_from(value: u8) -> Result<ResponseFlag, ResponseDeserializationError> {
        match value {
            _ if value == (ResponseFlag::NetworkData as u8) => Ok(Self::NetworkData),
            _ if value == (ResponseFlag::ConnectionError as u8) => Ok(Self::ConnectionError),
            value => Err(ResponseDeserializationError::UnknownResponseFlag { value }),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ResponseDeserializationError {
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
}

#[derive(Debug)]
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
                .chain(self.content.into_bytes().into_iter())
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
        connection_id: ConnectionId,
        data: Vec<u8>,
        is_closed: bool,
    ) -> Socks5Response {
        Socks5Response {
            protocol_version,
            content: Socks5ResponseContent::new_network_data(connection_id, data, is_closed),
        }
    }

    pub fn new_closed_empty(
        protocol_version: Socks5ProtocolVersion,
        connection_id: ConnectionId,
    ) -> Socks5Response {
        Socks5Response {
            protocol_version,
            content: Socks5ResponseContent::new_closed_empty(connection_id),
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
}

#[derive(Debug)]
pub enum Socks5ResponseContent {
    NetworkData(NetworkData),
    ConnectionError(ConnectionError),
}

impl Socks5ResponseContent {
    pub fn new_network_data(
        connection_id: ConnectionId,
        data: Vec<u8>,
        is_closed: bool,
    ) -> Socks5ResponseContent {
        Socks5ResponseContent::NetworkData(NetworkData::new(connection_id, data, is_closed))
    }

    pub fn new_closed_empty(connection_id: ConnectionId) -> Socks5ResponseContent {
        Socks5ResponseContent::NetworkData(NetworkData::new_closed_empty(connection_id))
    }

    pub fn new_connection_error(
        connection_id: ConnectionId,
        error_message: String,
    ) -> Socks5ResponseContent {
        Socks5ResponseContent::ConnectionError(ConnectionError::new(connection_id, error_message))
    }

    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            Socks5ResponseContent::NetworkData(res) => {
                std::iter::once(ResponseFlag::NetworkData as u8)
                    .chain(res.into_bytes().into_iter())
                    .collect()
            }
            Socks5ResponseContent::ConnectionError(res) => {
                std::iter::once(ResponseFlag::ConnectionError as u8)
                    .chain(res.into_bytes().into_iter())
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
            ResponseFlag::NetworkData => Ok(Socks5ResponseContent::NetworkData(
                NetworkData::try_from_bytes(&b[1..])?,
            )),
            ResponseFlag::ConnectionError => Ok(Socks5ResponseContent::ConnectionError(
                ConnectionError::try_from_bytes(&b[1..])?,
            )),
        }
    }
}

/// A remote network network data response retrieved by the Socks5 service provider. This
/// can be serialized and sent back through the mixnet to the requesting
/// application.
#[derive(Debug)]
pub struct NetworkData {
    pub data: Vec<u8>,
    pub connection_id: ConnectionId,
    pub is_closed: bool,
}

impl NetworkData {
    /// Constructor for responses
    pub fn new(connection_id: ConnectionId, data: Vec<u8>, is_closed: bool) -> Self {
        NetworkData {
            data,
            connection_id,
            is_closed,
        }
    }

    pub fn new_closed_empty(connection_id: ConnectionId) -> Self {
        NetworkData {
            data: vec![],
            connection_id,
            is_closed: false,
        }
    }

    pub fn try_from_bytes(b: &[u8]) -> Result<NetworkData, ResponseDeserializationError> {
        if b.is_empty() {
            return Err(ResponseDeserializationError::NoData);
        }

        let is_closed = b[0] != 0;

        if b.len() < 9 {
            return Err(ResponseDeserializationError::ConnectionIdTooShort);
        }

        let mut connection_id_bytes = b.to_vec();
        let data = connection_id_bytes.split_off(9);

        let connection_id = u64::from_be_bytes([
            connection_id_bytes[1],
            connection_id_bytes[2],
            connection_id_bytes[3],
            connection_id_bytes[4],
            connection_id_bytes[5],
            connection_id_bytes[6],
            connection_id_bytes[7],
            connection_id_bytes[8],
        ]);

        let response = NetworkData::new(connection_id, data, is_closed);
        Ok(response)
    }

    /// Serializes the response into bytes so that it can be sent back through
    /// the mixnet to the requesting application.
    pub fn into_bytes(self) -> Vec<u8> {
        std::iter::once(self.is_closed as u8)
            .chain(self.connection_id.to_be_bytes().iter().cloned())
            .chain(self.data.into_iter())
            .collect()
    }
}

#[derive(Debug)]
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
            .chain(self.network_requester_error.into_bytes().into_iter())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod constructing_socks5_data_responses_from_bytes {
        use super::*;

        #[test]
        fn fails_when_zero_bytes_are_supplied() {
            let response_bytes = Vec::new();

            assert_eq!(
                ResponseDeserializationError::NoData,
                NetworkData::try_from_bytes(&response_bytes).unwrap_err()
            );
        }

        #[test]
        fn fails_when_connection_id_bytes_are_too_short() {
            let response_bytes = vec![0, 1, 2, 3, 4, 5, 6];
            assert_eq!(
                ResponseDeserializationError::ConnectionIdTooShort,
                NetworkData::try_from_bytes(&response_bytes).unwrap_err()
            );
        }

        #[test]
        fn works_when_there_is_no_data() {
            let response_bytes = vec![0, 0, 1, 2, 3, 4, 5, 6, 7];
            let expected = NetworkData::new(
                u64::from_be_bytes([0, 1, 2, 3, 4, 5, 6, 7]),
                Vec::new(),
                false,
            );
            let actual = NetworkData::try_from_bytes(&response_bytes).unwrap();
            assert_eq!(expected.connection_id, actual.connection_id);
            assert_eq!(expected.data, actual.data);
            assert_eq!(expected.is_closed, actual.is_closed);
        }

        #[test]
        fn works_when_there_is_data() {
            let response_bytes = vec![0, 0, 1, 2, 3, 4, 5, 6, 7, 255, 255, 255];
            let expected = NetworkData::new(
                u64::from_be_bytes([0, 1, 2, 3, 4, 5, 6, 7]),
                vec![255, 255, 255],
                false,
            );
            let actual = NetworkData::try_from_bytes(&response_bytes).unwrap();
            assert_eq!(expected.connection_id, actual.connection_id);
            assert_eq!(expected.data, actual.data);
            assert_eq!(expected.is_closed, actual.is_closed);
        }
    }

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
            assert_eq!(err, ResponseDeserializationError::NoData);

            let bytes: [u8; 5] = [1, 2, 3, 4, 5];
            let err = ConnectionError::try_from_bytes(&bytes).err().unwrap();
            assert_eq!(err, ResponseDeserializationError::ConnectionIdTooShort);

            let bytes: Vec<u8> = 42u64
                .to_be_bytes()
                .into_iter()
                .chain([0, 159, 146, 150].into_iter())
                .collect();
            let err = ConnectionError::try_from_bytes(&bytes).err().unwrap();
            assert!(matches!(
                err,
                ResponseDeserializationError::MalformedErrorMessage { .. }
            ));
        }
    }
}
