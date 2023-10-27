// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmrs::AccountId;
use getset::{CopyGetters, Getters};
use nym_compact_ecash::{
    error::CompactEcashError,
    scheme::{withdrawal::WithdrawalRequest, EcashCredential},
    VerificationKeyAuth,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Getters, CopyGetters)]
pub struct VerifyCredentialBody {
    #[getset(get = "pub")]
    credential: EcashCredential,
    //#[getset(get = "pub")]
    //proposal_id: u64,
    #[getset(get = "pub")]
    gateway_cosmos_addr: AccountId,
}

impl VerifyCredentialBody {
    pub fn new(
        credential: EcashCredential,
        //proposal_id: u64,
        gateway_cosmos_addr: AccountId,
    ) -> VerifyCredentialBody {
        VerifyCredentialBody {
            credential,
            //proposal_id,
            gateway_cosmos_addr,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyCredentialResponse {
    pub verification_result: bool,
}

impl VerifyCredentialResponse {
    pub fn new(verification_result: bool) -> Self {
        VerifyCredentialResponse {
            verification_result,
        }
    }
}

//  All strings are base58 encoded representations of structs
#[derive(Clone, Serialize, Deserialize, Debug, Getters, CopyGetters)]
pub struct BlindSignRequestBody {
    #[getset(get = "pub")]
    withdrawal_request: WithdrawalRequest,
    #[getset(get = "pub")]
    tx_hash: String,
    #[getset(get = "pub")]
    signature: String,
    #[getset(get = "pub")]
    ecash_pubkey: String,
    //public_attributes: Vec<String>,
    #[getset(get = "pub")]
    public_attributes_plain: Vec<String>,
    #[getset(get = "pub")]
    total_params: u32,
}

impl BlindSignRequestBody {
    pub fn new(
        withdrawal_request: &WithdrawalRequest,
        tx_hash: String,
        signature: String,
        ecash_pubkey: String,
        //public_attributes: &[Attribute],
        public_attributes_plain: Vec<String>,
        total_params: u32,
    ) -> BlindSignRequestBody {
        BlindSignRequestBody {
            withdrawal_request: withdrawal_request.clone(),
            tx_hash,
            signature,
            ecash_pubkey,
            // public_attributes: public_attributes
            //     .iter()
            //     .map(|attr| attr.to_bs58())
            //     .collect(),
            public_attributes_plain,
            total_params,
        }
    }

    // pub fn public_attributes(&self) -> Vec<Attribute> {
    //     self.public_attributes
    //         .iter()
    //         .map(|x| Attribute::try_from_bs58(x).unwrap())
    //         .collect()
    // }
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

    pub fn from_base58_string<I: AsRef<[u8]>>(val: I) -> Result<Self, CompactEcashError> {
        let bytes = bs58::decode(val).into_vec()?;
        Self::from_bytes(&bytes)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.remote_key.to_vec();
        bytes.extend_from_slice(&self.encrypted_signature);
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CompactEcashError> {
        if bytes.len() < 32 {
            return Err(CompactEcashError::DeserializationMinLength {
                min: 32,
                actual: bytes.len(),
            });
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
    pub key: VerificationKeyAuth,
}

impl VerificationKeyResponse {
    pub fn new(key: VerificationKeyAuth) -> VerificationKeyResponse {
        VerificationKeyResponse { key }
    }
}

#[derive(Serialize, Deserialize)]
pub struct CosmosAddressResponse {
    pub addr: AccountId,
}

impl CosmosAddressResponse {
    pub fn new(addr: AccountId) -> CosmosAddressResponse {
        CosmosAddressResponse { addr }
    }
}
