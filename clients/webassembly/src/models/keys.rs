use crypto::identity::MixIdentityKeyPair;
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct FullJSONKeypair {
    private_key: String,
    public_key: String,
    address: String,
}

impl TryFrom<String> for FullJSONKeypair {
    type Error = serde_json::Error;

    fn try_from(msg: String) -> Result<Self, Self::Error> {
        serde_json::from_str(&msg)
    }
}

impl TryInto<String> for FullJSONKeypair {
    type Error = serde_json::Error;

    fn try_into(self) -> Result<String, Self::Error> {
        serde_json::to_string(&self)
    }
}

#[wasm_bindgen]
pub fn keygen() -> String {
    let keypair = MixIdentityKeyPair::new();
    let address = keypair.public_key().derive_address();

    FullJSONKeypair {
        private_key: keypair.private_key().to_base58_string(),
        public_key: keypair.public_key().to_base58_string(),
        address: address.to_base58_string(),
    }
    .try_into()
    .unwrap()
}
