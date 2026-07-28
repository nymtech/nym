// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::fmt;

use nym_credential_storage::models::BasicTicketbookInformation;
use nym_credentials_interface::TicketType;
use nym_ecash_time::{Date, EcashTime, OffsetDateTime};
use nym_validator_client::nym_api::EpochId;
use strum::IntoEnumIterator;

use crate::{config::BandwidthControllerConfig, error::BandwidthControllerError};

#[derive(Debug, Clone, PartialEq)]
pub struct AvailableTicketbook {
    pub id: i64,
    pub typ: TicketType,
    pub expiration: Date,
    pub epoch_id: EpochId,
    pub issued_tickets: u32,
    pub claimed_tickets: u32,
    pub ticket_size: u64,
}

impl AvailableTicketbook {
    pub fn issued_tickets_si(&self) -> String {
        si_scale::helpers::bibytes2(self.issued_tickets as u64 * self.ticket_size)
    }

    pub fn remaining_tickets(&self) -> u32 {
        self.issued_tickets.saturating_sub(self.claimed_tickets)
    }

    pub fn remaining_tickets_si(&self) -> String {
        si_scale::helpers::bibytes2(self.remaining_tickets() as u64 * self.ticket_size)
    }

    pub fn ticket_size_si(&self) -> String {
        si_scale::helpers::bibytes2(self.ticket_size)
    }

    pub fn has_expired(&self) -> bool {
        self.expiration <= nym_ecash_time::ecash_today().date()
    }

    // If that ticketbook will be expired in SOON_EXPIRY_THRESHOLD
    pub fn expired_soon(
        &self,
        datetime: OffsetDateTime,
        bc_config: &BandwidthControllerConfig,
    ) -> bool {
        self.expiration.ecash_datetime() < datetime + bc_config.soon_expiry_threshold
    }
}

impl fmt::Display for AvailableTicketbook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ecash_today = nym_ecash_time::ecash_today().date();

        let expiration = if self.expiration <= ecash_today {
            format!("EXPIRED ON: {}", self.expiration)
        } else {
            format!("expires: {}", self.expiration)
        };

        write!(
            f,
            "{{ id: {}, type: {}, epoch_id: {}, tickets: {}/{}, size: {}, remaining: {}/{}, {} }}",
            self.id,
            self.typ,
            self.epoch_id,
            self.remaining_tickets(),
            self.issued_tickets,
            self.ticket_size_si(),
            self.remaining_tickets_si(),
            self.issued_tickets_si(),
            expiration
        )
    }
}

impl TryFrom<BasicTicketbookInformation> for AvailableTicketbook {
    type Error = BandwidthControllerError;

    fn try_from(value: BasicTicketbookInformation) -> Result<Self, Self::Error> {
        let typ = value
            .ticketbook_type
            .parse()
            .map_err(|_| BandwidthControllerError::ParseTicketType(value.ticketbook_type))?;
        Ok(AvailableTicketbook {
            id: value.id,
            typ,
            expiration: value.expiration_date,
            epoch_id: EpochId::from(value.epoch_id),
            issued_tickets: value.total_tickets,
            claimed_tickets: value.used_tickets,
            ticket_size: typ.to_repr().bandwidth_value(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AvailableTicketbooks {
    pub ticketbooks: Vec<AvailableTicketbook>,
}

impl AvailableTicketbooks {
    pub fn remaining_tickets(&self, typ: TicketType) -> u64 {
        self.tickets_by_type(typ)
            .filter(|ticketbook| !ticketbook.has_expired())
            .map(|ticketbook| ticketbook.remaining_tickets())
            .fold(0, |acc, remaining| acc.saturating_add(remaining.into()))
    }

    pub fn remaining_data(&self, typ: TicketType) -> u64 {
        self.remaining_tickets(typ) * typ.to_repr().bandwidth_value()
    }

    pub fn remaining_data_si(&self, typ: TicketType) -> String {
        si_scale::helpers::bibytes2(self.remaining_data(typ) as f64)
    }

    fn tickets_by_type(&self, typ: TicketType) -> impl Iterator<Item = &AvailableTicketbook> {
        self.ticketbooks
            .iter()
            .filter(move |ticketbook| ticketbook.typ == typ)
    }

    /// The ticketbook that would be spent next for `typ`: the soonest-expiring usable one, mirroring
    /// storage's selection. Read-only - for its metadata (epoch/date), without withdrawing.
    pub(crate) fn next_usable(&self, typ: TicketType) -> Option<&AvailableTicketbook> {
        self.tickets_by_type(typ)
            .filter(|ticketbook| !ticketbook.has_expired() && ticketbook.remaining_tickets() > 0)
            .min_by_key(|ticketbook| ticketbook.expiration)
    }

    pub fn remaining_tickets_long_lasting(
        &self,
        typ: TicketType,
        bc_config: &BandwidthControllerConfig,
    ) -> u64 {
        self.tickets_by_type(typ)
            .filter(|ticketbook| !ticketbook.expired_soon(OffsetDateTime::now_utc(), bc_config))
            .map(|ticketbook| ticketbook.remaining_tickets())
            .fold(0, |acc, remaining| acc.saturating_add(remaining.into()))
    }

    pub fn remaining_unexpired_tickets(&self, typ: TicketType) -> u64 {
        self.tickets_by_type(typ)
            .filter(|ticketbook| !ticketbook.has_expired())
            .map(|ticketbook| ticketbook.remaining_tickets())
            .fold(0, |acc, remaining| acc.saturating_add(remaining.into()))
    }

    /// Whether `typ` should be proactively restocked
    pub fn needs_restock(&self, typ: TicketType, bc_config: &BandwidthControllerConfig) -> bool {
        let remaining = self.remaining_tickets_long_lasting(typ, bc_config);
        remaining <= bc_config.nb_ticket_restock
    }

    pub fn contains_minimal_tickets(
        &self,
        typ: TicketType,
        bc_config: &BandwidthControllerConfig,
    ) -> bool {
        let remaining = self.remaining_unexpired_tickets(typ);
        remaining > bc_config.min_nb_ticket_needed
    }

    pub fn len(&self) -> usize {
        self.ticketbooks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ticketbooks.is_empty()
    }

    pub fn len_not_expired(&self) -> usize {
        self.ticketbooks
            .iter()
            .filter(|ticketbook| !ticketbook.has_expired())
            .count()
    }

    pub fn ticketbook_types() -> Vec<TicketType> {
        // We don't include the mixnet exit ticket type as it's not used by the client
        TicketType::iter()
            .filter(|&t| t != TicketType::V1MixnetExit)
            .collect()
    }
}

impl Iterator for AvailableTicketbooks {
    type Item = AvailableTicketbook;

    fn next(&mut self) -> Option<Self::Item> {
        self.ticketbooks.pop()
    }
}

impl From<Vec<AvailableTicketbook>> for AvailableTicketbooks {
    fn from(ticketbooks: Vec<AvailableTicketbook>) -> Self {
        Self { ticketbooks }
    }
}

impl TryFrom<Vec<BasicTicketbookInformation>> for AvailableTicketbooks {
    type Error = BandwidthControllerError;

    fn try_from(value: Vec<BasicTicketbookInformation>) -> Result<Self, Self::Error> {
        let ticketbooks: Vec<_> = value
            .into_iter()
            .filter_map(|ticketbook| {
                AvailableTicketbook::try_from(ticketbook)
                    .inspect_err(|err| {
                        tracing::error!("Failed to parse ticketbook {err}");
                    })
                    .ok()
            })
            .collect();
        Ok(AvailableTicketbooks::from(ticketbooks))
    }
}
