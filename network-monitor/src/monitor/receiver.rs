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

use crate::gateways_reader::{GatewayChannel, GatewayMessages, GatewaysReader};
use crate::monitor::processor::ReceivedProcessorSender;
use crypto::asymmetric::identity;
use futures::channel::mpsc;
use futures::stream;
use gateway_client::MixnetMessageReceiver;
use tokio::stream::StreamExt;

pub(crate) type GatewayClientUpdateSender = mpsc::UnboundedSender<GatewayClientUpdate>;
pub(crate) type GatewayClientUpdateReceiver = mpsc::UnboundedReceiver<GatewayClientUpdate>;

pub(crate) enum GatewayClientUpdate {
    Failure(identity::PublicKey),
    New(identity::PublicKey, MixnetMessageReceiver),
}

pub(crate) struct PacketReceiver {
    gateways_reader: GatewaysReader,
    clients_updater: GatewayClientUpdateReceiver,
    processor_sender: ReceivedProcessorSender,
}

impl PacketReceiver {
    pub(crate) fn new(
        clients_updater: GatewayClientUpdateReceiver,
        processor_sender: ReceivedProcessorSender,
    ) -> Self {
        PacketReceiver {
            gateways_reader: GatewaysReader::new(),
            clients_updater,
            processor_sender,
        }
    }

    fn process_gateway_update(&mut self, update: GatewayClientUpdate) {
        match update {
            GatewayClientUpdate::New(id, receiver) => {
                let channel = GatewayChannel::new(id, receiver);
                self.gateways_reader.insert_channel(channel);
            }
            GatewayClientUpdate::Failure(id) => self.gateways_reader.remove_by_key(id),
        }
    }

    fn process_gateway_messages(&self, messages: GatewayMessages) {
        self.processor_sender
            .unbounded_send(messages)
            .expect("packet processor seems to have crashed!");
    }

    pub(crate) async fn run(&mut self) {
        loop {
            tokio::select! {
                // unwrap here is fine as it can only return a `None` if the PacketSender has died
                // and if that was the case, then the entire monitor is already in an undefined state
                update = self.clients_updater.next() => self.process_gateway_update(update.unwrap()),
                // similarly gateway reader will never return a `None` as it's implemented
                // as an infinite stream that returns Poll::Pending if it doesn't have anything
                // to return
                messages = self.gateways_reader.next() => self.process_gateway_messages(messages.unwrap()),
            }
        }
    }
}
