// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use time::OffsetDateTime;
use tracing::{debug, info, warn};

pub struct LockTimer {
    created: OffsetDateTime,
    message: String,
}

impl LockTimer {
    pub fn new<S: Into<String>>(message: S) -> Self {
        LockTimer {
            message: message.into(),
            ..Default::default()
        }
    }
}

impl Drop for LockTimer {
    fn drop(&mut self) {
        let time_taken = OffsetDateTime::now_utc() - self.created;
        let time_taken_formatted = humantime::format_duration(time_taken.unsigned_abs());
        if time_taken > time::Duration::SECOND * 10 {
            warn!(time_taken = %time_taken_formatted, "{}", self.message)
        } else if time_taken > time::Duration::SECOND * 5 {
            info!(time_taken = %time_taken_formatted, "{}", self.message)
        } else {
            debug!(time_taken = %time_taken_formatted, "{}", self.message)
        };
    }
}

impl Default for LockTimer {
    fn default() -> Self {
        LockTimer {
            created: OffsetDateTime::now_utc(),
            message: "released the lock".to_string(),
        }
    }
}
