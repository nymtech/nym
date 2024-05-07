// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::error::EcashError;
use crate::node_status_api::models::NymApiStorageError;
use nym_api_requests::ecash::models::{
    EpochCredentialsResponse, IssuedCredential as ApiIssuedCredential,
    IssuedCredentialBody as ApiIssuedCredentialInner,
};
use nym_api_requests::ecash::BlindedSignatureResponse;
use nym_compact_ecash::{Base58, BlindedSignature};
use nym_config::defaults::BloomfilterParameters;
use nym_ecash_contract_common::deposit::DepositId;
use sqlx::FromRow;
use std::fmt::Display;
use std::ops::Deref;
use time::{Date, OffsetDateTime};

pub struct EpochCredentials {
    pub epoch_id: u32,
    pub start_id: i64,
    pub total_issued: u32,
}

impl From<EpochCredentials> for EpochCredentialsResponse {
    fn from(value: EpochCredentials) -> Self {
        let first_epoch_credential_id = if value.start_id == -1 {
            None
        } else {
            Some(value.start_id)
        };

        EpochCredentialsResponse {
            epoch_id: value.epoch_id as u64,
            first_epoch_credential_id,
            total_issued: value.total_issued,
        }
    }
}

#[derive(FromRow)]
#[allow(unused)]
pub struct TicketProvider {
    pub(crate) id: i64,
    pub(crate) gateway_address: String,
    pub(crate) last_batch_verification: Option<OffsetDateTime>,
}

#[derive(FromRow)]
pub struct SerialNumberWrapper {
    pub serial_number: Vec<u8>,
}

impl Deref for SerialNumberWrapper {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.serial_number
    }
}

#[derive(FromRow)]
#[allow(unused)]
pub struct VerifiedTicket {
    pub(crate) id: i64,
    pub(crate) ticket_data: Vec<u8>,
    pub(crate) serial_number: Vec<u8>,
    pub(crate) spending_date: Date,
    pub(crate) verified_at: OffsetDateTime,
    pub(crate) gateway_id: i64,
}

#[derive(FromRow)]
pub struct IssuedCredential {
    pub id: i64,
    pub epoch_id: u32,
    pub deposit_id: DepositId,

    /// base58-encoded issued credential
    pub bs58_partial_credential: String,

    /// base58-encoded signature on the issued credential (and the attributes)
    pub bs58_signature: String,

    // i.e. "'attr1','attr2',..."
    pub joined_private_commitments: String,

    pub expiration_date: Date,
}

#[derive(FromRow)]
pub struct RawExpirationDateSignatures {
    pub epoch_id: u32,
    pub serialised_signatures: Vec<u8>,
}

#[derive(FromRow)]
pub(crate) struct StoredBloomfilterParams {
    pub(crate) id: i64,
    pub(crate) num_hashes: u32,
    pub(crate) bitmap_size: u32,

    pub(crate) sip0_key0: Vec<u8>,
    pub(crate) sip0_key1: Vec<u8>,

    pub(crate) sip1_key0: Vec<u8>,
    pub(crate) sip1_key1: Vec<u8>,
}

impl<'a> TryFrom<&'a StoredBloomfilterParams> for BloomfilterParameters {
    type Error = NymApiStorageError;
    fn try_from(value: &'a StoredBloomfilterParams) -> Result<Self, Self::Error> {
        let Ok(sip0_key0) = <[u8; 8]>::try_from(value.sip0_key0.as_ref()) else {
            return Err(NymApiStorageError::database_inconsistency(
                "malformed sip0 key0",
            ));
        };
        let Ok(sip0_key1) = <[u8; 8]>::try_from(value.sip0_key1.as_ref()) else {
            return Err(NymApiStorageError::database_inconsistency(
                "malformed sip0 key1",
            ));
        };
        let Ok(sip1_key0) = <[u8; 8]>::try_from(value.sip1_key0.as_ref()) else {
            return Err(NymApiStorageError::database_inconsistency(
                "malformed sip1 key0",
            ));
        };
        let Ok(sip1_key1) = <[u8; 8]>::try_from(value.sip1_key1.as_ref()) else {
            return Err(NymApiStorageError::database_inconsistency(
                "malformed sip1 key1",
            ));
        };
        Ok(BloomfilterParameters {
            num_hashes: value.num_hashes,
            bitmap_size: value.bitmap_size as u64,
            sip_keys: [
                (u64::from_be_bytes(sip0_key0), u64::from_be_bytes(sip0_key1)),
                (u64::from_be_bytes(sip1_key0), u64::from_be_bytes(sip1_key1)),
            ],
        })
    }
}

impl TryFrom<IssuedCredential> for ApiIssuedCredentialInner {
    type Error = EcashError;

    fn try_from(value: IssuedCredential) -> Result<Self, Self::Error> {
        Ok(ApiIssuedCredentialInner {
            credential: ApiIssuedCredential {
                id: value.id,
                epoch_id: value.epoch_id,
                deposit_id: value.deposit_id,
                blinded_partial_credential: BlindedSignature::try_from_bs58(
                    value.bs58_partial_credential,
                )?,
                bs58_encoded_private_attributes_commitments: split_attributes(
                    &value.joined_private_commitments,
                ),
                expiration_date: value.expiration_date,
            },
            signature: value.bs58_signature.parse()?,
        })
    }
}

impl TryFrom<IssuedCredential> for BlindedSignatureResponse {
    type Error = EcashError;

    fn try_from(value: IssuedCredential) -> Result<Self, Self::Error> {
        Ok(BlindedSignatureResponse {
            blinded_signature: BlindedSignature::try_from_bs58(value.bs58_partial_credential)?,
        })
    }
}

impl TryFrom<IssuedCredential> for BlindedSignature {
    type Error = EcashError;

    fn try_from(value: IssuedCredential) -> Result<Self, Self::Error> {
        Ok(BlindedSignature::try_from_bs58(
            value.bs58_partial_credential,
        )?)
    }
}

pub fn join_attributes<I, M>(attrs: I) -> String
where
    I: IntoIterator<Item = M>,
    M: Display,
{
    // I could have used `attrs.into_iter().join(",")`,
    // but my IDE didn't like it (compiler was fine)
    itertools::Itertools::join(&mut attrs.into_iter(), ",")
}

pub fn split_attributes(attrs: &str) -> Vec<String> {
    attrs.split(',').map(|s| s.to_string()).collect()
}
