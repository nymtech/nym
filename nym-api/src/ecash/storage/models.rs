// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::error::EcashError;
use nym_api_requests::coconut::models::{
    EpochCredentialsResponse, IssuedCredential as ApiIssuedCredential,
    IssuedCredentialBody as ApiIssuedCredentialInner,
};
use nym_api_requests::coconut::BlindedSignatureResponse;
use nym_compact_ecash::{Base58, BlindedSignature};
use nym_ecash_contract_common::deposit::DepositId;
use sqlx::FromRow;
use std::fmt::Display;
use std::ops::Deref;
use time::OffsetDateTime;

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
pub struct TicketProvider {
    #[allow(unused)]
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
pub struct VerifiedTicket {
    #[allow(unused)]
    pub(crate) id: i64,
    pub(crate) ticket_data: Vec<u8>,
    pub(crate) serial_number: Vec<u8>,
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

    pub expiration_date: OffsetDateTime,
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
