// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Statistics for the running node is collected by reporting events through the
//! `PacketEventReporter`, which is then saved in `SharedCurrentPacketEvents`. Periodically this is
//! merged into `SharedNodeStats`

use futures::channel::mpsc;
use std::time::Duration;

pub(crate) use node_stats::{NodeStats, NodeStatsSimple, SharedNodeStats};
pub(crate) use packet_event_reporter::PacketEventReporter;

mod console_logger;
mod node_stats;
mod packet_event_reporter;

/// Wire up and start the tasks used to collect, aggregate and log stats
pub struct NodeStatsTasks {
    /// Wrapper around channel sending information about new packet being received or sent
    event_reporter: packet_event_reporter::PacketEventReporter,

    /// Task responsible for handling data coming from `PacketEventReporter`
    event_handler: packet_event_reporter::PacketEventHandler,

    /// Pointer to the current node stats
    node_stats: node_stats::SharedNodeStats,

    /// Task responsible for updating stats at given interval
    stats_updater: node_stats::SharedStatsUpdater,

    /// Task responsible for logging stats to the console at given interval
    console_logger: console_logger::PacketStatsConsoleLogger,
}

impl NodeStatsTasks {
    pub fn new(logging_delay: Duration, stats_updating_delay: Duration) -> Self {
        // Channel to tie the PacketEventReporter and PacketEventHandler together
        let (sender, receiver) = mpsc::unbounded();

        // The packets events reported before it's collected into the node stats
        let shared_packet_events = packet_event_reporter::SharedCurrentPacketEvents::new();

        // Events are collected as statistics
        let shared_node_stats = node_stats::SharedNodeStats::new();

        NodeStatsTasks {
            event_reporter: PacketEventReporter::new(sender),
            event_handler: packet_event_reporter::PacketEventHandler::new(
                shared_packet_events.clone(),
                receiver,
            ),
            node_stats: shared_node_stats.clone(),
            stats_updater: node_stats::SharedStatsUpdater::new(
                stats_updating_delay,
                shared_packet_events,
                shared_node_stats.clone(),
            ),
            console_logger: console_logger::PacketStatsConsoleLogger::new(
                logging_delay,
                shared_node_stats,
            ),
        }
    }

    pub fn get_shared_node_stats(&self) -> node_stats::SharedNodeStats {
        self.node_stats.clone()
    }

    pub fn start(self) -> PacketEventReporter {
        // move out of self
        let mut event_handler = self.event_handler;
        let stats_updater = self.stats_updater;
        let mut console_logger = self.console_logger;

        tokio::spawn(async move { event_handler.run().await });
        tokio::spawn(async move { stats_updater.run().await });
        tokio::spawn(async move { console_logger.run().await });

        self.event_reporter
    }
}
