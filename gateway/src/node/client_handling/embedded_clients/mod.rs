// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::client_handling::websocket::message_receiver::{
    MixMessageReceiver, MixMessageSender,
};
use futures::StreamExt;
use nym_network_requester::{GatewayPacketRouter, PacketRouter};
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::DestinationAddressBytes;
use nym_task::TaskClient;
use tracing::{debug, error, trace};

#[derive(Debug)]
pub(crate) struct LocalEmbeddedClientHandle {
    /// Nym address of the embedded client.
    pub(crate) address: Recipient,

    /// Message channel used internally to forward any received mix packets to the client.
    pub(crate) mix_message_sender: MixMessageSender,
}

impl LocalEmbeddedClientHandle {
    pub(crate) fn new(address: Recipient, mix_message_sender: MixMessageSender) -> Self {
        Self {
            address,
            mix_message_sender,
        }
    }

    pub(crate) fn client_destination(&self) -> DestinationAddressBytes {
        self.address.identity().derive_destination_address()
    }
}

// we could have just passed a `PacketRouter` around instead of creating a dedicated task for
// calling the method. however, this would have caused slightly more complexity and more overhead
// (due to more data being copied to every [mix] connection)
//
/// task responsible for receiving messages for locally embedded clients from multiple mix
/// connections and forwarding them via the router. kinda equivalent of a client socket handler
pub(crate) struct MessageRouter {
    mix_receiver: MixMessageReceiver,
    packet_router: PacketRouter,
}

impl MessageRouter {
    pub(crate) fn new(mix_receiver: MixMessageReceiver, packet_router: PacketRouter) -> Self {
        Self {
            mix_receiver,
            packet_router,
        }
    }

    pub(crate) fn start_with_shutdown(self, shutdown: TaskClient) {
        tokio::spawn(self.run_with_shutdown(shutdown));
    }

    fn handle_received_messages(&self, messages: Vec<Vec<u8>>) {
        if let Err(err) = self.packet_router.route_received(messages) {
            // TODO: what should we do here? I don't think this could/should ever fail.
            // is panicking the appropriate thing to do then?
            error!("failed to route packets to local embedded client: {err}")
        }
    }

    pub(crate) async fn run_with_shutdown(mut self, mut shutdown: TaskClient) {
        debug!("Started embedded client message router with graceful shutdown support");
        while !shutdown.is_shutdown() {
            tokio::select! {
                messages = self.mix_receiver.next() => match messages {
                    Some(messages) => self.handle_received_messages(messages),
                    None => {
                        trace!("embedded_clients::MessageRouter: Stopping since channel closed");
                        break;
                    }
                },
                _ = shutdown.recv_with_delay() => {
                    trace!("embedded_clients::MessageRouter: Received shutdown");
                    debug_assert!(shutdown.is_shutdown());
                    break
                }
            }
        }

        debug!("embedded_network_clients::MessageRouter: Exiting")
    }
}
