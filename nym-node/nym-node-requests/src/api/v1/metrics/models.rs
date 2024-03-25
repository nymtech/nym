// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct MixingStats {
    #[serde(with = "time::serde::rfc3339")]
    pub update_time: OffsetDateTime,

    #[serde(with = "time::serde::rfc3339")]
    pub previous_update_time: OffsetDateTime,

    pub received_since_startup: u64,

    // note: sent does not imply forwarded. We don't know if it was delivered successfully
    pub sent_since_startup: u64,

    // we know for sure we dropped those packets
    pub dropped_since_startup: u64,

    pub received_since_last_update: u64,

    // note: sent does not imply forwarded. We don't know if it was delivered successfully
    pub sent_since_last_update: u64,

    // we know for sure we dropped those packets
    pub dropped_since_last_update: u64,
}
