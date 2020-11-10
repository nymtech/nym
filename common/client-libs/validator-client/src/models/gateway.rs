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
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::io;
use std::net::ToSocketAddrs;

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

// used for gateways to register themselves
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayRegistrationInfo {
    #[serde(flatten)]
    pub(crate) node_info: NodeInfo,
    pub(crate) clients_host: String,
}

impl GatewayRegistrationInfo {
    pub fn new(
        mix_host: String,
        clients_host: String,
        identity_key: String,
        sphinx_key: String,
        version: String,
        location: String,
        incentives_address: Option<String>,
    ) -> Self {
        GatewayRegistrationInfo {
            node_info: NodeInfo {
                mix_host,
                identity_key,
                sphinx_key,
                version,
                location,
                incentives_address: incentives_address.unwrap_or_else(|| "".to_string()),
            },
            clients_host,
        }
    }
}

// actual entry in topology
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisteredGateway {
    #[serde(flatten)]
    pub(crate) gateway_info: GatewayRegistrationInfo,
    pub(crate) registration_time: i64,
    pub(crate) reputation: i64,
}

impl RegisteredGateway {
    pub fn identity(&self) -> String {
        self.gateway_info.node_info.identity_key.clone()
    }

    pub fn mixnet_listener(&self) -> String {
        self.gateway_info.node_info.mix_host.clone()
    }

    pub fn clients_listener(&self) -> String {
        self.gateway_info.clients_host.clone()
    }
}

impl TryInto<topology::gateway::Node> for RegisteredGateway {
    type Error = ConversionError;

    fn try_into(self) -> Result<topology::gateway::Node, Self::Error> {
        let resolved_mix_hostname = self
            .gateway_info
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
            })?;

        Ok(topology::gateway::Node {
            location: self.gateway_info.node_info.location,
            mixnet_listener: resolved_mix_hostname,
            client_listener: self.gateway_info.clients_host,
            identity_key: identity::PublicKey::from_base58_string(
                self.gateway_info.node_info.identity_key,
            )?,
            sphinx_key: encryption::PublicKey::from_base58_string(
                self.gateway_info.node_info.sphinx_key,
            )?,
            registration_time: self.registration_time,
            reputation: self.reputation,
            version: self.gateway_info.node_info.version,
        })
    }
}
