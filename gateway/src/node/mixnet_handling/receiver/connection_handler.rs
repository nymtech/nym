// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::client_handling::active_clients::ActiveClientsStore;
use crate::node::client_handling::websocket::message_receiver::MixMessageSender;
use crate::node::mixnet_handling::receiver::packet_processing::PacketProcessor;
use crate::node::storage::error::StorageError;
use crate::node::storage::Storage;
use futures::channel::mpsc::SendError;
use futures::StreamExt;
use log::*;
use nym_client_core::client::topology_control::accessor::TopologyAccessor;
use nym_crypto::asymmetric::encryption;
use nym_mixnet_client::forwarder::MixForwardingSender;
use nym_mixnode_common::packet_processor::processor::ProcessedFinalHop;
use nym_noise::upgrade_noise_responder_with_topology;
use nym_sphinx::forwarding::packet::MixPacket;
use nym_sphinx::framing::codec::NymCodec;
use nym_sphinx::framing::packet::FramedNymPacket;
use nym_sphinx::DestinationAddressBytes;
use nym_task::TaskClient;
use nym_validator_client::NymApiClient;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

// defines errors that warrant a panic if not thrown in the context of a shutdown
#[derive(Debug, Error)]
enum CriticalPacketProcessingError {
    #[error("failed to forward an ack")]
    AckForwardingFailure { source: SendError },
}

pub(crate) struct ConnectionHandler<St: Storage> {
    packet_processor: PacketProcessor,

    // TODO: investigate performance trade-offs for whether this cache even makes sense
    // at this point.
    // keep the following in mind: each action on ActiveClientsStore requires going through RwLock
    // and each `get` internally copies the channel, however, is it really that expensive?
    clients_store_cache: HashMap<DestinationAddressBytes, MixMessageSender>,
    active_clients_store: ActiveClientsStore,
    storage: St,
    ack_sender: MixForwardingSender,
    topology_access: TopologyAccessor,
    api_client: NymApiClient,
    local_identity: Arc<encryption::KeyPair>,
}

impl<St: Storage + Clone> Clone for ConnectionHandler<St> {
    fn clone(&self) -> Self {
        // remove stale entries from the cache while cloning
        let mut clients_store_cache = HashMap::with_capacity(self.clients_store_cache.capacity());
        for (k, v) in self.clients_store_cache.iter() {
            if !v.is_closed() {
                clients_store_cache.insert(*k, v.clone());
            }
        }

        ConnectionHandler {
            packet_processor: self.packet_processor.clone(),
            clients_store_cache,
            active_clients_store: self.active_clients_store.clone(),
            storage: self.storage.clone(),
            ack_sender: self.ack_sender.clone(),
            topology_access: self.topology_access.clone(),
            api_client: self.api_client.clone(),
            local_identity: self.local_identity.clone(),
        }
    }
}

impl<St: Storage> ConnectionHandler<St> {
    pub(crate) fn new(
        packet_processor: PacketProcessor,
        storage: St,
        ack_sender: MixForwardingSender,
        active_clients_store: ActiveClientsStore,
        topology_access: TopologyAccessor,
        api_client: NymApiClient,
        local_identity: Arc<encryption::KeyPair>,
    ) -> Self {
        ConnectionHandler {
            packet_processor,
            clients_store_cache: HashMap::new(),
            storage,
            active_clients_store,
            ack_sender,
            topology_access,
            api_client,
            local_identity,
        }
    }

    fn update_clients_store_cache_entry(&mut self, client_address: DestinationAddressBytes) {
        if let Some(client_senders) = self.active_clients_store.get_sender(client_address) {
            self.clients_store_cache
                .insert(client_address, client_senders);
        }
    }

    fn check_cache(&mut self, client_address: DestinationAddressBytes) {
        match self.clients_store_cache.get(&client_address) {
            None => self.update_clients_store_cache_entry(client_address),
            Some(entry) => {
                if entry.is_closed() {
                    self.update_clients_store_cache_entry(client_address)
                }
            }
        }
    }

    fn try_push_message_to_client(
        &mut self,
        client_address: DestinationAddressBytes,
        message: Vec<u8>,
    ) -> Result<(), Vec<u8>> {
        self.check_cache(client_address);

        match self.clients_store_cache.get(&client_address) {
            None => Err(message),
            Some(sender_channel) => {
                if let Err(unsent) = sender_channel.unbounded_send(vec![message]) {
                    // the unwrap here is fine as the original message got returned;
                    // plus we're only ever sending 1 message at the time (for now)
                    #[allow(clippy::unwrap_used)]
                    return Err(unsent.into_inner().pop().unwrap());
                } else {
                    Ok(())
                }
            }
        }
    }

    pub(crate) async fn store_processed_packet_payload(
        &self,
        client_address: DestinationAddressBytes,
        message: Vec<u8>,
    ) -> Result<(), StorageError> {
        debug!("Storing received message for {client_address} on the disk...",);

        self.storage.store_message(client_address, message).await
    }

