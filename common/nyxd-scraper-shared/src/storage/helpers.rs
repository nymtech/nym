// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use tokio::time::Instant;
use tracing::{debug, error, info, trace, warn};

pub fn log_db_operation_time(op_name: &str, start_time: Instant) {
    let elapsed = start_time.elapsed();
    let formatted = humantime::format_duration(elapsed);

    match elapsed.as_millis() {
        v if v > 10000 => error!("{op_name} took {formatted} to execute"),
        v if v > 1000 => warn!("{op_name} took {formatted} to execute"),
        v if v > 100 => info!("{op_name} took {formatted} to execute"),
        v if v > 10 => debug!("{op_name} took {formatted} to execute"),
        _ => trace!("{op_name} took {formatted} to execute"),
    }
}
