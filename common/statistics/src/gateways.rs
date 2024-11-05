// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_credentials_interface::TicketType;
use nym_sphinx::DestinationAddressBytes;
use time::OffsetDateTime;

pub type GatewayStatsReceiver = tokio::sync::mpsc::UnboundedReceiver<GatewayStatsEvent>;

#[derive(Clone)]
pub struct GatewayStatsReporter {
    stats_tx: tokio::sync::mpsc::UnboundedSender<GatewayStatsEvent>,
}

impl GatewayStatsReporter {
    pub fn new(stats_tx: tokio::sync::mpsc::UnboundedSender<GatewayStatsEvent>) -> Self {
        Self { stats_tx }
    }

    pub fn report(&self, event: GatewayStatsEvent) {
        self.stats_tx.send(event).unwrap_or_else(|err| {
            log::error!("Failed to report gateway stat event : {:?}", err);
        });
    }
}
pub enum GatewayStatsEvent {
    SessionStatsEvent(SessionEvent),
}

impl GatewayStatsEvent {
    pub fn new_session_start(client: DestinationAddressBytes) -> GatewayStatsEvent {
        GatewayStatsEvent::SessionStatsEvent(SessionEvent::SessionStart {
            start_time: OffsetDateTime::now_utc(),
            client,
        })
    }

    pub fn new_session_stop(client: DestinationAddressBytes) -> GatewayStatsEvent {
        GatewayStatsEvent::SessionStatsEvent(SessionEvent::SessionStop {
            stop_time: OffsetDateTime::now_utc(),
            client,
        })
    }

    pub fn new_ecash_ticket(
        client: DestinationAddressBytes,
        ticket_type: TicketType,
    ) -> GatewayStatsEvent {
        GatewayStatsEvent::SessionStatsEvent(SessionEvent::EcashTicket {
            ticket_type,
            client,
        })
    }
}

pub enum SessionEvent {
    SessionStart {
        start_time: OffsetDateTime,
        client: DestinationAddressBytes,
    },
    SessionStop {
        stop_time: OffsetDateTime,
        client: DestinationAddressBytes,
    },
    EcashTicket {
        ticket_type: TicketType,
        client: DestinationAddressBytes,
    },
}
