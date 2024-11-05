// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::time::Duration;

use events::StatisticsEvent;
use nym_sphinx::addressing::Recipient;
use nym_task::connections::TransmissionLane;

use crate::spawn_future;

use super::inbound_messages::{InputMessage, InputMessageSender};

pub(crate) mod events;

// Time interval between reporting statistics
const STATS_REPORT_INTERVAL_SECS: u64 = 300;

type StatisticsReceiver = tokio::sync::mpsc::UnboundedReceiver<StatisticsEvent>;

#[derive(Clone)]
pub struct StatisticsReporter {
    stats_tx: tokio::sync::mpsc::UnboundedSender<StatisticsEvent>,
}

impl StatisticsReporter {
    pub(crate) fn new(stats_tx: tokio::sync::mpsc::UnboundedSender<StatisticsEvent>) -> Self {
        Self { stats_tx }
    }

    pub(crate) fn report(&self, event: StatisticsEvent) {
        self.stats_tx.send(event).unwrap_or_else(|err| {
            log::error!("Failed to report client stat event : {:?}", err);
        });
    }
}

pub(crate) struct StatisticsControl {
    // Incoming stats events from other tasks
    stats_rx: StatisticsReceiver,

    //service-provider address to send stats reports
    reporting_address: Recipient,

    //channel to send stats report through the mixnet
    report_tx: InputMessageSender,
}

impl StatisticsControl {
    pub(crate) fn new(
        reporting_address: Recipient,
        report_tx: InputMessageSender,
    ) -> (Self, StatisticsReporter) {
        let (stats_tx, stats_rx) = tokio::sync::mpsc::unbounded_channel();
        (
            StatisticsControl {
                stats_rx,
                reporting_address,
                report_tx,
            },
            StatisticsReporter::new(stats_tx),
        )
    }

    fn handle_event(&mut self, event: StatisticsEvent) {
        todo!()
    }
    async fn report_stats(&self) {
        let report_message = InputMessage::new_regular(
            self.reporting_address,
            "StatsReport".as_bytes().to_vec(),
            TransmissionLane::General,
            None,
        );
        self.report_tx
            .send(report_message)
            .await
            .unwrap_or_else(|err| log::error!("Failed to report client stat: {:?}", err));
    }

    pub(crate) async fn run_with_shutdown(&mut self, mut shutdown: nym_task::TaskClient) {
        log::debug!("Started StatisticsControl with graceful shutdown support");

        let report_interval = Duration::from_secs(STATS_REPORT_INTERVAL_SECS);
        let mut report_interval = tokio::time::interval(report_interval);

        loop {
            tokio::select! {
                stats_event = self.stats_rx.recv() => match stats_event {
                    Some(stats_event) => {
                        log::trace!("StatisticsControl: Received stats event");
                        self.handle_event(stats_event);
                    },
                    None => {
                        log::trace!("StatisticsControl: stopping since stats channel was closed");
                        break;
                    }
                },
                _ = report_interval.tick() => {
                    self.report_stats().await;
                }
                _ = shutdown.recv_with_delay() => {
                    log::trace!("StatisticsControl: Received shutdown");
                    break;
                },
            }
        }
        log::debug!("StatisticsControl: Exiting");
    }

    pub(crate) fn start_with_shutdown(mut self, task_client: nym_task::TaskClient) {
        spawn_future(async move {
            self.run_with_shutdown(task_client).await;
        })
    }
}
