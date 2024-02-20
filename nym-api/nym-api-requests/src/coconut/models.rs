// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::helpers::issued_credential_plaintext;
use cosmrs::AccountId;
use nym_credentials_interface::{
    hash_to_scalar, Attribute, BlindSignRequest, BlindedSignature, Bytable, CoconutError,
    CredentialSpendingData, VerificationKey,
};
use nym_crypto::asymmetric::identity;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tendermint::hash::Hash;

#[derive(Serialize, Deserialize)]
pub struct VerifyCredentialBody {
    /// The cryptographic material required for spending the underlying credential.
    pub credential_data: CredentialSpendingData,

    /// Multisig proposal for releasing funds for the provided bandwidth credential
    pub proposal_id: u64,

    /// Cosmos address of the spender of the credential
    pub gateway_cosmos_addr: AccountId,
}

impl VerifyCredentialBody {
    pub fn new(
        credential_data: CredentialSpendingData,
        proposal_id: u64,
        gateway_cosmos_addr: AccountId,
    ) -> VerifyCredentialBody {
        VerifyCredentialBody {
            credential_data,
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

    pub fn encode_commitments(&self) -> Vec<String> {
        use nym_credentials_interface::Base58;

        self.inner_sign_request
            .get_private_attributes_pedersen_commitments()
            .iter()
            .map(|c| c.to_bs58())
            .collect()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FreePassNonceResponse {
    pub current_nonce: [u8; 16],
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlindedSignatureResponse {
    pub blinded_signature: BlindedSignature,
}

impl BlindedSignatureResponse {
    pub fn new(blinded_signature: BlindedSignature) -> BlindedSignatureResponse {
        BlindedSignatureResponse { blinded_signature }
    }

    pub fn to_base58_string(&self) -> String {
        bs58::encode(&self.to_bytes()).into_string()
    }

    pub fn from_base58_string<I: AsRef<[u8]>>(val: I) -> Result<Self, CoconutError> {
        let bytes = bs58::decode(val).into_vec()?;
        Self::from_bytes(&bytes)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.blinded_signature.to_byte_vec()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CoconutError> {
        Ok(BlindedSignatureResponse {
            blinded_signature: BlindedSignature::from_bytes(bytes)?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FreePassRequest {
    // secp256k1 key associated with the admin account
    pub cosmos_pubkey: cosmrs::crypto::PublicKey,

    pub inner_sign_request: BlindSignRequest,

    // we need to include a nonce here to prevent replay attacks
    // (and not making the nym-api store the serial numbers of all issued credential)
    pub used_nonce: [u8; 16],

    /// Signature on the nonce
    /// to prove the possession of the cosmos key/address
    pub nonce_signature: cosmrs::crypto::secp256k1::Signature,

    pub public_attributes_plain: Vec<String>,
}

impl FreePassRequest {
    pub fn new(
        cosmos_pubkey: cosmrs::crypto::PublicKey,
        inner_sign_request: BlindSignRequest,
        used_nonce: [u8; 16],
        nonce_signature: cosmrs::crypto::secp256k1::Signature,
        public_attributes_plain: Vec<String>,
    ) -> Self {
        FreePassRequest {
            cosmos_pubkey,
            inner_sign_request,
            used_nonce,
            nonce_signature,
            public_attributes_plain,
        }
    }

    pub fn tendermint_pubkey(&self) -> tendermint::PublicKey {
        self.cosmos_pubkey.into()
    }

    pub fn public_attributes_hashed(&self) -> Vec<Attribute> {
        self.public_attributes_plain
            .iter()
            .map(hash_to_scalar)
            .collect()
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
pub struct Pagination<T> {
    /// last_key is the last value returned in the previous query.
    /// it's used to indicate the start of the next (this) page.
    /// the value itself is not included in the response.
    pub last_key: Option<T>,

    /// limit is the total number of results to be returned in the result page.
    /// If left empty it will default to a value to be set by each app.
    pub limit: Option<u32>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CredentialsRequestBody {
    /// Explicit ids of the credentials to retrieve. Note: it can't be set alongside pagination.
    pub credential_ids: Vec<i64>,

    /// Pagination settings for retrieving credentials. Note: it can't be set alongside explicit ids.
    pub pagination: Option<Pagination<i64>>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EpochCredentialsResponse {
    pub epoch_id: u64,
    pub first_epoch_credential_id: Option<i64>,
    pub total_issued: u32,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IssuedCredentialsResponse {
    // note: BTreeMap returns ordered results so it's fine to use it with pagination
    pub credentials: BTreeMap<i64, IssuedCredentialBody>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IssuedCredentialResponse {
    pub credential: Option<IssuedCredentialBody>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IssuedCredentialBody {
    pub credential: IssuedCredential,

    pub signature: identity::Signature,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IssuedCredential {
    pub id: i64,
    pub epoch_id: u32,
    pub tx_hash: Hash,

    // NOTE: if we find creation of this guy takes too long,
    // change `BlindedSignature` to `BlindedSignatureBytes`
    // so that nym-api wouldn't need to parse the value out of its storage
    pub blinded_partial_credential: BlindedSignature,
    pub bs58_encoded_private_attributes_commitments: Vec<String>,
    pub public_attributes: Vec<String>,
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
