// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct BandwidthControllerConfig {
    // How often the controller proactively checks whether any ticket type needs restocking.
    pub topup_interval: Duration,
    // Threshold to determine if a ticket is soon expired
    pub soon_expiry_threshold: Duration,

    // If we go below this threshold, we should request more tickets
    pub nb_ticket_restock: u64,

    // If we go below this threshold, we should request more tickets
    pub min_nb_ticket_needed: u64,
}

impl Default for BandwidthControllerConfig {
    fn default() -> Self {
        Self {
            topup_interval: Duration::from_secs(3 * 3600), // 3 hours,
            soon_expiry_threshold: Duration::from_secs(12 * 3600), // 12 hours,
            nb_ticket_restock: 20,
            min_nb_ticket_needed: 5,
        }
    }
}
