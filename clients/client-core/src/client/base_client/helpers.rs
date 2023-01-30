// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0
//
use crate::{client::replies::reply_storage, config::DebugConfig};

pub fn setup_empty_reply_surb_backend(debug_config: &DebugConfig) -> reply_storage::Empty {
    reply_storage::Empty {
        min_surb_threshold: debug_config.minimum_reply_surb_storage_threshold,
        max_surb_threshold: debug_config.maximum_reply_surb_storage_threshold,
    }
}
