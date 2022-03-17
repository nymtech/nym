// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use coconut_interface::{Attribute, BlindSignRequest};
use serde::{Deserialize, Serialize};

use crypto::asymmetric::{encryption, identity};
use pemstore::traits::PemStorableKeyPair;

use crate::error::{CredentialClientError, Result};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct KeyPair {
    pub public_key: String,
    pub private_key: String,
}

impl From<identity::KeyPair> for KeyPair {
    fn from(kp: identity::KeyPair) -> Self {
        Self {
            public_key: kp.public_key().to_base58_string(),
            private_key: kp.private_key().to_base58_string(),
        }
    }
}

impl From<encryption::KeyPair> for KeyPair {
    fn from(kp: encryption::KeyPair) -> Self {
        Self {
            public_key: kp.public_key().to_base58_string(),
            private_key: kp.private_key().to_base58_string(),
        }
    }
}

impl Into<identity::KeyPair> for KeyPair {
    fn into(self) -> identity::KeyPair {
        identity::KeyPair::from_keys(
            identity::PrivateKey::from_base58_string(self.private_key).unwrap(),
            identity::PublicKey::from_base58_string(self.public_key).unwrap(),
        )
    }
}

impl Into<encryption::KeyPair> for KeyPair {
    fn into(self) -> encryption::KeyPair {
        encryption::KeyPair::from_keys(
            encryption::PrivateKey::from_base58_string(self.private_key).unwrap(),
            encryption::PublicKey::from_base58_string(self.public_key).unwrap(),
        )
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct State {
    pub amount: u64,
    pub tx_hash: String,
    pub signing_keypair: KeyPair,
    pub encryption_keypair: KeyPair,
    pub blind_request_data: Option<RequestData>,
    pub signature: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct RequestData {
    first_attribute: [u8; 32],
    second_attribute: [u8; 32],
    blind_sign_req: Vec<u8>,
}

impl RequestData {
    pub fn new(attributes: &[Attribute], blind_sign_request: &BlindSignRequest) -> Result<Self> {
        if attributes.len() != 2 {
            Err(CredentialClientError::WrongAttributeNumber)
        } else {
            Ok(RequestData {
                first_attribute: attributes[0].to_bytes(),
                second_attribute: attributes[1].to_bytes(),
                blind_sign_req: blind_sign_request.to_bytes(),
            })
        }
    }
}
