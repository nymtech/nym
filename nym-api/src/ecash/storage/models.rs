// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node_status_api::models::NymApiStorageError;
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
pub struct IssuedTicketbooksCount {
    pub issuance_date: Date,
    pub expiration_date: Date,
    pub count: u32,
}

#[derive(FromRow)]
pub struct IssuedTicketbooksOnCount {
    pub expiration_date: Date,
    pub count: u32,
}

#[derive(FromRow)]
pub struct IssuedTicketbooksForCount {
    pub issuance_date: Date,
    pub count: u32,
}

impl From<IssuedTicketbooksCount> for nym_api_requests::ecash::models::IssuedTicketbooksCount {
    fn from(value: IssuedTicketbooksCount) -> Self {
        nym_api_requests::ecash::models::IssuedTicketbooksCount {
            issuance_date: value.issuance_date,
            expiration_date: value.expiration_date,
            count: value.count,
        }
    }
}

impl From<IssuedTicketbooksForCount>
    for nym_api_requests::ecash::models::IssuedTicketbooksForCount
{
    fn from(value: IssuedTicketbooksForCount) -> Self {
        nym_api_requests::ecash::models::IssuedTicketbooksForCount {
            issuance_date: value.issuance_date,
            count: value.count,
        }
    }
}

impl From<IssuedTicketbooksOnCount> for nym_api_requests::ecash::models::IssuedTicketbooksOnCount {
    fn from(value: IssuedTicketbooksOnCount) -> Self {
        nym_api_requests::ecash::models::IssuedTicketbooksOnCount {
            expiration_date: value.expiration_date,
            count: value.count,
        }
    }
}
