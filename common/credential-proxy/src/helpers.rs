// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use rand::rngs::OsRng;
use rand::RngCore;
use time::OffsetDateTime;
use tracing::{debug, info, warn};
use uuid::Uuid;

pub fn random_uuid() -> Uuid {
    let mut bytes = [0u8; 16];
    let mut rng = OsRng;
    rng.fill_bytes(&mut bytes);
    Uuid::from_bytes(bytes)
}

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

// #[allow(clippy::panic)]
// fn build_sha_short() -> &'static str {
//     let bin_info = bin_info!();
//     if bin_info.commit_sha.len() < 7 {
//         panic!("unavailable build commit sha")
//     }
//
//     if bin_info.commit_sha == "VERGEN_IDEMPOTENT_OUTPUT" {
//         error!("the binary hasn't been built correctly. it doesn't have a commit sha information");
//         return "unknown";
//     }
//
//     &bin_info.commit_sha[..7]
// }
