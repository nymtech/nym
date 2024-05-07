// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use log::error;
use nym_credentials::coconut::bandwidth::CredentialType;
use std::num::ParseIntError;
use thiserror::Error;
use time::error::ComponentRange;
use time::OffsetDateTime;

#[derive(Debug, Error)]
pub enum BandwidthError {
    #[error("Provided bandwidth credential asks for more bandwidth than it is supported to add at once (credential value: {0}, supported: {}). Try to split it before attempting again", i64::MAX)]
    UnsupportedBandwidthValue(u64),

    #[error("the provided free pass has already expired (expiry was on {expiry_date})")]
    ExpiredFreePass { expiry_date: OffsetDateTime },

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

    pub fn get_for_type(typ: CredentialType) -> Self {
        match typ {
            CredentialType::TicketBook => Bandwidth {
                value: nym_network_defaults::TICKET_BANDWIDTH_VALUE,
            },
            CredentialType::FreePass => Bandwidth {
                value: nym_network_defaults::BYTES_PER_FREEPASS,
            },
        }
    }

    pub fn value(&self) -> u64 {
        self.value
    }
}
