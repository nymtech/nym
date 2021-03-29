// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::client_handling::clients_handler::{
    ClientsHandlerRequest, ClientsHandlerRequestSender, ClientsHandlerResponse,
};
use crate::node::client_handling::websocket::message_receiver::MixMessageSender;
use crate::node::mixnet_handling::receiver::packet_processing::PacketProcessor;
use crate::node::storage::inboxes::{ClientStorage, StoreData};
use dashmap::DashMap;
use futures::channel::oneshot;
use futures::StreamExt;
use log::*;
use mixnet_client::forwarder::MixForwardingSender;
use mixnode_common::cached_packet_processor::processor::ProcessedFinalHop;
use nymsphinx::forwarding::packet::MixPacket;
use nymsphinx::framing::codec::SphinxCodec;
use nymsphinx::framing::packet::FramedSphinxPacket;
use nymsphinx::DestinationAddressBytes;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

pub(crate) struct ConnectionHandler {
    packet_processor: PacketProcessor,

    // TODO: method for cache invalidation so that we wouldn't keep all stale channel references
    // we could use our friend DelayQueue. Alternatively we could periodically check for if the
    // channels are closed.
    available_socket_senders_cache: DashMap<DestinationAddressBytes, MixMessageSender>,
    client_store: ClientStorage,
    clients_handler_sender: ClientsHandlerRequestSender,
    ack_sender: MixForwardingSender,
}

impl ConnectionHandler {
    pub(crate) fn new(
        packet_processor: PacketProcessor,
        clients_handler_sender: ClientsHandlerRequestSender,
        client_store: ClientStorage,

        ack_sender: MixForwardingSender,
    ) -> Self {
        ConnectionHandler {
            packet_processor,
            available_socket_senders_cache: DashMap::new(),
            client_store,
            clients_handler_sender,
            ack_sender,
        }
    }

    pub(crate) fn clone_without_cache(&self) -> Self {
        // TODO: should this be even cloned?
        let senders_cache = DashMap::with_capacity(self.available_socket_senders_cache.capacity());
        for element_guard in self.available_socket_senders_cache.iter() {
            let (k, v) = element_guard.pair();
            // TODO: this will be made redundant once there's some cache invalidator mechanism here
            if !v.is_closed() {
                senders_cache.insert(*k, v.clone());
            }
        }

        ConnectionHandler {
            packet_processor: self.packet_processor.clone_without_key_cache(),
            available_socket_senders_cache: senders_cache,
            client_store: self.client_store.clone(),
            clients_handler_sender: self.clients_handler_sender.clone(),
            ack_sender: self.ack_sender.clone(),
        }
    }

    fn try_push_message_to_client(
        &self,
        sender_channel: Option<MixMessageSender>,
        message: Vec<u8>,
    ) -> Result<(), Vec<u8>> {
        match sender_channel {
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

    fn remove_stale_client_sender(&self, client_address: &DestinationAddressBytes) {
        if self
            .available_socket_senders_cache
            .remove(client_address)
            .is_none()
        {
            warn!(
                "Tried to remove stale entry for non-existent client sender: {}",
                client_address
            )
        }
    }

    async fn try_to_obtain_client_ws_message_sender(
        &self,
        client_address: DestinationAddressBytes,
    ) -> Option<MixMessageSender> {
        let mut should_remove_stale = false;
        if let Some(sender_ref) = self.available_socket_senders_cache.get(&client_address) {
            let sender = sender_ref.value();
            if !sender.is_closed() {
                return Some(sender.clone());
            } else {
                should_remove_stale = true;
            }
        }

        // we want to do it outside the immutable borrow into the map
        if should_remove_stale {
            self.remove_stale_client_sender(&client_address)
        }

        // if we got here it means that either we have no sender channel for this client or it's closed
        // so we must refresh it from the source, i.e. ClientsHandler
        let (res_sender, res_receiver) = oneshot::channel();
        let clients_handler_request = ClientsHandlerRequest::IsOnline(client_address, res_sender);
        self.clients_handler_sender
            .unbounded_send(clients_handler_request)
            .unwrap(); // the receiver MUST BE alive

        let client_sender = match res_receiver.await.unwrap() {
            ClientsHandlerResponse::IsOnline(client_sender) => client_sender,
            _ => panic!("received response to wrong query!"), // again, this should NEVER happen
        }?;

        // finally update the cache
        if self
            .available_socket_senders_cache
            .insert(client_address, client_sender.clone())
            .is_some()
        {
            // this warning is harmless, but I want to see if it's realistically for it to even occur
            warn!("Other thread already updated cache for client sender!")
        }

        Some(client_sender)
    }

    pub(crate) async fn store_processed_packet_payload(
        &self,
        client_address: DestinationAddressBytes,
        message: Vec<u8>,
    ) -> io::Result<()> {
        debug!(
            "Storing received message for {} on the disk...",
            client_address
        );

        let store_data = StoreData::new(client_address, message);
        self.client_store.store_processed_data(store_data).await
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

    async fn handle_processed_packet(&self, processed_final_hop: ProcessedFinalHop) {
        let client_address = processed_final_hop.destination;
        let message = processed_final_hop.message;
        let forward_ack = processed_final_hop.forward_ack;

        let client_sender = self
            .try_to_obtain_client_ws_message_sender(client_address)
            .await;

        // we failed to push message directly to the client - it's probably offline.
        // we should store it on the disk instead.
        match self.try_push_message_to_client(client_sender, message) {
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

    async fn handle_received_packet(self: Arc<Self>, framed_sphinx_packet: FramedSphinxPacket) {
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

    pub(crate) async fn handle_connection(self, conn: TcpStream, remote: SocketAddr) {
        debug!("Starting connection handler for {:?}", remote);
        let this = Arc::new(self);
        let mut framed_conn = Framed::new(conn, SphinxCodec);
        while let Some(framed_sphinx_packet) = framed_conn.next().await {
            match framed_sphinx_packet {
                Ok(framed_sphinx_packet) => {
                    // TODO: benchmark spawning tokio task with full processing vs just processing it
                    // synchronously (without delaying inside of course,
                    // delay could be moved to a per-connection DelayQueue. The delay queue future
                    // could automatically just forward packet that is done being delayed)
                    // under higher load in single and multi-threaded situation.
                    //
                    // My gut feeling is saying that we might get some nice performance boost
                    // if we introduced the change
                    let this = Arc::clone(&this);
                    tokio::spawn(this.handle_received_packet(framed_sphinx_packet));
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
