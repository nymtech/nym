// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_credentials::IssuedTicketBook;
use nym_ecash_time::Date;
use time::OffsetDateTime;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub struct RetrievedTicketbook {
    pub ticketbook_id: i64,
    pub total_tickets: u32,
    pub ticketbook: IssuedTicketBook,
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

/// The global signing data currently held in storage, identified by epoch (and expiration date for
/// the expiration-date signatures) - without loading the (large) data itself.
#[derive(Debug, Default)]
pub struct AvailableGlobalData {
    pub master_verification_key_epochs: Vec<u64>,
    pub coin_index_signature_epochs: Vec<u64>,
    pub expiration_date_signatures: Vec<(u64, Date)>,
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
pub struct RawExpirationDateSignatures {
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

#[derive(Clone, Debug)]
#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::FromRow))]
pub struct EmergencyCredential {
    pub id: i64,
    #[cfg_attr(not(target_arch = "wasm32"), sqlx(flatten))]
    pub data: EmergencyCredentialContent,
}

#[derive(Clone, Debug)]
#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::FromRow))]
pub struct EmergencyCredentialContent {
    #[cfg_attr(not(target_arch = "wasm32"), sqlx(rename = "type"))]
    pub typ: String,
    pub content: Vec<u8>,
    pub expiration: Option<OffsetDateTime>,
}
