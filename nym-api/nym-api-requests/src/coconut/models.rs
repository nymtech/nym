// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::helpers::issued_credential_plaintext;
use cosmrs::AccountId;
use nym_credentials_interface::{
    BlindedSignature, CompactEcashError, CredentialSpendingData, OldCredentialSpendingData,
    PartialCoinIndexSignature, PartialExpirationDateSignature, PublicKeyUser, VerificationKeyAuth,
    WithdrawalRequest,
};
use nym_crypto::asymmetric::identity;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tendermint::hash::Hash;

#[derive(Serialize, Deserialize)]
pub struct VerifyCredentialBody {
    /// The cryptographic material required for spending the underlying credential.
    pub credential_data: OldCredentialSpendingData,

    /// Multisig proposal for releasing funds for the provided bandwidth credential
    pub proposal_id: u64,

    /// Cosmos address of the spender of the credential
    pub gateway_cosmos_addr: AccountId,
}

impl VerifyCredentialBody {
    pub fn new(
        credential_data: OldCredentialSpendingData,
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

#[derive(Serialize, Deserialize, Clone)]
pub struct OfflineVerifyCredentialBody {
    pub credential: CredentialSpendingData,

    pub gateway_cosmos_addr: AccountId,
}

impl OfflineVerifyCredentialBody {
    pub fn new(
        credential: CredentialSpendingData,
        gateway_cosmos_addr: AccountId,
    ) -> OfflineVerifyCredentialBody {
        OfflineVerifyCredentialBody {
            credential,
            gateway_cosmos_addr,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct OnlineVerifyCredentialBody {
    /// The cryptographic material required for spending the underlying credential.
    pub credential: CredentialSpendingData,

    /// Multisig proposal for releasing funds for the provided bandwidth credential
    pub proposal_id: u64,

    /// Cosmos address of the spender of the credential
    pub gateway_cosmos_addr: AccountId,
}

impl OnlineVerifyCredentialBody {
    pub fn new(
        credential: CredentialSpendingData,
        proposal_id: u64,
        gateway_cosmos_addr: AccountId,
    ) -> OnlineVerifyCredentialBody {
        OnlineVerifyCredentialBody {
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
    pub inner_sign_request: WithdrawalRequest,

    /// Hash of the deposit transaction
    pub tx_hash: Hash,

    /// Signature on the inner sign request and the tx hash
    pub signature: identity::Signature,

    pub ecash_pubkey: PublicKeyUser,

    pub expiration_date: u64,
}

impl BlindSignRequestBody {
    pub fn new(
        inner_sign_request: WithdrawalRequest,
        tx_hash: Hash,
        signature: identity::Signature,
        ecash_pubkey: PublicKeyUser,
        expiration_date: u64,
    ) -> BlindSignRequestBody {
        BlindSignRequestBody {
            inner_sign_request,
            tx_hash,
            signature,
            ecash_pubkey,
            expiration_date,
        }
    }

    pub fn encode_commitments(&self) -> Vec<String> {
        use nym_compact_ecash::Base58;

        self.inner_sign_request
            .get_private_attributes_commitments()
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

    pub fn from_base58_string<I: AsRef<[u8]>>(val: I) -> Result<Self, CompactEcashError> {
        let bytes = bs58::decode(val).into_vec()?;
        Self::from_bytes(&bytes)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.blinded_signature.to_bytes().to_vec()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CompactEcashError> {
        Ok(BlindedSignatureResponse {
            blinded_signature: BlindedSignature::from_bytes(bytes)?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FreePassRequest {
    // secp256k1 key associated with the admin account
    pub cosmos_pubkey: cosmrs::crypto::PublicKey,

    pub inner_sign_request: WithdrawalRequest,

    // we need to include a nonce here to prevent replay attacks
    // (and not making the nym-api store the serial numbers of all issued credential)
    pub used_nonce: [u8; 16],

    /// Signature on the nonce
    /// to prove the possession of the cosmos key/address
    pub nonce_signature: cosmrs::crypto::secp256k1::Signature,

    pub ecash_pubkey: PublicKeyUser,

    pub expiration_date: u64,
}

impl FreePassRequest {
    pub fn new(
        cosmos_pubkey: cosmrs::crypto::PublicKey,
        inner_sign_request: WithdrawalRequest,
        used_nonce: [u8; 16],
        nonce_signature: cosmrs::crypto::secp256k1::Signature,
        ecash_pubkey: PublicKeyUser,
        expiration_date: u64,
    ) -> Self {
        FreePassRequest {
            cosmos_pubkey,
            inner_sign_request,
            used_nonce,
            nonce_signature,
            ecash_pubkey,
            expiration_date,
        }
    }

    pub fn tendermint_pubkey(&self) -> tendermint::PublicKey {
        self.cosmos_pubkey.into()
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

#[derive(Serialize, Deserialize)]
pub struct PartialExpirationDateSignatureResponse {
    pub signs: Vec<PartialExpirationDateSignature>,
}

impl PartialExpirationDateSignatureResponse {
    pub fn new(signs: &[PartialExpirationDateSignature]) -> PartialExpirationDateSignatureResponse {
        PartialExpirationDateSignatureResponse {
            signs: signs.to_owned(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct PartialCoinIndicesSignatureResponse {
    pub signs: Vec<PartialCoinIndexSignature>,
}

impl PartialCoinIndicesSignatureResponse {
    pub fn new(signs: &[PartialCoinIndexSignature]) -> PartialCoinIndicesSignatureResponse {
        PartialCoinIndicesSignatureResponse {
            signs: signs.to_owned(),
        }
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
    pub expiration_date: i64,
}

impl IssuedCredential {
    // this method doesn't have to be reversible so just naively concatenate everything
    pub fn signable_plaintext(&self) -> Vec<u8> {
        issued_credential_plaintext(
            self.epoch_id,
            self.tx_hash,
            &self.blinded_partial_credential,
            &self.bs58_encoded_private_attributes_commitments,
            self.expiration_date,
        )
    }
}
