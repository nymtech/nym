// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::network_monitor::gateways_reader::{GatewayMessages, GatewaysReader};
use crate::network_monitor::monitor::processor::ReceivedProcessorSender;
use crypto::asymmetric::identity;
use futures::channel::mpsc;
use futures::StreamExt;
use gateway_client::{AcknowledgementReceiver, MixnetMessageReceiver};

pub(crate) type GatewayClientUpdateSender = mpsc::UnboundedSender<GatewayClientUpdate>;
pub(crate) type GatewayClientUpdateReceiver = mpsc::UnboundedReceiver<GatewayClientUpdate>;

pub(crate) enum GatewayClientUpdate {
    Failure(identity::PublicKey),
    New(
        identity::PublicKey,
        (MixnetMessageReceiver, AcknowledgementReceiver),
    ),
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
            GatewayClientUpdate::New(id, (message_receiver, ack_receiver)) => {
                self.gateways_reader
                    .add_recievers(id, message_receiver, ack_receiver);
            }
            GatewayClientUpdate::Failure(id) => {
                self.gateways_reader.remove_recievers(&id.to_string());
            }
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
                Some((_gateway_id, message)) = self.gateways_reader.stream_map().next() => {
                        self.process_gateway_messages(message)
                }
            }
        }
    }
}
