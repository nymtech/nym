// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;
use wasm_timer;

pub use wasm_timer::*;
pub type IntervalStream = gloo_timers::future::IntervalStream;

pub(crate) fn get_time_now() -> Instant {
    wasm_timer::Instant::now()
}

pub(crate) fn new_interval_stream(polling_rate: Duration) -> IntervalStream {
    gloo_timers::future::IntervalStream::new(polling_rate.as_millis() as u32)
}
