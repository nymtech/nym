// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::helpers::issued_credential_plaintext;
use cosmrs::AccountId;
use nym_coconut_interface::{
    error::CoconutInterfaceError, hash_to_scalar, Attribute, BlindSignRequest, BlindedSignature,
    Credential, VerificationKey,
};
use nym_crypto::asymmetric::identity;
use serde::{Deserialize, Serialize};
use tendermint::hash::Hash;

#[derive(Serialize, Deserialize)]
pub struct VerifyCredentialBody {
    pub credential: Credential,

    pub proposal_id: u64,

    pub gateway_cosmos_addr: AccountId,
}

impl VerifyCredentialBody {
    pub fn new(
        credential: Credential,
        proposal_id: u64,
        gateway_cosmos_addr: AccountId,
    ) -> VerifyCredentialBody {
        VerifyCredentialBody {
            credential,
            proposal_id,
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
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BlindSignRequestBody {
    pub inner_sign_request: BlindSignRequest,

    /// Hash of the deposit transaction
    pub tx_hash: Hash,

    /// Signature on the inner sign request and the tx hash
    pub signature: identity::Signature,

    // public_attributes: Vec<String>,
    pub public_attributes_plain: Vec<String>,
}

impl BlindSignRequestBody {
    pub fn new(
        inner_sign_request: BlindSignRequest,
        tx_hash: Hash,
        signature: identity::Signature,
        public_attributes_plain: Vec<String>,
    ) -> BlindSignRequestBody {
        BlindSignRequestBody {
            inner_sign_request,
            tx_hash,
            signature,
            public_attributes_plain,
        }
    }

    pub fn attributes(&self) -> u32 {
        (self.public_attributes_plain.len() + self.inner_sign_request.num_private_attributes())
            as u32
    }

    pub fn public_attributes_hashed(&self) -> Vec<Attribute> {
        self.public_attributes_plain
            .iter()
            .map(hash_to_scalar)
            .collect()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlindedSignatureResponseNew {
    pub blinded_signature: BlindedSignature,
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

#[derive(Serialize, Deserialize)]
pub struct CosmosAddressResponse {
    pub addr: AccountId,
}

impl CosmosAddressResponse {
    pub fn new(addr: AccountId) -> CosmosAddressResponse {
        CosmosAddressResponse { addr }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CredentialsRequestBody {
    pub credential_ids: Vec<i64>,
}

pub struct EpochCredentialsResponse {
    //
}

pub struct IssuedCredentialResponse {
    credential: IssuedCredential,

    signature: identity::Signature,
}

pub struct IssuedCredential {
    epoch_id: u32,
    tx_hash: Hash,
    blinded_partial_credential: BlindedSignature,
    bs58_encoded_private_attributes_commitments: Vec<String>,
    public_attributes: Vec<String>,
}

impl IssuedCredential {
    // this method doesn't have to be reversible so just naively concatenate everything
    pub fn signable_plaintext(&self) -> Vec<u8> {
        issued_credential_plaintext(
            self.epoch_id,
            self.tx_hash,
            &self.blinded_partial_credential,
            &self.bs58_encoded_private_attributes_commitments,
            &self.public_attributes,
        )
    }
}
