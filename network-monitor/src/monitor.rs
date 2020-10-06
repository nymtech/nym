// Copyright 2020 Nym Technologies SA
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

use crate::{notifications::Notifier, packet_sender::PacketSender};
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use log::*;
use tokio::time::{self, Duration};

pub(crate) type MixnetReceiver = UnboundedReceiver<Vec<Vec<u8>>>;
pub(crate) type MixnetSender = UnboundedSender<Vec<Vec<u8>>>;
pub(crate) type AckSender = UnboundedSender<Vec<Vec<u8>>>;

pub(crate) const MONITOR_RUN_INTERVAL: Duration = Duration::from_secs(60);
pub(crate) const NOTIFIER_DELIVERY_TIMEOUT: Duration = Duration::from_secs(20);

pub struct Monitor;

impl Monitor {
    pub fn new() -> Monitor {
        Monitor {}
    }

    pub(crate) async fn run(&mut self, mut notifier: Notifier, mut packet_sender: PacketSender) {
        println!("Network monitor running.");
        println!("--------------------------------------------------");
        tokio::spawn(async move {
            notifier.run().await;
        });

        tokio::spawn(async move {
            let mut interval = time::interval(MONITOR_RUN_INTERVAL);
            loop {
                interval.tick().await;
                info!(target: "Monitor", "Starting test run"); // TODO: nonce

                if let Err(err) = packet_sender.run_test().await {
                    error!("Test run failed! - {:?}", err);
                }
                // if let Err(err) = packet_sender.sanity_check().await {
                //     error!("failed sanity check... - {:?}", err);
                //     continue;
                // }
            }
        });

        self.wait_for_interrupt().await
    }

    async fn wait_for_interrupt(&self) {
        if let Err(e) = tokio::signal::ctrl_c().await {
            error!(
                "There was an error while capturing SIGINT - {:?}. We will terminate regardless",
                e
            );
        }
        println!("Received SIGINT - the network monitor will terminate now");
    }
}
