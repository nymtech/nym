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

use crate::models::node::NodeInfo;
use crypto::asymmetric::{encryption, identity};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::io;
use std::net::{SocketAddr, ToSocketAddrs};
use topology::mix::Node;

#[derive(Debug)]
pub enum ConversionError {
    InvalidIdentityKeyError(identity::KeyRecoveryError),
    InvalidSphinxKeyError(encryption::KeyRecoveryError),
    InvalidAddress(io::Error),
}

impl From<encryption::KeyRecoveryError> for ConversionError {
    fn from(err: encryption::KeyRecoveryError) -> Self {
        ConversionError::InvalidSphinxKeyError(err)
    }
}

impl From<identity::KeyRecoveryError> for ConversionError {
    fn from(err: identity::KeyRecoveryError) -> Self {
        ConversionError::InvalidIdentityKeyError(err)
    }
}

// used for mixnode to register themselves
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MixRegistrationInfo {
    #[serde(flatten)]
    pub(crate) node_info: NodeInfo,
    pub(crate) layer: u64,
}

impl MixRegistrationInfo {
    pub fn new(
        mix_host: String,
        identity_key: String,
        sphinx_key: String,
        version: String,
        location: String,
        layer: u64,
        incentives_address: Option<String>,
    ) -> Self {
        MixRegistrationInfo {
            node_info: NodeInfo {
                mix_host,
                identity_key,
                sphinx_key,
                version,
                location,
                incentives_address: incentives_address.unwrap_or_else(|| "".to_string()),
            },
            layer,
        }
    }
}

// actual entry in topology
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegisteredMix {
    #[serde(flatten)]
    pub(crate) mix_info: MixRegistrationInfo,
    pub(crate) registration_time: i64,
    pub(crate) reputation: i64,
}

impl RegisteredMix {
    pub fn identity(&self) -> String {
        self.mix_info.node_info.identity_key.clone()
    }

    pub fn mix_host(&self) -> String {
        self.mix_info.node_info.mix_host.clone()
    }

    pub fn reputation(&self) -> i64 {
        self.reputation
    }

    pub fn layer(&self) -> u64 {
        self.mix_info.layer
    }

    pub fn version(&self) -> String {
        self.mix_info.node_info.version.clone()
    }

    pub fn version_ref(&self) -> &str {
        &self.mix_info.node_info.version
    }

    pub fn incentives_address(&self) -> String {
        self.mix_info.node_info.incentives_address.clone()
    }

    fn resolve_hostname(&self) -> Result<SocketAddr, ConversionError> {
        self.mix_info
            .node_info
            .mix_host
            .to_socket_addrs()
            .map_err(ConversionError::InvalidAddress)?
            .next()
            .ok_or_else(|| {
                ConversionError::InvalidAddress(io::Error::new(
                    io::ErrorKind::Other,
                    "no valid socket address",
                ))
            })
    }
}

impl TryInto<topology::mix::Node> for RegisteredMix {
    type Error = ConversionError;

    fn try_into(self) -> Result<topology::mix::Node, Self::Error> {
        Ok(topology::mix::Node {
            host: self.resolve_hostname()?,
            location: self.mix_info.node_info.location,
            identity_key: identity::PublicKey::from_base58_string(
                self.mix_info.node_info.identity_key,
            )?,
            sphinx_key: encryption::PublicKey::from_base58_string(
                self.mix_info.node_info.sphinx_key,
            )?,
            layer: self.mix_info.layer,
            registration_time: self.registration_time,
            reputation: self.reputation,
            version: self.mix_info.node_info.version,
        })
    }
}

impl<'a> TryInto<topology::mix::Node> for &'a RegisteredMix {
    type Error = ConversionError;

    fn try_into(self) -> Result<Node, Self::Error> {
        Ok(topology::mix::Node {
            host: self.resolve_hostname()?,
            location: self.mix_info.node_info.location.clone(),
            identity_key: identity::PublicKey::from_base58_string(
                &self.mix_info.node_info.identity_key,
            )?,
            sphinx_key: encryption::PublicKey::from_base58_string(
                &self.mix_info.node_info.sphinx_key,
            )?,
            layer: self.mix_info.layer,
            registration_time: self.registration_time,
            reputation: self.reputation,
            version: self.mix_info.node_info.version.clone(),
        })
    }
}
