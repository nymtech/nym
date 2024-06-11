// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::helpers::issued_credential_plaintext;
use cosmrs::AccountId;
use nym_credentials_interface::{
    BlindedSignature, CompactEcashError, CredentialSpendingData, PartialCoinIndexSignature,
    PartialExpirationDateSignature, PublicKeyUser, VerificationKeyAuth, WithdrawalRequest,
};
use nym_crypto::asymmetric::identity;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt::Display};
use time::OffsetDateTime;

#[derive(Serialize, Deserialize, Clone)]
pub struct VerifyEcashCredentialBody {
    /// The cryptographic material required for spending the underlying credential.
    pub credential: CredentialSpendingData,

    /// Cosmos address of the sender of the credential
    pub gateway_cosmos_addr: AccountId,

    /// Multisig proposal for releasing funds for the provided bandwidth credential
    pub proposal_id: u64,
}

impl VerifyEcashCredentialBody {
    pub fn new(
        credential: CredentialSpendingData,
        gateway_cosmos_addr: AccountId,
        proposal_id: u64,
    ) -> VerifyEcashCredentialBody {
        VerifyEcashCredentialBody {
            credential,
            gateway_cosmos_addr,
            proposal_id,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum VerifyEcashCredentialResponse {
    InvalidFormat(String),
    DoubleSpend,
    AlreadySent,
    SubmittedTooLate {
        expected_until: OffsetDateTime,
        actual: OffsetDateTime,
    },
    Refused,
    Accepted,
}

impl Display for VerifyEcashCredentialResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFormat(reason) => {
                write!(f, "invalid format : {:?}", reason)
            }
            Self::DoubleSpend => write!(f, "credential was already spent"),
            Self::AlreadySent => write!(f, "this credential was already sent"),
            Self::SubmittedTooLate {
                expected_until,
                actual,
            } => {
                write!(
                    f,
                    "credential spent too late. Accepted from {:#?}, spent on {:#?}",
                    expected_until, actual,
                )
            }
            Self::Refused => write!(f, "credential failed to validate"),
            Self::Accepted => write!(f, "credential was accepted"),
        }
    }
}

//  All strings are base58 encoded representations of structs
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BlindSignRequestBody {
    pub inner_sign_request: WithdrawalRequest,

    /// the id of the associated deposit
    pub deposit_id: u32,

    /// Signature on the inner sign request and the tx hash
    pub signature: identity::Signature,

    pub ecash_pubkey: PublicKeyUser,

    pub expiration_date: OffsetDateTime,
}

impl BlindSignRequestBody {
    pub fn new(
        inner_sign_request: WithdrawalRequest,
        deposit_id: u32,
        signature: identity::Signature,
        ecash_pubkey: PublicKeyUser,
        expiration_date: OffsetDateTime,
    ) -> BlindSignRequestBody {
        BlindSignRequestBody {
            inner_sign_request,
            deposit_id,
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
    pub signatures: Vec<PartialExpirationDateSignature>,
}

impl PartialExpirationDateSignatureResponse {
    pub fn new(
        signatures: &[PartialExpirationDateSignature],
    ) -> PartialExpirationDateSignatureResponse {
        PartialExpirationDateSignatureResponse {
            signatures: signatures.to_owned(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct PartialCoinIndicesSignatureResponse {
    pub signatures: Vec<PartialCoinIndexSignature>,
}

impl PartialCoinIndicesSignatureResponse {
    pub fn new(signatures: &[PartialCoinIndexSignature]) -> PartialCoinIndicesSignatureResponse {
        PartialCoinIndicesSignatureResponse {
            signatures: signatures.to_owned(),
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
pub struct SpentCredentialsResponse {
    pub bitmap: Vec<u8>,
}

impl SpentCredentialsResponse {
    pub fn new(bitmap: Vec<u8>) -> SpentCredentialsResponse {
        SpentCredentialsResponse { bitmap }
    }
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
    pub deposit_id: u32,

    // NOTE: if we find creation of this guy takes too long,
    // change `BlindedSignature` to `BlindedSignatureBytes`
    // so that nym-api wouldn't need to parse the value out of its storage
    pub blinded_partial_credential: BlindedSignature,
    pub bs58_encoded_private_attributes_commitments: Vec<String>,
    pub expiration_date: OffsetDateTime,
}

impl IssuedCredential {
    // this method doesn't have to be reversible so just naively concatenate everything
    pub fn signable_plaintext(&self) -> Vec<u8> {
        issued_credential_plaintext(
            self.epoch_id,
            self.deposit_id,
            &self.blinded_partial_credential,
            &self.bs58_encoded_private_attributes_commitments,
            self.expiration_date,
        )
    }
}
