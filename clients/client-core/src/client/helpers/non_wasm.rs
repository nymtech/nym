// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use tokio::time::*;
pub type IntervalStream = tokio_stream::wrappers::IntervalStream;

pub(crate) fn get_time_now() -> Instant {
    Instant::now()
}

pub(crate) fn new_interval_stream(polling_rate: Duration) -> IntervalStream {
    tokio_stream::wrappers::IntervalStream::new(tokio::time::interval(polling_rate))
}
