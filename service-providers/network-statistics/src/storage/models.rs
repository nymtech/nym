// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use sqlx::types::chrono::NaiveDateTime;

// Internally used struct to catch results from the database to get mixnet statistics
pub(crate) struct ServiceStatistics {
    #[allow(dead_code)]
    pub(crate) id: i64,
    pub(crate) requested_service: String,
    pub(crate) request_processed_bytes: i64,
    pub(crate) response_processed_bytes: i64,
    pub(crate) interval_seconds: i64,
    pub(crate) timestamp: NaiveDateTime,
}

pub(crate) struct GatewayStatistics {
    #[allow(dead_code)]
    pub(crate) id: i64,
    pub(crate) inbox_count: i64,
    pub(crate) timestamp: NaiveDateTime,
}
