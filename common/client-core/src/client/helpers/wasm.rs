// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#![allow(unused_imports)]

use std::time::Duration;

pub use wasmtimer::{std::Instant, tokio::*};
pub type IntervalStream = gloo_timers::future::IntervalStream;

pub(crate) fn get_time_now() -> Instant {
    Instant::now()
}

pub(crate) fn new_interval_stream(polling_rate: Duration) -> IntervalStream {
    gloo_timers::future::IntervalStream::new(polling_rate.as_millis() as u32)
}
