// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::NymApiStorageError;
use nym_config::defaults::BloomfilterParameters;
use nym_credentials_interface::TicketType;
use nym_ecash_contract_common::deposit::DepositId;
use nym_ticketbooks_merkle::IssuedTicketbook;
use sqlx::FromRow;
use std::ops::Deref;
use time::{Date, OffsetDateTime};

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

    /// index of the leaf under which the data has been inserted
    pub merkle_index: u32,
}

impl TryFrom<RawIssuedTicketbook> for IssuedTicketbook {
    type Error = NymApiStorageError;
    fn try_from(raw: RawIssuedTicketbook) -> Result<Self, Self::Error> {
        Ok(IssuedTicketbook {
            deposit_id: raw.deposit_id,
            epoch_id: raw.dkg_epoch_id as u64,
            blinded_partial_credential: raw.blinded_partial_credential,
            joined_encoded_private_attributes_commitments: raw.joined_private_commitments,
            expiration_date: raw.expiration_date,
            ticketbook_type: TicketType::try_from_encoded(raw.ticketbook_type_repr)
                .map_err(|err| NymApiStorageError::database_inconsistency(err.to_string()))?,
        })
    }
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
