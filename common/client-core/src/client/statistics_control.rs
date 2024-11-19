// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! # Statistics collection and reporting.
//!
//! Modular metrics collection and reporting system. submodules can be added to collect different types of metrics.
//! On creation the Statistics controller will start a task that will listen for incoming stats events and
//! multiplex them out to the appropriate metrics module based on type.
//!
//! Adding A new module you need to write a new module that implements the `StatsObj` trait and add it to
//! the `stats` hashmap in the `StatisticsControl` struct during it's initialization in the `new` function in
//! this file.

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::todo)]
#![warn(clippy::dbg_macro)]

use std::time::Duration;

use nym_client_core_config_types::StatsReporting;
use nym_sphinx::addressing::Recipient;
use nym_statistics_common::{clients::{
    ClientStatsController, ClientStatsReceiver, ClientStatsSender,
}, report::DataSender};
use nym_task::connections::TransmissionLane;

use crate::{
    client::inbound_messages::{InputMessage, InputMessageSender},
    spawn_future,
};

pub struct MixnetReporter {
    message_rx: DataSender,
    
    /// Channel to send stats report through the mixnet
    report_tx: InputMessageSender,

    /// Recipient of the reports sent over the mixnet.
    recipient: Recipient,
}

impl MixnetReporter {
    pub(crate) async fn run_with_shutdown(&mut self, task_client: TaskClient) {
        loop {
            tokio::select! {
                msg = self.message_rx.recv() => {
                    let report_message = InputMessage::new_regular(
                        self.recipient,
                        msg,
                        TransmissionLane::General,
                        None,
                    );
                    if let Err(err) = self.report_tx.send(report_message).await {
                        log::error!("Failed to report client stats: {:?}", err);
                    }
                },
                _ = task_client.recv_with_delay() => {
                    break;
                },
            }
        }
    }

    pub(crate) fn start_with_shutdown(mut self, task_client: TaskClient) {
        spawn_future(async move {
            self.run_with_shutdown(task_client).await;
        })
    }

    pub(crate) fn create_and_start_with_shutdown( report_tx: InputMessageSender, task_client: TaskClient) -> ClientStatsSender {
        let (controller, sender) = Self::create(report_tx);
        controller.start_with_shutdown(task_client);
        sender
    }
}