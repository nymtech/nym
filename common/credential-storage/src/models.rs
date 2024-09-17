// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_credentials::{IssuanceTicketBook, IssuedTicketBook};
use nym_ecash_time::Date;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub struct RetrievedTicketbook {
    pub ticketbook_id: i64,
    pub ticketbook: IssuedTicketBook,
}

pub struct RetrievedPendingTicketbook {
    pub pending_id: i64,
    pub pending_ticketbook: IssuanceTicketBook,
}

#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::FromRow))]
pub struct BasicTicketbookInformation {
    pub id: i64,
    pub expiration_date: Date,
    pub ticketbook_type: String,
    pub epoch_id: u32,
    pub total_tickets: u32,
    pub used_tickets: u32,
}

#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::FromRow))]
#[derive(Zeroize, ZeroizeOnDrop, Clone)]
pub struct StoredIssuedTicketbook {
    pub id: i64,

    pub serialization_revision: u8,

    pub ticketbook_type: String,

    pub ticketbook_data: Vec<u8>,

    #[zeroize(skip)]
    pub expiration_date: Date,

    pub epoch_id: u32,

    pub total_tickets: u32,
    pub used_tickets: u32,
}

#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::FromRow))]
#[derive(Zeroize, ZeroizeOnDrop, Clone)]
pub struct StoredPendingTicketbook {
    pub deposit_id: i64,

    pub serialization_revision: u8,

    pub pending_ticketbook_data: Vec<u8>,

    #[zeroize(skip)]
    pub expiration_date: Date,
}

#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::FromRow))]
pub struct RawExpirationDateSignatures {
    pub epoch_id: u32,
    pub serialised_signatures: Vec<u8>,
    pub serialization_revision: u8,
}

#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::FromRow))]
pub struct RawCoinIndexSignatures {
    pub epoch_id: u32,
    pub serialised_signatures: Vec<u8>,
    pub serialization_revision: u8,
}

#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::FromRow))]
pub struct RawVerificationKey {
    pub epoch_id: u32,
    pub serialised_key: Vec<u8>,
    pub serialization_revision: u8,
}
