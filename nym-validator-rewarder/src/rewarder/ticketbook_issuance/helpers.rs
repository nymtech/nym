// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::ops::Add;
use std::time::Duration;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tokio::time::{interval_at, Instant, Interval};
use tracing::info;

pub fn end_of_day_ticker() -> Interval {
    let now = OffsetDateTime::now_utc();

    // safety: we're not running this in year 9999...
    #[allow(clippy::unwrap_used)]
    let next_day = now.date().next_day().unwrap().midnight().assume_utc();

    // safety: the duration is guaranteed to be positive
    #[allow(clippy::unwrap_used)]
    let until_next_day: Duration = (next_day - now).try_into().unwrap();

    // add extra 2h to account for leeway with issuance at the beginning of a day
    let until_next_rewarding = until_next_day.add(Duration::from_secs(2 * 60 * 60));

    // safety: we're using well-defined format provided by the library
    #[allow(clippy::unwrap_used)]
    let next_rewarding_rfc3339 = (now + until_next_rewarding).format(&Rfc3339).unwrap();
    info!(
        "the next ticketbook issuance rewarding will happen on {next_rewarding_rfc3339} ({} secs remaining)",
        until_next_rewarding.as_secs()
    );

    interval_at(
        Instant::now().add(until_next_rewarding),
        Duration::from_secs(24 * 60 * 60),
    )
}
