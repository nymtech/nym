// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use time::{Duration, PrimitiveDateTime, Time};

pub use time::{Date, OffsetDateTime};

pub trait EcashTime {
    fn ecash_unix_timestamp(&self) -> u32 {
        let ts = self.ecash_datetime().unix_timestamp();

        // just panic on pre-1970 timestamps...
        assert!(ts > 0);

        // and on anything in 22nd century...
        assert!(ts <= u32::MAX as i64);

        ts as u32
    }

    fn ecash_date(&self) -> Date {
        self.ecash_datetime().date()
    }

    fn ecash_datetime(&self) -> OffsetDateTime;
}

impl EcashTime for OffsetDateTime {
    fn ecash_datetime(&self) -> OffsetDateTime {
        self.replace_time(Time::MIDNIGHT)
    }
}

impl EcashTime for PrimitiveDateTime {
    fn ecash_datetime(&self) -> OffsetDateTime {
        self.assume_utc().ecash_datetime()
    }
}

impl EcashTime for Date {
    fn ecash_datetime(&self) -> OffsetDateTime {
        OffsetDateTime::new_utc(*self, Time::MIDNIGHT)
    }
}

pub fn ecash_today() -> OffsetDateTime {
    OffsetDateTime::now_utc().ecash_datetime()
}

pub fn ecash_today_date() -> Date {
    ecash_today().ecash_date()
}

// no point in supporting more than i8 variance
pub fn ecash_date_offset(offset: i8) -> OffsetDateTime {
    let today = ecash_today();

    let day = today + Duration::days(offset as i64);

    // make sure to correct the time in case of DST
    day.replace_time(Time::MIDNIGHT)
}

#[cfg(feature = "expiration")]
pub fn cred_exp_date() -> OffsetDateTime {
    //count today as well
    ecash_date_offset(nym_compact_ecash::constants::CRED_VALIDITY_PERIOD_DAYS as i8 - 1)
    // ecash_today() + Duration::days(constants::CRED_VALIDITY_PERIOD_DAYS as i64 - 1)
}

#[cfg(feature = "expiration")]
pub fn ecash_default_expiration_date() -> Date {
    cred_exp_date().ecash_date()
}
