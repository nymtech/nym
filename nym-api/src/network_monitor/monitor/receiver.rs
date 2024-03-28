// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::network_monitor::gateways_reader::{GatewayMessages, GatewaysReader};
use crate::network_monitor::monitor::processor::ReceivedProcessorSender;
use futures::channel::mpsc;
use futures::StreamExt;
use nym_crypto::asymmetric::identity;
use nym_gateway_client::{AcknowledgementReceiver, MixnetMessageReceiver};
use nym_task::TaskClient;
use tracing::trace;

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
                    .add_receivers(id, message_receiver, ack_receiver);
            }
            GatewayClientUpdate::Failure(id) => {
                self.gateways_reader.remove_receivers(&id.to_string());
            }
        }
    }

    fn process_gateway_messages(&self, messages: GatewayMessages) {
        self.processor_sender
            .unbounded_send(messages)
            .expect("packet processor seems to have crashed!");
    }

    pub(crate) async fn run(&mut self, mut shutdown: TaskClient) {
        while !shutdown.is_shutdown() {
            tokio::select! {
                biased;
                _ = shutdown.recv() => {
                    trace!("UpdateHandler: Received shutdown");
                }
                // unwrap here is fine as it can only return a `None` if the PacketSender has died
                // and if that was the case, then the entire monitor is already in an undefined state
                update = self.clients_updater.next() => self.process_gateway_update(update.unwrap()),
                Some((_gateway_id, messages)) = self.gateways_reader.next() => {
                    self.process_gateway_messages(messages)
                }
            }
        }
    }
}
