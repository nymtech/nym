// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::time::Duration;

pub(crate) enum StatisticsEvent {
    Connection {
        gateway_id: String,
        duration: Duration,
    },
}
