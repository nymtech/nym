// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use nym_credentials_interface::TicketType;

use crate::ticketbooks::AvailableTicketbooks;

#[derive(Debug, Clone)]
pub struct BandwidthControllerConfig {
    // How often the controller proactively checks whether any ticket type needs restocking.
    pub topup_interval: Duration,
    // Threshold to determine if a ticket is soon expired
    pub soon_expiry_threshold: Duration,

    // If we go below this threshold, we should request more tickets
    pub nb_ticket_restock: u64,

    // If we go below this threshold, we should request more tickets
    pub min_nb_ticket_needed: u64,

    // Ticket types the controller proactively restocks: the periodic sweep, the post-spend top-up,
    // and the restock triggered when a credential fetcher is installed. Defaults to every
    // non-mixnet-exit type. Set to an empty vec to disable proactive restocking entirely — a
    // credential fetcher can still be installed to serve on-demand / explicit fetches without the
    // controller ever depositing on its own.
    pub managed_ticket_types: Vec<TicketType>,
}

impl Default for BandwidthControllerConfig {
    fn default() -> Self {
        Self {
            topup_interval: Duration::from_secs(3 * 3600), // 3 hours,
            soon_expiry_threshold: Duration::from_secs(12 * 3600), // 12 hours,
            nb_ticket_restock: 20,
            min_nb_ticket_needed: 5,
            managed_ticket_types: AvailableTicketbooks::ticketbook_types(),
        }
    }
}
