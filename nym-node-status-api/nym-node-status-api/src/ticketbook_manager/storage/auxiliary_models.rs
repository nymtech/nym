// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::bail;
use nym_credentials::IssuedTicketBook;
use nym_credentials::ecash::bandwidth::serialiser::VersionedSerialise;
use nym_node_status_client::models::AttachedTicket;
use sqlx::FromRow;
use time::Date;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub(super) struct RetrievedTicketbook {
    pub ticketbook_id: i32,
    pub total_tickets: u32,
    pub spent_tickets: u32,
    pub ticketbook: IssuedTicketBook,
}

impl TryFrom<RetrievedTicketbook> for AttachedTicket {
    type Error = anyhow::Error;

    fn try_from(retrieved: RetrievedTicketbook) -> Result<Self, Self::Error> {
        // spent_tickets is the post-increment number from the DB: the ticket we're
        // handing out has already been counted as "used" in the DB, but hasn't actually
        // been spent yet by the recipient. To get its 0-based index in the ticketbook,
        // subtract 1 (e.g. spent_tickets=1, the ticket at index 0).
        if retrieved.spent_tickets < 1 {
            bail!("Malformed ticket: cannot convert from ticket with spent_tickets=0");
        }
        let ticket = AttachedTicket {
            ticketbook: retrieved.ticketbook.pack(),
            usable_index: retrieved.spent_tickets - 1,
        };

        Ok(ticket)
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
