// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use wasm_timer;

pub use wasm_timer::*;

pub(crate) fn get_time_now() -> Instant {
    wasm_timer::Instant::now()
}
