// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::client_handling::websocket::message_receiver::{
    MixMessageReceiver, MixMessageSender,
};
use futures::StreamExt;
use nym_network_requester::{GatewayPacketRouter, PacketRouter};
use nym_service_providers_common::embedded::ProviderInputSender;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::addressing::ClientAddress;
use nym_sphinx::DestinationAddressBytes;
use nym_task::ShutdownToken;
use std::collections::HashMap;
use std::sync::mpsc::TrySendError;
use tracing::{debug, error, trace};

#[derive(Debug)]
pub struct LocalEmbeddedClientHandle {
    /// Nym address of the embedded client.
    pub(crate) address: Recipient,

    /// Message channel used internally to forward any received mix packets to the client.
    ///
    /// Carries **plaintext**: the gateway is the last sphinx hop on the legacy path, so it peels
    /// before handing anything over.
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

/// The service providers this node runs in-process, indexed the way the LP data plane addresses
/// them.
///
/// Which providers a node hosts is configuration, not something that comes and goes, so this is
/// built once at startup and never mutated - a plain map, with no locking on the packet path.
#[derive(Default)]
pub struct EmbeddedServiceProviders {
    by_address: HashMap<ClientAddress, ProviderInputSender>,
}

impl EmbeddedServiceProviders {
    #[cfg(test)]
    fn new(providers: HashMap<ClientAddress, ProviderInputSender>) -> Self {
        EmbeddedServiceProviders {
            by_address: providers,
        }
    }

    /// Index a provider under the address the LP data plane reaches it by.
    ///
    /// Covers the encryption key as well as the identity, unlike the legacy path's destination -
    /// neither derives from the other.
    pub fn insert(&mut self, address: Recipient, to_provider: ProviderInputSender) {
        let client = ClientAddress::from_keys(address.identity(), address.encryption_key());
        self.by_address.insert(client, to_provider);
    }

    /// Whether this node hosts `client` itself.
    ///
    /// Also the guard that stops the node putting its own provider's traffic on the wire: answer
    /// `false` and the packet falls through to the forwarding path, waits out the stall timeout
    /// against a peer it has no session with, and is dropped with a misleading reason.
    pub fn hosts(&self, client: ClientAddress) -> bool {
        self.by_address.contains_key(&client)
    }

    /// Hand a sphinx packet to the provider, which is the hop it is addressed to.
    pub fn deliver_sp_payload(
        &self,
        client: ClientAddress,
        packet: Vec<u8>,
    ) -> Result<(), SpDeliveryError> {
        let Some(sender) = self.by_address.get(&client) else {
            return Err(SpDeliveryError::NotHosted);
        };

        sender.try_send(packet).map_err(|err| match err {
            TrySendError::Full(_) => SpDeliveryError::Full,
            TrySendError::Disconnected(_) => SpDeliveryError::Gone,
        })
    }
}

/// Why a packet could not be handed to a provider this node hosts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpDeliveryError {
    /// This node hosts no provider at that address.
    NotHosted,

    /// The provider is behind: its queue is full, and the packet is dropped as it would be at a
    /// full socket.
    Full,

    /// The provider has stopped taking packets.
    Gone,
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

    fn handle_received_messages(&self, messages: Vec<Vec<u8>>) {
        if let Err(err) = self.packet_router.route_received(messages) {
            // TODO: what should we do here? I don't think this could/should ever fail.
            // is panicking the appropriate thing to do then?
            error!("failed to route packets to local embedded client: {err}")
        }
    }

    pub(crate) async fn run_with_shutdown(mut self, shutdown: ShutdownToken) {
        debug!("Started embedded client message router with graceful shutdown support");
        loop {
            tokio::select! {
                biased;
                 _ = shutdown.cancelled() => {
                    trace!("embedded_clients::MessageRouter: Received shutdown");
                    break;
                }
                messages = self.mix_receiver.next() => match messages {
                    Some(messages) => self.handle_received_messages(messages),
                    None => {
                        trace!("embedded_clients::MessageRouter: Stopping since channel closed");
                        break;
                    }
                },
            }
        }

        debug!("embedded_network_clients::MessageRouter: Exiting")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_service_providers_common::embedded::PROVIDER_CHANNEL_DEPTH;

    fn client(n: u8) -> ClientAddress {
        ClientAddress::from_bytes([n; 20])
    }

    /// A hosted provider takes the packet; anyone else is left for the caller to route onward.
    #[test]
    fn only_hosted_providers_are_delivered_to() {
        let (sender, receiver) = std::sync::mpsc::sync_channel(PROVIDER_CHANNEL_DEPTH);
        let providers = EmbeddedServiceProviders::new(HashMap::from([(client(1), sender)]));

        assert!(providers.hosts(client(1)));
        assert!(!providers.hosts(client(2)));

        assert_eq!(
            providers.deliver_sp_payload(client(1), b"mine".to_vec()),
            Ok(())
        );
        assert_eq!(
            providers.deliver_sp_payload(client(2), b"not mine".to_vec()),
            Err(SpDeliveryError::NotHosted),
            "a client we do not host must not be reported as delivered"
        );

        assert_eq!(receiver.try_recv().unwrap(), b"mine".to_vec());
    }

    /// A provider that has gone away reports the failure rather than swallowing the packet.
    ///
    /// It stays `hosts`, since the map is fixed at startup - so the caller has to distinguish
    /// "not ours, route it on" from "ours but unreachable, drop it".
    #[test]
    fn a_departed_provider_reports_failure() {
        let (sender, receiver) = std::sync::mpsc::sync_channel(PROVIDER_CHANNEL_DEPTH);
        let providers = EmbeddedServiceProviders::new(HashMap::from([(client(1), sender)]));

        drop(receiver);

        assert!(providers.hosts(client(1)));
        assert_eq!(
            providers.deliver_sp_payload(client(1), b"nobody home".to_vec()),
            Err(SpDeliveryError::Gone)
        );
    }

    /// A provider that is only behind is told apart from one that has gone.
    #[test]
    fn a_full_provider_is_not_reported_gone() {
        let (sender, _receiver) = std::sync::mpsc::sync_channel(1);
        let providers = EmbeddedServiceProviders::new(HashMap::from([(client(1), sender)]));

        assert_eq!(
            providers.deliver_sp_payload(client(1), b"fits".to_vec()),
            Ok(())
        );
        assert_eq!(
            providers.deliver_sp_payload(client(1), b"does not".to_vec()),
            Err(SpDeliveryError::Full)
        );
    }
}