    fn forward_ack(
        &self,
        forward_ack: Option<MixPacket>,
        client_address: DestinationAddressBytes,
    ) -> Result<(), CriticalPacketProcessingError> {
        if let Some(forward_ack) = forward_ack {
            let next_hop = forward_ack.next_hop();
            trace!("Sending ack from packet for {client_address} to {next_hop}",);

            self.ack_sender
                .unbounded_send(forward_ack)
                .map_err(
                    |source| CriticalPacketProcessingError::AckForwardingFailure {
                        source: source.into_send_error(),
                    },
                )?;
        }
        Ok(())
    }

    async fn handle_processed_packet(
        &mut self,
        processed_final_hop: ProcessedFinalHop,
    ) -> Result<(), CriticalPacketProcessingError> {
        let client_address = processed_final_hop.destination;
        let message = processed_final_hop.message;
        let forward_ack = processed_final_hop.forward_ack;

        // we failed to push message directly to the client - it's probably offline.
        // we should store it on the disk instead.
        match self.try_push_message_to_client(client_address, message) {
            Err(unsent_plaintext) => match self
                .store_processed_packet_payload(client_address, unsent_plaintext)
                .await
            {
                Err(err) => error!("Failed to store client data - {err}"),
                Ok(_) => trace!("Stored packet for {client_address}"),
            },
            Ok(_) => trace!("Pushed received packet to {client_address}"),
        }

        // if we managed to either push message directly to the [online] client or store it at
        // its inbox, it means that it must exist at this gateway, hence we can send the
        // received ack back into the network
        self.forward_ack(forward_ack, client_address)
    }

    async fn handle_received_packet(
        &mut self,
        framed_sphinx_packet: FramedNymPacket,
    ) -> Result<(), CriticalPacketProcessingError> {
        //
        // TODO: here be replay attack detection - it will require similar key cache to the one in
        // packet processor for vpn packets,
        // question: can it also be per connection vs global?
        //

        let processed_final_hop = match self.packet_processor.process_received(framed_sphinx_packet)
        {
            Err(err) => {
                debug!("We failed to process received sphinx packet - {err}");
                return Ok(());
            }
            Ok(processed_final_hop) => processed_final_hop,
        };

        self.handle_processed_packet(processed_final_hop).await
    }

    pub(crate) async fn handle_connection(
        mut self,
        conn: TcpStream,
        remote: SocketAddr,
        mut shutdown: TaskClient,
    ) {
        debug!("Starting connection handler for {:?}", remote);
        shutdown.mark_as_success();

        let Some(topology) = self.topology_access.current_topology().await else {
            error!("Cannot perform Noise handshake to {remote}, due to topology error");
            return;
        };

        let epoch_id = match self.api_client.get_current_epoch_id().await {
            Ok(id) => id,
            Err(err) => {
                error!("Cannot perform Noise handshake to {remote}, due to epoch id error - {err}");
                return;
            }
        };

        let noise_stream = match upgrade_noise_responder_with_topology(
            conn,
            Default::default(),
            &topology,
            epoch_id,
            self.local_identity.public_key(),
            self.local_identity.private_key(),
        )
        .await
        {
            Ok(noise_stream) => noise_stream,
            Err(err) => {
                error!("Failed to perform Noise handshake with {remote} - {err}");
                return;
            }
        };
        debug!("Noise responder handshake completed for {:?}", remote);
        let mut framed_conn = Framed::new(noise_stream, NymCodec);
        while !shutdown.is_shutdown() {
            tokio::select! {
                biased;
                _ = shutdown.recv() => {
                    log::trace!("ConnectionHandler: received shutdown");
                }
                framed_sphinx_packet = framed_conn.next() => {
                    match framed_sphinx_packet {
                        Some(Ok(framed_sphinx_packet)) => {
                            // TODO: benchmark spawning tokio task with full processing vs just processing it
                            // synchronously under higher load in single and multi-threaded situation.

                            // in theory we could process multiple sphinx packet from the same connection in parallel,
                            // but we already handle multiple concurrent connections so if anything, making
                            // that change would only slow things down
                            if let Err(critical_err) = self.handle_received_packet(framed_sphinx_packet).await {
                                if !shutdown.is_shutdown() {
                                    panic!("experienced critical failure when processing received packet: {critical_err}")
                                }
                            }
                        }
                        Some(Err(err)) => {
                            error!(
                                "The socket connection got corrupted with error: {err}. Closing the socket",
                            );
                            return;
                        }
                        None => break, // stream got closed by remote
                    }
                }
            }
        }

        match framed_conn.into_inner().peer_addr() {
            Ok(peer_addr) => {
                debug!("closing connection from {peer_addr}")
            }
            Err(err) => {
                warn!("closing connection from an unknown peer: {err}")
            }
        }
    }
}
