// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::report::{ClientStatsReport, LocalStatsReport, OsInformation};

use time::{OffsetDateTime, Time};
use tokio::sync::mpsc::UnboundedSender;

/// Active gateway connection statistics.
pub mod gateway_conn_statistics;

/// Nym API connection statistics.
pub mod nym_api_statistics;

/// Packet count based statistics.
pub mod packet_statistics;

pub mod connection;

/// Channel receiving generic stats events to be used by a statistics aggregator.
pub type ClientStatsReceiver = tokio::sync::mpsc::UnboundedReceiver<ClientStatsEvents>;

/// Channel allowing generic statistics events to be reported to a stats event aggregator
#[derive(Clone)]
pub struct ClientStatsSender {
    stats_tx: Option<UnboundedSender<ClientStatsEvents>>,
}

impl ClientStatsSender {
    /// Create a new statistics Sender
    pub fn new(stats_tx: Option<UnboundedSender<ClientStatsEvents>>) -> Self {
        ClientStatsSender { stats_tx }
    }

    /// Report a statistics event using the sender.
    pub fn report(&self, event: ClientStatsEvents) {
        if let Some(tx) = &self.stats_tx {
            if let Err(err) = tx.send(event) {
                log::error!("Failed to send stats event: {:?}", err);
            }
        }
    }
}

/// Client Statistics events (static for now)
#[derive(Clone)]
pub enum ClientStatsEvents {
    /// Packet count events
    PacketStatistics(packet_statistics::PacketStatisticsEvent),
    /// Gateway Connection events
    GatewayConn(gateway_conn_statistics::GatewayStatsEvent),
    /// Nym API connection events
    NymApi(nym_api_statistics::NymApiStatsEvent),
    /// Credential events
    Connection(connection::ConnectionStatsEvent),
}

/// Controls stats event handling and reporting
pub struct ClientStatsController {
    //static infos
    last_update_time: OffsetDateTime,
    client_id: String,
    client_type: String,
    os_information: OsInformation,

    stats: StatsModules,
}

impl ClientStatsController {
    /// Creates a ClientStatsController given a client_id
    pub fn new(client_id: String, client_type: String) -> Self {
        ClientStatsController {
            last_update_time: ClientStatsController::get_update_time(),
            client_id,
            client_type,
            os_information: OsInformation::new(),
            stats: Default::default(),
        }
    }

    /// Returns a static ClientStatsReport that can be sent somewhere
    pub fn build_report(&self) -> ClientStatsReport {
        ClientStatsReport {
            last_update_time: self.last_update_time,
            client_id: self.client_id.clone(),
            client_type: self.client_type.clone(),
            os_information: self.os_information.clone(),
            packet_stats: self.stats.packet_stats.report(),
            gateway_conn_stats: self.stats.gateway_conn_stats.report(),
            nym_api_stats: self.stats.nym_api_stats.report(),
            connection_stats: self.stats.connection_stats.report(),
            ..Default::default()
        }
    }

    /// Handle and dispatch incoming stats event
    pub fn handle_event(&mut self, stats_event: ClientStatsEvents) {
        self.stats.handle_event(stats_event);
    }

    /// Reset the metrics to their initial state.
    ///
    /// Used to periodically reset the metrics in accordance with periodic reporting strategy
    pub fn reset(&mut self) {
        self.stats.reset();
        self.last_update_time = ClientStatsController::get_update_time();
    }

    /// snapshot the current state of the metrics for module that needs it
    pub fn snapshot(&mut self) {
        self.stats.snapshot();
    }

    fn get_update_time() -> OffsetDateTime {
        let now = OffsetDateTime::now_utc();
        #[allow(clippy::unwrap_used)]
        //Safety : 0 is always a valid number of seconds, hours and minutes comes from a valid source
        let new_time = Time::from_hms(now.hour(), now.minute(), 0).unwrap();
        //allows a bigger anonymity by hiding exact sending time
        now.replace_time(new_time)
    }
}

// Statistics collection modules
#[derive(Default)]
struct StatsModules {
    pub(crate) packet_stats: packet_statistics::PacketStatisticsControl,
    pub(crate) gateway_conn_stats: gateway_conn_statistics::GatewayStatsControl,
    pub(crate) nym_api_stats: nym_api_statistics::NymApiStatsControl,
    pub(crate) connection_stats: connection::ConnectionStatsControl,
}

impl StatsModules {
    /// Handle and dispatch incoming stats event
    fn handle_event(&mut self, stats_event: ClientStatsEvents) {
        match stats_event {
            ClientStatsEvents::PacketStatistics(event) => self.packet_stats.handle_event(event),
            ClientStatsEvents::GatewayConn(event) => self.gateway_conn_stats.handle_event(event),
            ClientStatsEvents::NymApi(event) => self.nym_api_stats.handle_event(event),
            ClientStatsEvents::Connection(event) => self.connection_stats.handle_event(event),
        }
    }

    /// Reset the metrics to their initial state.
    ///
    /// Used to periodically reset the metrics in accordance with periodic reporting strategy
    fn reset(&mut self) {
        self.nym_api_stats = Default::default();
        self.gateway_conn_stats = Default::default();
        self.connection_stats = Default::default();
        //no periodic reset for packet stats
    }

    /// snapshot the current state of the metrics for module that needs it
    fn snapshot(&mut self) {
        //no snapshot for gateway_conn_stats
        //no snapshot for nym_api_stats
        self.packet_stats.snapshot();
    }
}

/// Controls stats event handling and reporting
pub struct LocalStatsController {
    last_update_time: OffsetDateTime,
    stats: StatsModules,
}

impl Default for LocalStatsController {
    /// Creates a ClientStatsController given a client_id
    fn default() -> Self {
        Self {
            last_update_time: ClientStatsController::get_update_time(),
            stats: Default::default(),
        }
    }
}

impl LocalStatsController {
    /// Handle and dispatch incoming stats event
    pub fn handle_event(&mut self, stats_event: ClientStatsEvents) {
        self.stats.handle_event(stats_event);
    }

    /// Reset the metrics to their initial state.
    ///
    /// Used to periodically reset the metrics in accordance with periodic reporting strategy
    pub fn reset(&mut self) {
        self.stats.reset();
        self.last_update_time = ClientStatsController::get_update_time();
    }

    /// snapshot the current state of the metrics for module that needs it
    pub fn snapshot(&mut self) {
        self.stats.snapshot();
    }

    /// Returns a static ClientStatsReport that can be sent somewhere
    pub fn build_report(&self) -> LocalStatsReport {
        LocalStatsReport {
            packet_stats: self.stats.packet_stats.report(),
            gateway_conn_stats: self.stats.gateway_conn_stats.report(),
            nym_api_stats: self.stats.nym_api_stats.report(),
            connection_stats: self.stats.connection_stats.report(),
        }
    }

    fn get_update_time() -> OffsetDateTime {
        let now = OffsetDateTime::now_utc();
        #[allow(clippy::unwrap_used)]
        //Safety : 0 is always a valid number of seconds, hours and minutes comes from a valid source
        let new_time = Time::from_hms(now.hour(), now.minute(), 0).unwrap();
        //allows a bigger anonymity by hiding exact sending time
        now.replace_time(new_time)
    }
}
