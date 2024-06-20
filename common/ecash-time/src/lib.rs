// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use time::{Duration, Time};

pub use time::OffsetDateTime;

pub fn ecash_today() -> OffsetDateTime {
    let now_utc = OffsetDateTime::now_utc();
    now_utc.replace_time(Time::MIDNIGHT)
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
