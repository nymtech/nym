// Copyright 2021 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::monitor::preparer::{PacketPreparer, PreparedPackets};
use crate::monitor::processor::ReceivedProcessor;
use crate::monitor::sender::PacketSender;
use crate::monitor::summary_producer::SummaryProducer;
use log::*;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::time::{delay_for, interval_at, Duration, Instant};
use validator_client::models::mixmining::BatchMixStatus;

pub(crate) mod preparer;
pub(crate) mod processor;
pub(crate) mod receiver;
pub(crate) mod sender;
pub(crate) mod summary_producer;

const PACKET_DELIVERY_TIMEOUT: Duration = Duration::from_secs(20);
const MONITOR_RUN_INTERVAL: Duration = Duration::from_secs(60);

pub(super) struct Monitor {
    nonce: u64,
    packet_preparer: PacketPreparer,
    packet_sender: PacketSender,
    received_processor: ReceivedProcessor,
    summary_producer: SummaryProducer,
    validator_client: Arc<validator_client::Client>,
}

impl Monitor {
    pub(super) fn new(
        packet_preparer: PacketPreparer,
        packet_sender: PacketSender,
        received_processor: ReceivedProcessor,
        summary_producer: SummaryProducer,
        validator_client: Arc<validator_client::Client>,
    ) -> Self {
        Monitor {
            nonce: 1,
            packet_preparer,
            packet_sender,
            received_processor,
            summary_producer,
            validator_client,
        }
    }

    // while it might have been cleaner to put this into a separate `Notifier` structure,
    // I don't see much point considering it's only a single, small, method
    async fn notify_validator(&self, status: BatchMixStatus) {
        if let Err(err) = self
            .validator_client
            .post_batch_mixmining_status(status)
            .await
        {
            warn!("Failed to send batch status to validator - {:?}", err)
        }
    }

    fn all_run_gateways(&self, prepared_packets: &PreparedPackets) -> HashSet<String> {
        prepared_packets
            .packets
            .iter()
            .map(|packets| packets.gateway_address().to_base58_string())
            .collect()
    }

    async fn test_run(&mut self) {
        info!(target: "Monitor", "Starting test run no. {}", self.nonce);

        debug!(target: "Monitor", "preparing mix packets to all nodes...");
        let prepared_packets = match self.packet_preparer.prepare_test_packets(self.nonce).await {
            Ok(packets) => packets,
            Err(err) => {
                error!("failed to create packets for the test run - {:?}", err);
                // TODO: return error?
                return;
            }
        };
        let all_gateways = self.all_run_gateways(&prepared_packets);

        self.received_processor.set_new_expected(self.nonce).await;

        debug!(target: "Monitor", "starting to send all the packets...");
        self.packet_sender
            .send_packets(prepared_packets.packets)
            .await;

        debug!(target: "Monitor", "sending is over, waiting for {:?} before checking what we received", PACKET_DELIVERY_TIMEOUT);

        // give the packets some time to traverse the network
        delay_for(PACKET_DELIVERY_TIMEOUT).await;

        let received = self.received_processor.return_received().await;

        let batch_status = self.summary_producer.produce_summary(
            prepared_packets.tested_nodes,
            received,
            prepared_packets.invalid_nodes,
            all_gateways,
        );

        self.notify_validator(batch_status).await;

        self.nonce += 1;
    }

    pub(crate) async fn run(&mut self) {
        let mut interval = interval_at(Instant::now(), MONITOR_RUN_INTERVAL);
        loop {
            // let run_deadline = delay_for(MONITOR_RUN_INTERVAL);
            interval.tick().await;
            self.test_run().await;
            // run_deadline.await;
        }
    }
}
