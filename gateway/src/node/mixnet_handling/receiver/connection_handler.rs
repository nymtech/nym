// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::client_handling::active_clients::ActiveClientsStore;
use crate::node::client_handling::websocket::message_receiver::MixMessageSender;
use crate::node::mixnet_handling::receiver::packet_processing::PacketProcessor;
use crate::node::storage::error::StorageError;
use crate::node::storage::Storage;
use futures::StreamExt;
use log::*;
use mixnet_client::forwarder::MixForwardingSender;
use mixnode_common::packet_processor::processor::ProcessedFinalHop;
use nymsphinx::forwarding::packet::MixPacket;
use nymsphinx::framing::codec::SphinxCodec;
use nymsphinx::framing::packet::FramedSphinxPacket;
use nymsphinx::DestinationAddressBytes;
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

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
        }
    }
}

impl<St: Storage> ConnectionHandler<St> {
    pub(crate) fn new(
        packet_processor: PacketProcessor,
        storage: St,
        ack_sender: MixForwardingSender,
        active_clients_store: ActiveClientsStore,
    ) -> Self {
        ConnectionHandler {
            packet_processor,
            clients_store_cache: HashMap::new(),
            storage,
            active_clients_store,
            ack_sender,
        }
    }

    fn update_clients_store_cache_entry(&mut self, client_address: DestinationAddressBytes) {
        if let Some(client_sender) = self.active_clients_store.get(client_address) {
            self.clients_store_cache
                .insert(client_address, client_sender);
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
                sender_channel
                    .unbounded_send(vec![message])
                    // right now it's a "simpler" case here as we're only ever sending 1 message
                    // at the time, but the channel itself could accept arbitrary many messages at once
                    .map_err(|try_send_err| try_send_err.into_inner().pop().unwrap())
            }
        }
    }

    pub(crate) async fn store_processed_packet_payload(
        &self,
        client_address: DestinationAddressBytes,
        message: Vec<u8>,
    ) -> Result<(), StorageError> {
        debug!(
            "Storing received message for {} on the disk...",
            client_address
        );

        self.storage.store_message(client_address, message).await
    }

    fn forward_ack(&self, forward_ack: Option<MixPacket>, client_address: DestinationAddressBytes) {
        if let Some(forward_ack) = forward_ack {
            trace!(
                "Sending ack from packet for {} to {}",
                client_address,
                forward_ack.next_hop()
            );

            self.ack_sender.unbounded_send(forward_ack).unwrap();
        }
    }

    async fn handle_processed_packet(&mut self, processed_final_hop: ProcessedFinalHop) {
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
                Err(err) => error!("Failed to store client data - {}", err),
                Ok(_) => trace!("Stored packet for {}", client_address),
            },
            Ok(_) => trace!("Pushed received packet to {}", client_address),
        }

        // if we managed to either push message directly to the [online] client or store it at
        // its inbox, it means that it must exist at this gateway, hence we can send the
        // received ack back into the network
        self.forward_ack(forward_ack, client_address);
    }

    async fn handle_received_packet(&mut self, framed_sphinx_packet: FramedSphinxPacket) {
        //
        // TODO: here be replay attack detection - it will require similar key cache to the one in
        // packet processor for vpn packets,
        // question: can it also be per connection vs global?
        //

        let processed_final_hop = match self.packet_processor.process_received(framed_sphinx_packet)
        {
            Err(e) => {
                debug!("We failed to process received sphinx packet - {:?}", e);
                return;
            }
            Ok(processed_final_hop) => processed_final_hop,
        };

        self.handle_processed_packet(processed_final_hop).await
    }

    pub(crate) async fn handle_connection(mut self, conn: TcpStream, remote: SocketAddr) {
        debug!("Starting connection handler for {:?}", remote);
        let mut framed_conn = Framed::new(conn, SphinxCodec);
        while let Some(framed_sphinx_packet) = framed_conn.next().await {
            match framed_sphinx_packet {
                Ok(framed_sphinx_packet) => {
                    // TODO: benchmark spawning tokio task with full processing vs just processing it
                    // synchronously under higher load in single and multi-threaded situation.

                    // in theory we could process multiple sphinx packet from the same connection in parallel,
                    // but we already handle multiple concurrent connections so if anything, making
                    // that change would only slow things down
                    self.handle_received_packet(framed_sphinx_packet).await;
                }
                Err(err) => {
                    error!(
                        "The socket connection got corrupted with error: {:?}. Closing the socket",
                        err
                    );
                    return;
                }
            }
        }

        info!(
            "Closing connection from {:?}",
            framed_conn.into_inner().peer_addr()
        );
    }
}
