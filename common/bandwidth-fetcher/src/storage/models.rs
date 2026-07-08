// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_credentials::IssuanceTicketBook;
use nym_ecash_time::Date;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[cfg_attr(not(target_arch = "wasm32"), derive(sqlx::FromRow))]
#[derive(Zeroize, ZeroizeOnDrop, Clone)]
pub struct StoredPendingTicketbook {
    pub deposit_id: i64,

    pub serialization_revision: u8,

    pub pending_ticketbook_data: Vec<u8>,

    #[zeroize(skip)]
    pub expiration_date: Date,
}

pub struct RetrievedPendingTicketbook {
    pub pending_id: i64,
    pub pending_ticketbook: IssuanceTicketBook,
}
