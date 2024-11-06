// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::report::StatisticsReporter;
use nym_task::spawn;

use tokio::sync::mpsc::UnboundedSender;

/// Active gateway connection statistics.
pub mod gateway_conn_statistics;

/// Nym API connection statistics.
pub mod nym_api_statistics;

/// Packet count based statistics.
pub mod packet_statistics;

/// Channel receiving generic stats events to be used by a statistics aggregator.
pub type ClientStatsReceiver = tokio::sync::mpsc::UnboundedReceiver<ClientStatsEvents>;

/// Channel allowing generic statistics events to be reported to a stats event aggregator
#[derive(Clone)]
pub struct ClientStatsSender {
    stats_tx: UnboundedSender<ClientStatsEvents>,
}

impl ClientStatsSender {
    /// Create a new statistics Sender
    pub fn new(stats_tx: UnboundedSender<ClientStatsEvents>) -> Self {
        ClientStatsSender { stats_tx }
    }

    /// Report a statistics event using the sender.
    pub fn report(&self, event: ClientStatsEvents) {
        if let Err(err) = self.stats_tx.send(event) {
            log::error!("Failed to send stats event: {:?}", err);
        }
    }

    /// Used when stats reporting is disabled -- reads all incoming messages and discards them
    pub fn sink(mut shutdown: nym_task::TaskClient) -> Self {
        let (stats_tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        spawn(async move { 
            loop {
                tokio::select! {
                    m = rx.recv() => {
                        if m.is_none() {
                            log::trace!("StatisticsSink: channel closed shutting down");
                            break;
                        }
                    },
                    _ = shutdown.recv_with_delay() => {
                        log::trace!("StatisticsSink: Received shutdown");
                        break;
                    },
                }
            }
            log::debug!("StatsSink: Exited");
        });
        Self { stats_tx }
    }
}

/// Identity types for Client statistics events allowing for triage of generic incoming stats events.
#[derive(PartialEq, Eq, Hash, Debug)]
pub enum ClientStatsType {
    /// Packet count events
    Packets,
    /// Gateway Connection events
    Gateway,
    /// Nym API connection events
    NymApi,
}

impl ClientStatsType {
    /// Return a string representation of the Stats Type
    pub fn as_str(&self) -> &'static str {
        match self {
            ClientStatsType::Packets => "packets",
            ClientStatsType::Gateway => "gateway_conn",
            ClientStatsType::NymApi => "nym_api",
        }
    }
}

/// Client Statistics events (static for now)
pub enum ClientStatsEvents {
    /// Packet count events
    PacketStatistics(packet_statistics::PacketStatisticsEvent),
    /// Gateway Connection events
    GatewayConn(gateway_conn_statistics::GatewayStatsEvent),
    /// Nym API connection events
    NymApi(nym_api_statistics::NymApiStatsEvent),
}

impl ClientStatsEvents {
    /// Returns the type identity of a client side statistics event
    pub fn metrics_type(&self) -> ClientStatsType {
        match self {
            ClientStatsEvents::PacketStatistics(_) => ClientStatsType::Packets,
            ClientStatsEvents::GatewayConn(_) => ClientStatsType::Gateway,
            ClientStatsEvents::NymApi(_) => ClientStatsType::NymApi,
        }
    }
}

/// Items implementing the StatsObj interface can be treated as generic statistics capture objects
/// to allow for decentralized event implementation, but centralized event cordination, sorage, management
/// and reporting.
pub trait ClientStatsObj: StatisticsReporter + Send {
    /// Returns a Statistics event type used to identify this event so it can be properly triaged.
    fn type_identity(&self) -> ClientStatsType;

    /// Handle an incoming stats event
    fn handle_event(&mut self, event: ClientStatsEvents);

    /// snapshot the current state of the metrics if the module wishes to use it
    fn snapshot(&mut self);

    /// Reset the metrics to their initial state.
    ///
    /// Used to periodically reset the metrics in accordance with periodic reporting strategy
    fn periodic_reset(&mut self);
}
