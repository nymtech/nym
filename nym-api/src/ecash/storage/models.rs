// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::error::EcashError;
use crate::node_status_api::models::NymApiStorageError;
use nym_api_requests::ecash::models::{
    EpochCredentialsResponse, IssuedTicketbookBody as ApiIssuedCredentialInner,
    IssuedTicketbookDeprecated as ApiIssuedCredential,
};
use nym_api_requests::ecash::BlindedSignatureResponse;
use nym_compact_ecash::BlindedSignature;
use nym_config::defaults::BloomfilterParameters;
use nym_credentials_interface::TicketType;
use nym_crypto::asymmetric::ed25519;
use nym_ecash_contract_common::deposit::DepositId;
use sqlx::FromRow;
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

pub struct IssuedHash {
    pub deposit_id: DepositId,
    pub merkle_leaf: [u8; 32],
    pub merkle_index: usize,
}

impl IssuedHash {
    pub fn new(deposit_id: DepositId, merkle_leaf: [u8; 32], merkle_index: usize) -> Self {
        IssuedHash {
            deposit_id,
            merkle_leaf,
            merkle_index,
        }
    }
}

#[deprecated]
#[derive(FromRow)]
pub struct IssuedTicketbook {
    pub id: i64,
    pub epoch_id: u32,
    pub deposit_id: DepositId,

    pub partial_credential: Vec<u8>,

    /// signature on the issued credential (and the attributes)
    pub signature: Vec<u8>,

    pub joined_private_commitments: Vec<u8>,

    pub expiration_date: Date,

    pub ticketbook_type_repr: u8,
}

#[derive(FromRow)]
pub struct RawIssuedTicketbook {
    pub deposit_id: DepositId,

    pub dkg_epoch_id: u32,

    pub blinded_partial_credential: Vec<u8>,

    pub joined_private_commitments: Vec<u8>,

    pub expiration_date: Date,

    pub ticketbook_type_repr: u8,

    /// hash on the whole data as in what has been inserted into the merkle tree
    pub merkle_leaf: Vec<u8>,
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

impl TryFrom<IssuedTicketbook> for ApiIssuedCredentialInner {
    type Error = EcashError;

    fn try_from(value: IssuedTicketbook) -> Result<Self, Self::Error> {
        Ok(ApiIssuedCredentialInner {
            credential: ApiIssuedCredential {
                id: value.id,
                epoch_id: value.epoch_id,
                deposit_id: value.deposit_id,
                blinded_partial_credential: BlindedSignature::from_bytes(
                    &value.partial_credential,
                )?,
                encoded_private_attributes_commitments: split_attributes(
                    value.joined_private_commitments,
                ),
                expiration_date: value.expiration_date,
                ticketbook_type: TicketType::try_from_encoded(value.ticketbook_type_repr)?,
            },
            signature: ed25519::Signature::from_bytes(&value.signature)?,
        })
    }
}

impl TryFrom<IssuedTicketbook> for BlindedSignatureResponse {
    type Error = EcashError;

    fn try_from(value: IssuedTicketbook) -> Result<Self, Self::Error> {
        Ok(BlindedSignatureResponse {
            blinded_signature: BlindedSignature::from_bytes(&value.partial_credential)?,
        })
    }
}

impl TryFrom<IssuedTicketbook> for BlindedSignature {
    type Error = EcashError;

    fn try_from(value: IssuedTicketbook) -> Result<Self, Self::Error> {
        Ok(BlindedSignature::from_bytes(&value.partial_credential)?)
    }
}

pub(crate) fn join_attributes(attrs: Vec<Vec<u8>>) -> Vec<u8> {
    // note: 48 is length of encoded G1 element
    let mut out = Vec::with_capacity(48 * attrs.len());
    for mut attr in attrs {
        // since this is called internally only, we expect valid attributes here!
        assert_eq!(attr.len(), 48);

        out.append(&mut attr)
    }

    out
}

pub(crate) fn split_attributes(attrs: Vec<u8>) -> Vec<Vec<u8>> {
    assert_eq!(attrs.len() % 48, 0, "database corruption");
    attrs.chunks_exact(48).map(|c| c.to_vec()).collect()
}
