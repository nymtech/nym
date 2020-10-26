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

#[derive(Debug)]
pub enum ConversionError {
    InvalidIdentityKeyError(identity::KeyRecoveryError),
    InvnalidSphinxKeyError(encryption::KeyRecoveryError),
    // InvalidAddress(io::Error),
}

impl From<encryption::KeyRecoveryError> for ConversionError {
    fn from(err: encryption::KeyRecoveryError) -> Self {
        ConversionError::InvnalidSphinxKeyError(err)
    }
}

impl From<identity::KeyRecoveryError> for ConversionError {
    fn from(err: identity::KeyRecoveryError) -> Self {
        ConversionError::InvalidIdentityKeyError(err)
    }
}

// impl From<io::Error> for ConversionError {
//     fn from(err: io::Error) -> Self {
//         ConversionError::InvalidAddress(err)
//     }
// }

// used for mixnode to register themselves
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayRegistrationInfo {
    #[serde(flatten)]
    node_info: NodeInfo,
    clients_host: String,
}

// actual entry in topology
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisteredGateway {
    #[serde(flatten)]
    mix_info: GatewayRegistrationInfo,
    registration_time: i64,
    reputation: i64,
}

impl TryInto<topology::gateway::Node> for RegisteredGateway {
    type Error = ConversionError;

    fn try_into(self) -> Result<topology::gateway::Node, Self::Error> {
        unimplemented!()
    }
}
