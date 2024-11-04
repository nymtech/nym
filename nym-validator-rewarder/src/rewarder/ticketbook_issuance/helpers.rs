// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::ops::Add;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::time::{interval_at, Instant, Interval};

pub fn end_of_day_ticker() -> Interval {
    let now = OffsetDateTime::now_utc();

    // safety: we're not running this in year 9999...
    #[allow(clippy::unwrap_used)]
    let next_day = now.date().next_day().unwrap().midnight().assume_utc();

    // safety: the duration is guaranteed to be positive
    #[allow(clippy::unwrap_used)]
    let until_next_day: Duration = (next_day - now).try_into().unwrap();

    interval_at(
        // add extra 2h to account for leeway with issuance at the beginning of a day
        Instant::now()
            .add(until_next_day)
            .add(Duration::from_secs(2 * 60 * 60)),
        Duration::from_secs(24 * 60 * 60),
    )
}
