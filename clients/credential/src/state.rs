// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use coconut_interface::{Attribute, BlindSignRequest, Bytable, PrivateAttribute};
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
    pub serial_number: Vec<u8>,
    pub binding_number: Vec<u8>,
    pub first_attribute: Vec<u8>,
    pub second_attribute: Vec<u8>,
    pub blind_sign_req: Vec<u8>,
}

impl RequestData {
    pub fn new(
        private_attributes: Vec<PrivateAttribute>,
        attributes: &[Attribute],
        blind_sign_request: &BlindSignRequest,
    ) -> Result<Self> {
        if private_attributes.len() != 2 || attributes.len() != 2 {
            Err(CredentialClientError::WrongAttributeNumber)
        } else {
            Ok(RequestData {
                serial_number: private_attributes[0].to_byte_vec(),
                binding_number: private_attributes[1].to_byte_vec(),
                first_attribute: attributes[0].to_byte_vec(),
                second_attribute: attributes[1].to_byte_vec(),
                blind_sign_req: blind_sign_request.to_bytes(),
            })
        }
    }
}
