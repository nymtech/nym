// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::bail;
use nym_credentials::IssuedTicketBook;
use nym_credentials::ecash::bandwidth::serialiser::VersionedSerialise;
use nym_gateway_probe::types::AttachedTicket;
use nym_validator_client::nym_api::EpochId;
use sqlx::FromRow;
use time::Date;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub(crate) struct RetrievedTicketbook {
    usable_index: u32,
    ticketbook: IssuedTicketBook,
}

impl RetrievedTicketbook {
    pub fn new(ticketbook: IssuedTicketBook) -> anyhow::Result<Self> {
        let usable_index = ticketbook.spent_tickets() as u32 - 1;
        // spent_tickets is the post-increment number from the DB: the ticket we're
        // handing out has already been counted as "used" in the DB, but has NOT YET
        // been spent yet by the recipient. To get its 0-based index in the ticketbook,
        // subtract 1 (e.g. spent_tickets=1, the ticket at index 0).
        if usable_index < 1 {
            bail!("Malformed ticket: cannot convert from ticket with spent_tickets=0");
        }
        Ok(Self {
            usable_index,
            ticketbook,
        })
    }

    pub fn epoch_id(&self) -> EpochId {
        self.ticketbook.epoch_id()
    }

    pub fn expiration_date(&self) -> time::Date {
        self.ticketbook.expiration_date()
    }
}

impl From<RetrievedTicketbook> for AttachedTicket {
    fn from(retrieved: RetrievedTicketbook) -> Self {
        AttachedTicket {
            ticketbook: retrieved.ticketbook.pack(),
            usable_index: retrieved.usable_index,
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
