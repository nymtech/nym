// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::report::ClientStatsReport;
use nym_task::spawn;

use time::OffsetDateTime;
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

        //let's not propagate a shutdown if we happen to error out while doing the blackhole
        shutdown.disarm();

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

/// Client Statistics events (static for now)
pub enum ClientStatsEvents {
    /// Packet count events
    PacketStatistics(packet_statistics::PacketStatisticsEvent),
    /// Gateway Connection events
    GatewayConn(gateway_conn_statistics::GatewayStatsEvent),
    /// Nym API connection events
    NymApi(nym_api_statistics::NymApiStatsEvent),
}

/// Controls stats event handling and reporting
pub struct ClientStatsController {
    //static infos
    last_update_time: OffsetDateTime,
    client_id: String,
    client_type: String,
    os_information: os_info::Info,

    // stats collection modules
    packet_stats: packet_statistics::PacketStatisticsControl,
    gateway_conn_stats: gateway_conn_statistics::GatewayStatsControl,
    nym_api_stats: nym_api_statistics::NymApiStatsControl,
}

impl ClientStatsController {
    /// Creates a ClientStatsController given a client_id
    pub fn new(client_id: String, client_type: String) -> Self {
        ClientStatsController {
            //Safety : 0 is always a valid number of seconds
            #[allow(clippy::unwrap_used)]
            last_update_time: OffsetDateTime::now_utc().replace_second(0).unwrap(), // allow a bigger anonymity set wrt
            client_id,
            client_type,
            os_information: os_info::get(),
            packet_stats: Default::default(),
            gateway_conn_stats: Default::default(),
            nym_api_stats: Default::default(),
        }
    }
    /// Returns a static ClientStatsReport that can be sent somewhere
    pub fn build_report(&self) -> ClientStatsReport {
        ClientStatsReport {
            last_update_time: self.last_update_time,
            client_id: self.client_id.clone(),
            client_type: self.client_type.clone(),
            os_information: self.os_information.clone(),
            packet_stats: self.packet_stats.report(),
            gateway_conn_stats: self.gateway_conn_stats.report(),
            nym_api_stats: self.nym_api_stats.report(),
        }
    }

    /// Handle and dispatch incoming stats event
    pub fn handle_event(&mut self, stats_event: ClientStatsEvents) {
        match stats_event {
            ClientStatsEvents::PacketStatistics(event) => self.packet_stats.handle_event(event),
            ClientStatsEvents::GatewayConn(event) => self.gateway_conn_stats.handle_event(event),
            ClientStatsEvents::NymApi(event) => self.nym_api_stats.handle_event(event),
        }
    }

    /// Reset the metrics to their initial state.
    ///
    /// Used to periodically reset the metrics in accordance with periodic reporting strategy
    pub fn reset(&mut self) {
        self.nym_api_stats = Default::default();
        self.gateway_conn_stats = Default::default();
        //no periodic reset for packet stats

        #[allow(clippy::unwrap_used)] //Safety : 0 is always a valid number of seconds
        let now = OffsetDateTime::now_utc().replace_second(0).unwrap();
        self.last_update_time = now;
    }

    /// snapshot the current state of the metrics for module that needs it
    pub fn snapshot(&mut self) {
        //no snapshot for gateway_conn_stats
        //no snapshot for nym_api_stats
        self.packet_stats.snapshot();
    }
}
