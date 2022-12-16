// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use tokio::time::*;

pub(crate) fn get_time_now() -> Instant {
    Instant::now()
}
