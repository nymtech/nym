// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::utils::scalar_serde_helper;
use nym_coconut_interface::{hash_to_scalar, Attribute, PublicAttribute};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime, Time};
use zeroize::{Zeroize, ZeroizeOnDrop};

pub const MAX_FREE_PASS_VALIDITY: Duration = Duration::WEEK; // 1 week

#[derive(Zeroize, ZeroizeOnDrop, Serialize, Deserialize)]
pub struct FreePassIssuedData {
    /// the plain validity value of this credential expressed as unix timestamp
    #[zeroize(skip)]
    expiry_date: OffsetDateTime,
}

impl<'a> From<&'a FreePassIssuanceData> for FreePassIssuedData {
    fn from(value: &'a FreePassIssuanceData) -> Self {
        FreePassIssuedData {
            expiry_date: value.expiry_date,
        }
    }
}

impl FreePassIssuedData {
    pub fn expiry_date_plain(&self) -> String {
        self.expiry_date.unix_timestamp().to_string()
    }
}

#[derive(Zeroize, ZeroizeOnDrop, Serialize, Deserialize)]
pub struct FreePassIssuanceData {
    /// the plain validity value of this credential expressed as unix timestamp
    #[zeroize(skip)]
    expiry_date: OffsetDateTime,

    // the expiry date, as unix timestamp, hashed into a scalar
    #[serde(with = "scalar_serde_helper")]
    expiry_date_prehashed: PublicAttribute,
}

impl FreePassIssuanceData {
    pub fn new(expiry_date: Option<OffsetDateTime>) -> Self {
        // ideally we should have implemented a proper error handling here, sure.
        // but given it's meant to only be used by nym, imo it's fine to just panic here in case of invalid arguments
        let expiry_date = if let Some(provided) = expiry_date {
            if provided - OffsetDateTime::now_utc() > MAX_FREE_PASS_VALIDITY {
                panic!("the provided expiry date is bigger than the maximum value of {MAX_FREE_PASS_VALIDITY}");
            }

            provided
        } else {
            Self::default_expiry_date()
        };

        let expiry_date_prehashed = hash_to_scalar(expiry_date.unix_timestamp().to_string());

        FreePassIssuanceData {
            expiry_date,
            expiry_date_prehashed,
        }
    }

    pub fn default_expiry_date() -> OffsetDateTime {
        // set it to furthest midnight in the future such as it's no more than a week away,
        // i.e. if it's currently for example 9:43 on 2nd March 2024, it will set it to 0:00 on 9th March 2024
        (OffsetDateTime::now_utc() + MAX_FREE_PASS_VALIDITY).replace_time(Time::MIDNIGHT)
    }

    pub fn expiry_date_attribute(&self) -> &Attribute {
        &self.expiry_date_prehashed
    }

    pub fn expiry_date_plain(&self) -> String {
        self.expiry_date.unix_timestamp().to_string()
    }
}
