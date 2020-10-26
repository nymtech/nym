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

use crypto::asymmetric::identity;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;

#[derive(Debug)]
pub enum ConversionError {
    InvalidKeyError,
}

impl From<identity::KeyRecoveryError> for ConversionError {
    fn from(_: identity::KeyRecoveryError) -> Self {
        ConversionError::InvalidKeyError
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CocoPresence {
    pub location: String,
    pub host: String,
    pub pub_key: String,
    pub last_seen: u64,
    pub version: String,
}

impl TryInto<topology::coco::Node> for CocoPresence {
    type Error = ConversionError;

    fn try_into(self) -> Result<topology::coco::Node, Self::Error> {
        Ok(topology::coco::Node {
            location: self.location,
            host: self.host,
            pub_key: identity::PublicKey::from_base58_string(self.pub_key)?,
            last_seen: self.last_seen,
            version: self.version,
        })
    }
}
