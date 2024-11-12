// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::ClientStatsEvents;

use nym_credentials_interface::TicketType;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize)]
pub(crate) struct CredentialStats {
    mixnet_entry_spent: u32,
    vpn_entry_spent: u32,
    mixnet_exit_spent: u32,
    vpn_exit_spent: u32,
}

/// Event space for Nym API statistics tracking
#[derive(Debug)]
pub enum CredentialStatsEvent {
    /// ecash ticket was spend
    TicketSpent { typ: TicketType, amount: u32 },
}

impl From<CredentialStatsEvent> for ClientStatsEvents {
    fn from(event: CredentialStatsEvent) -> ClientStatsEvents {
        ClientStatsEvents::Credential(event)
    }
}

/// Nym API statistics tracking object
#[derive(Default)]
pub struct CredentialStatsControl {
    // Keep track of packet statistics over time
    stats: CredentialStats,
}

impl CredentialStatsControl {
    pub(crate) fn handle_event(&mut self, event: CredentialStatsEvent) {
        match event {
            CredentialStatsEvent::TicketSpent { typ, amount } => match typ {
                TicketType::V1MixnetEntry => self.stats.mixnet_entry_spent += amount,
                TicketType::V1MixnetExit => self.stats.mixnet_exit_spent += amount,
                TicketType::V1WireguardEntry => self.stats.vpn_entry_spent += amount,
                TicketType::V1WireguardExit => self.stats.vpn_exit_spent += amount,
            },
        }
    }

    pub(crate) fn report(&self) -> CredentialStats {
        self.stats
    }
}
