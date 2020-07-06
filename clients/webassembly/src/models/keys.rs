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
use std::convert::{TryFrom, TryInto};
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct GatewayIdentity {
    private_key: String,
    public_key: String,
    address: String,
}

impl TryFrom<String> for GatewayIdentity {
    type Error = serde_json::Error;

    fn try_from(msg: String) -> Result<Self, Self::Error> {
        serde_json::from_str(&msg)
    }
}

impl TryInto<String> for GatewayIdentity {
    type Error = serde_json::Error;

    fn try_into(self) -> Result<String, Self::Error> {
        serde_json::to_string(&self)
    }
}

#[wasm_bindgen]
pub fn keygen() -> String {
    let keypair = identity::KeyPair::new();
    let address = keypair.public_key().derive_destination_address();

    GatewayIdentity {
        private_key: keypair.private_key().to_base58_string(),
        public_key: keypair.public_key().to_base58_string(),
        address: address.to_base58_string(),
    }
    .try_into()
    .unwrap()
}
