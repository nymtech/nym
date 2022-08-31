// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ConnectionId;

#[derive(Debug)]
pub struct NetworkRequesterResponse {
    pub connection_id: ConnectionId,
    pub network_requester_error: String,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no data provided")]
    NoData,

    #[error("not enough bytes to recover the connection id")]
    ConnectionIdTooShort,

    #[error("message is not utf8 encoded")]
    MalformedErrorMessage(#[from] std::string::FromUtf8Error),
}

impl NetworkRequesterResponse {
    pub fn new(connection_id: ConnectionId, network_requester_error: String) -> Self {
        NetworkRequesterResponse {
            connection_id,
            network_requester_error,
        }
    }

    pub fn try_from_bytes(b: &[u8]) -> Result<NetworkRequesterResponse, Error> {
        if b.is_empty() {
            return Err(Error::NoData);
        }

        if b.len() < 8 {
            return Err(Error::ConnectionIdTooShort);
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

        Ok(NetworkRequesterResponse {
            connection_id,
            network_requester_error,
        })
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.connection_id
            .to_be_bytes()
            .iter()
            .cloned()
            .chain(self.network_requester_error.into_bytes().into_iter())
            .collect()
    }
}
