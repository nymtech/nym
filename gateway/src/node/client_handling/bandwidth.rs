// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_network_defaults::TicketTypeRepr;
use std::num::ParseIntError;
use thiserror::Error;
use time::error::ComponentRange;
use tracing::error;

#[derive(Debug, Error)]
pub enum BandwidthError {
    #[error("Provided bandwidth credential asks for more bandwidth than it is supported to add at once (credential value: {0}, supported: {}). Try to split it before attempting again", i64::MAX)]
    UnsupportedBandwidthValue(u64),

    #[error("failed to parse the bandwidth voucher value: {source}")]
    VoucherValueParsingFailure {
        #[source]
        source: ParseIntError,
    },

    #[error("failed to parse the free pass expiry date: {source}")]
    ExpiryDateParsingFailure {
        #[source]
        source: ParseIntError,
    },

    #[error("failed to parse expiry timestamp into proper datetime: {source}")]
    InvalidExpiryDate {
        unix_timestamp: i64,
        #[source]
        source: ComponentRange,
    },
}

#[derive(Debug, Copy, Clone)]
pub struct Bandwidth {
    value: u64,
}

impl Bandwidth {
    pub const fn new_unchecked(value: u64) -> Bandwidth {
        Bandwidth { value }
    }

    pub fn ticket_amount(typ: TicketTypeRepr) -> Self {
        Bandwidth {
            value: typ.bandwidth_value(),
        }
    }

    pub fn value(&self) -> u64 {
        self.value
    }
}
