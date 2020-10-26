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
pub struct MixRegistrationInfo {
    #[serde(flatten)]
    node_info: NodeInfo,
    layer: usize,
}

impl MixRegistrationInfo {
    pub fn new(
        mix_host: String,
        identity_key: String,
        sphinx_key: String,
        version: String,
        location: String,
        layer: usize,
    ) -> Self {
        MixRegistrationInfo {
            node_info: NodeInfo {
                mix_host,
                identity_key,
                sphinx_key,
                version,
                location,
            },
            layer,
        }
    }
}

// actual entry in topology
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisteredMix {
    #[serde(flatten)]
    mix_info: MixRegistrationInfo,
    registration_time: i64,
    reputation: i64,
}

impl TryInto<topology::mix::Node> for RegisteredMix {
    type Error = ConversionError;

    fn try_into(self) -> Result<topology::mix::Node, Self::Error> {
        unimplemented!()
    }
}
