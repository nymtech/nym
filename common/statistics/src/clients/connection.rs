// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::ClientStatsEvents;

use nym_credentials_interface::TicketType;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ConnectionStats {
    //tickets
    mixnet_entry_spent: u32,
    vpn_entry_spent: u32,
    mixnet_exit_spent: u32,
    vpn_exit_spent: u32,

    //country_connection
    wg_exit_country_code: String,
    mix_exit_country_code: String,
}

/// Event space for Nym API statistics tracking
#[derive(Debug, Clone)]
pub enum ConnectionStatsEvent {
    /// ecash ticket was spend
    TicketSpent {
        typ: TicketType,
        amount: u32,
    },
    WgCountry(String),
    MixCountry(String),
}
impl From<ConnectionStatsEvent> for ClientStatsEvents {
    fn from(event: ConnectionStatsEvent) -> ClientStatsEvents {
        ClientStatsEvents::Connection(event)
    }
}

/// Nym API statistics tracking object
#[derive(Default)]
pub struct ConnectionStatsControl {
    // Keep track of packet statistics over time
    stats: ConnectionStats,
}

impl ConnectionStatsControl {
    pub(crate) fn handle_event(&mut self, event: ConnectionStatsEvent) {
        match event {
            ConnectionStatsEvent::TicketSpent { typ, amount } => match typ {
                TicketType::V1MixnetEntry => self.stats.mixnet_entry_spent += amount,
                TicketType::V1MixnetExit => self.stats.mixnet_exit_spent += amount,
                TicketType::V1WireguardEntry => self.stats.vpn_entry_spent += amount,
                TicketType::V1WireguardExit => self.stats.vpn_exit_spent += amount,
            },
            ConnectionStatsEvent::WgCountry(cc) => {
                self.stats.wg_exit_country_code = cc;
            }
            ConnectionStatsEvent::MixCountry(cc) => {
                self.stats.mix_exit_country_code = cc;
            }
        }
    }

    pub(crate) fn report(&self) -> ConnectionStats {
        self.stats.clone()
    }
}
