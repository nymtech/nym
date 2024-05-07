// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use log::{error, warn};
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

    pub fn new(bandwidth_value: u64) -> Result<Bandwidth, BandwidthError> {
        if bandwidth_value > i64::MAX as u64 {
            // note that this would have represented more than 1 exabyte,
            // which is like 125,000 worth of hard drives, so I don't think we have
            // to worry about it for now...
            warn!("Somehow we received bandwidth value higher than 9223372036854775807. We don't really want to deal with this now");
            return Err(BandwidthError::UnsupportedBandwidthValue(bandwidth_value));
        }

        Ok(Bandwidth {
            value: bandwidth_value,
        })
    }

    pub fn get_for_type(typ: CredentialType) -> Self {
        match typ {
            CredentialType::TicketBook => Bandwidth {
                value: nym_network_defaults::TICKET_BANDWIDTH_VALUE,
            },
            CredentialType::FreePass => Bandwidth {
                value: nym_network_defaults::BYTES_PER_FREEPASS,
            },
            CredentialType::Voucher => {
                unimplemented!()
            }
        }
    }

    pub(crate) fn parse_raw_bandwidth(
        value: &str,
        typ: CredentialType,
    ) -> Result<(u64, Option<OffsetDateTime>), BandwidthError> {
        let (bandwidth_value, freepass_expiration) =
            match typ {
                CredentialType::Voucher => {
                    let token_value: u64 = value
                        .parse()
                        .map_err(|source| BandwidthError::VoucherValueParsingFailure { source })?;
                    (token_value * nym_network_defaults::BYTES_PER_UTOKEN, None)
                }
                CredentialType::FreePass => {
                    let expiry_timestamp: i64 = value
                        .parse()
                        .map_err(|source| BandwidthError::ExpiryDateParsingFailure { source })?;

                    let expiry_date = OffsetDateTime::from_unix_timestamp(expiry_timestamp)
                        .map_err(|source| BandwidthError::InvalidExpiryDate {
                            unix_timestamp: expiry_timestamp,
                            source,
                        })?;
                    let now = OffsetDateTime::now_utc();

                    if expiry_date < now {
                        return Err(BandwidthError::ExpiredFreePass { expiry_date });
                    }
                    (nym_network_defaults::BYTES_PER_FREEPASS, Some(expiry_date))
                }
                CredentialType::TicketBook => {
                    unimplemented!()
                }
            };
        Ok((bandwidth_value, freepass_expiration))
    }

    pub fn value(&self) -> u64 {
        self.value
    }
}
