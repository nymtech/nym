// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod error;

use getset::{CopyGetters, Getters};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use error::CoconutInterfaceError;

pub use nymcoconut::*;

#[derive(Serialize, Deserialize, Getters, CopyGetters, Clone)]
pub struct Credential {
    #[getset(get = "pub")]
    n_params: u32,
    #[getset(get = "pub")]
    theta: Theta,
    public_attributes: Vec<Vec<u8>>,
    #[getset(get = "pub")]
    signature: Signature,
}
impl Credential {
    pub fn new(
        n_params: u32,
        theta: Theta,
        voucher_value: String,
        voucher_info: String,
        signature: &Signature,
    ) -> Credential {
        let public_attributes = vec![voucher_value.into_bytes(), voucher_info.into_bytes()];
        Credential {
            n_params,
            theta,
            public_attributes,
            signature: *signature,
        }
    }

    pub fn voucher_value(&self) -> Result<u64, CoconutInterfaceError> {
        let bandwidth_vec = self
            .public_attributes
            .get(0)
            .ok_or(CoconutInterfaceError::NotEnoughPublicAttributes)?
            .to_owned();
        let bandwidth_str = String::from_utf8(bandwidth_vec)
            .map_err(|_| CoconutInterfaceError::InvalidBandwidth)?;
        let value =
            u64::from_str(&bandwidth_str).map_err(|_| CoconutInterfaceError::InvalidBandwidth)?;

        Ok(value)
    }

    pub fn verify(&self, verification_key: &VerificationKey) -> bool {
        let params = Parameters::new(self.n_params).unwrap();
        let public_attributes = self
            .public_attributes
            .iter()
            .map(hash_to_scalar)
            .collect::<Vec<Attribute>>();
        nymcoconut::verify_credential(&params, verification_key, &self.theta, &public_attributes)
    }
}

#[derive(Serialize, Deserialize, Debug, Getters, CopyGetters)]
pub struct VerifyCredentialBody {
    #[getset(get = "pub")]
    n_params: u32,
    #[getset(get = "pub")]
    theta: Theta,
    public_attributes: Vec<String>,
}

impl VerifyCredentialBody {
    pub fn new(
        n_params: u32,
        theta: &Theta,
        public_attributes: &[Attribute],
    ) -> VerifyCredentialBody {
        VerifyCredentialBody {
            n_params,
            theta: theta.clone(),
            public_attributes: public_attributes
                .iter()
                .map(|attr| attr.to_bs58())
                .collect(),
        }
    }

    pub fn public_attributes(&self) -> Vec<Attribute> {
        self.public_attributes
            .iter()
            .map(|x| Attribute::try_from_bs58(x).unwrap())
            .collect()
    }
}
//  All strings are base58 encoded representations of structs
#[derive(Clone, Serialize, Deserialize, Debug, Getters, CopyGetters)]
pub struct BlindSignRequestBody {
    #[getset(get = "pub")]
    blind_sign_request: BlindSignRequest,
    #[getset(get = "pub")]
    tx_hash: String,
    #[getset(get = "pub")]
    signature: String,
    public_attributes: Vec<String>,
    #[getset(get = "pub")]
    public_attributes_plain: Vec<String>,
    #[getset(get = "pub")]
    total_params: u32,
}

impl BlindSignRequestBody {
    pub fn new(
        blind_sign_request: &BlindSignRequest,
        tx_hash: String,
        signature: String,
        public_attributes: &[Attribute],
        public_attributes_plain: Vec<String>,
        total_params: u32,
    ) -> BlindSignRequestBody {
        BlindSignRequestBody {
            blind_sign_request: blind_sign_request.clone(),
            tx_hash,
            signature,
            public_attributes: public_attributes
                .iter()
                .map(|attr| attr.to_bs58())
                .collect(),
            public_attributes_plain,
            total_params,
        }
    }

    pub fn public_attributes(&self) -> Vec<Attribute> {
        self.public_attributes
            .iter()
            .map(|x| Attribute::try_from_bs58(x).unwrap())
            .collect()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlindedSignatureResponse {
    pub remote_key: [u8; 32],
    pub encrypted_signature: Vec<u8>,
}

impl BlindedSignatureResponse {
    pub fn new(encrypted_signature: Vec<u8>, remote_key: [u8; 32]) -> BlindedSignatureResponse {
        BlindedSignatureResponse {
            encrypted_signature,
            remote_key,
        }
    }

    pub fn to_base58_string(&self) -> String {
        bs58::encode(&self.to_bytes()).into_string()
    }

    pub fn from_base58_string<I: AsRef<[u8]>>(val: I) -> Result<Self, CoconutInterfaceError> {
        let bytes = bs58::decode(val).into_vec()?;
        Self::from_bytes(&bytes)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.remote_key.to_vec();
        bytes.extend_from_slice(&self.encrypted_signature);
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CoconutInterfaceError> {
        if bytes.len() < 32 {
            return Err(CoconutInterfaceError::InvalidByteLength(bytes.len(), 32));
        }
        let mut remote_key = [0u8; 32];
        remote_key.copy_from_slice(&bytes[..32]);
        let encrypted_signature = bytes[32..].to_vec();
        Ok(BlindedSignatureResponse {
            remote_key,
            encrypted_signature,
        })
    }
}

#[derive(Serialize, Deserialize)]
pub struct VerificationKeyResponse {
    pub key: VerificationKey,
}

impl VerificationKeyResponse {
    pub fn new(key: VerificationKey) -> VerificationKeyResponse {
        VerificationKeyResponse { key }
    }
}
