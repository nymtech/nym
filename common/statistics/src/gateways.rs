// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_credentials_interface::TicketType;
use nym_sphinx::DestinationAddressBytes;
use time::OffsetDateTime;

/// Channel for receiving incoming Stats events
pub type GatewayStatsReceiver = tokio::sync::mpsc::UnboundedReceiver<GatewayStatsEvent>;

/// Channel allowing for generic statistics events to be reported to a stats event aggregator.
#[derive(Clone)]
pub struct GatewayStatsReporter {
    stats_tx: tokio::sync::mpsc::UnboundedSender<GatewayStatsEvent>,
}

impl GatewayStatsReporter {
    /// Construct a new gateway statistics event reporter
    pub fn new(stats_tx: tokio::sync::mpsc::UnboundedSender<GatewayStatsEvent>) -> Self {
        Self { stats_tx }
    }

    /// Report a gateway statistivs event using the reporter
    pub fn report(&self, event: GatewayStatsEvent) {
        self.stats_tx.send(event).unwrap_or_else(|err| {
            log::error!("Failed to report gateway stat event : {:?}", err);
        });
    }
}

/// Gateway Statistics events
pub enum GatewayStatsEvent {
    /// Events in the lifecycle of an established client tunnel
    SessionStatsEvent(SessionEvent),
}

impl GatewayStatsEvent {
    /// A new session between this gateway and the client remote has successfully opened
    pub fn new_session_start(client: DestinationAddressBytes) -> GatewayStatsEvent {
        GatewayStatsEvent::SessionStatsEvent(SessionEvent::SessionStart {
            start_time: OffsetDateTime::now_utc(),
            client,
        })
    }

    /// An existing session with the client remote has ended
    pub fn new_session_stop(client: DestinationAddressBytes) -> GatewayStatsEvent {
        GatewayStatsEvent::SessionStatsEvent(SessionEvent::SessionStop {
            stop_time: OffsetDateTime::now_utc(),
            client,
        })
    }

    /// A new ecash ticket has been added / requested
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

/// Events in the lifecycle of an established client tunnel
pub enum SessionEvent {
    /// A new session between this gateway and the client remote has successfully opened
    SessionStart {
        /// The timestamp of the session open event
        start_time: OffsetDateTime,
        /// Address of the remote client opening the connection
        client: DestinationAddressBytes,
    },
    /// An existing session with the client remote has ended
    SessionStop {
        /// Timestamp of the session end event
        stop_time: OffsetDateTime,
        /// Address of the remote client opening the connection
        client: DestinationAddressBytes,
    },
    /// A new ecash ticket has been added / requested
    EcashTicket {
        /// Type of ecash ticket that has been created as part of the session
        ticket_type: TicketType,
        /// Address of the remote client opening the connection
        client: DestinationAddressBytes,
    },
}
