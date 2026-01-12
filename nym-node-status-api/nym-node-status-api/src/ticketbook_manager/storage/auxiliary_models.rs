// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_credentials::IssuedTicketBook;
use nym_credentials::ecash::bandwidth::serialiser::VersionedSerialise;
use nym_node_status_client::models::AttachedTicket;
use sqlx::FromRow;
use time::Date;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub struct RetrievedTicketbook {
    pub ticketbook_id: i32,
    pub total_tickets: u32,
    pub spent_tickets: u32,
    pub ticketbook: IssuedTicketBook,
}

impl From<RetrievedTicketbook> for AttachedTicket {
    fn from(retrieved: RetrievedTicketbook) -> Self {
        AttachedTicket {
            ticketbook: retrieved.ticketbook.pack(),
            usable_index: retrieved.spent_tickets,
        }
    }
}

#[derive(Zeroize, ZeroizeOnDrop, Clone, FromRow)]
pub struct StoredIssuedTicketbook {
    pub id: i32,

    pub serialization_revision: i16,

    pub ticketbook_type: String,

    pub ticketbook_data: Vec<u8>,

    #[zeroize(skip)]
    pub expiration_date: Date,

    pub epoch_id: i32,

    pub total_tickets: i32,
    pub used_tickets: i32,
}
