// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crypto::asymmetric::{encryption, identity};
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::io;
use std::net::ToSocketAddrs;

#[derive(Debug)]
pub enum ConversionError {
    InvalidKeyError,
    InvalidAddress(io::Error),
}

impl From<identity::SignatureError> for ConversionError {
    fn from(_: identity::SignatureError) -> Self {
        ConversionError::InvalidKeyError
    }
}

impl From<encryption::EncryptionKeyError> for ConversionError {
    fn from(_: encryption::EncryptionKeyError) -> Self {
        ConversionError::InvalidKeyError
    }
}

impl From<io::Error> for ConversionError {
    fn from(err: io::Error) -> Self {
        ConversionError::InvalidAddress(err)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayPresence {
    pub location: String,
    pub client_listener: String,
    pub mixnet_listener: String,
    pub identity_key: String,
    pub sphinx_key: String,
    pub last_seen: u64,
    pub version: String,
}

impl TryInto<topology::gateway::Node> for GatewayPresence {
    type Error = ConversionError;

    fn try_into(self) -> Result<topology::gateway::Node, Self::Error> {
        let resolved_mix_hostname = self.mixnet_listener.to_socket_addrs()?.next();
        if resolved_mix_hostname.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "no valid socket address",
            ))?;
        }

        Ok(topology::gateway::Node {
            location: self.location,
            client_listener: self.client_listener,
            mixnet_listener: resolved_mix_hostname.unwrap(),
            identity_key: identity::PublicKey::from_base58_string(self.identity_key)?,
            sphinx_key: encryption::PublicKey::from_base58_string(self.sphinx_key)?,
            last_seen: self.last_seen,
            version: self.version,
        })
    }
}
