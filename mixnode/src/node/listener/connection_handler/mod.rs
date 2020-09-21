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

use crate::node::listener::connection_handler::packet_processing::{
    MixProcessingResult, PacketProcessor,
};
use dashmap::DashMap;
use futures::channel::mpsc;
use log::*;
use nymsphinx::addressing::nodes::NymNodeRoutingAddress;
use nymsphinx::framing::SphinxCodec;
use nymsphinx::{header::keys::RoutingKeys, SharedSecret, SphinxPacket};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::stream::StreamExt;
use tokio_util::codec::Framed;

pub(crate) mod packet_forwarding;
pub(crate) mod packet_processing;

pub(crate) type CachedKeys = (Option<SharedSecret>, RoutingKeys);

pub(crate) struct ConnectionHandler {
    packet_processor: PacketProcessor,
    // TODO: TYPE ALIAS FOR THIS GUY (or at least for the tuple inside)
    forwarding_channel: mpsc::UnboundedSender<(NymNodeRoutingAddress, SphinxPacket)>,

    // TODO: method for cache invalidation so that we wouldn't keep all keys for all eternity
    // we could use our friend DelayQueue. One of tokio's examples is literally using it for
    // cache invalidation: https://docs.rs/tokio/0.2.22/tokio/time/struct.DelayQueue.html
    vpn_key_cache: DashMap<SharedSecret, CachedKeys>,
    // vpn_key_cache: HashMap<SharedSecret, CachedKeys>,
}

impl ConnectionHandler {
    pub(crate) fn new(
        packet_processor: PacketProcessor,
        forwarding_channel: mpsc::UnboundedSender<(NymNodeRoutingAddress, SphinxPacket)>,
    ) -> Self {
        ConnectionHandler {
            packet_processor,
            forwarding_channel,
            vpn_key_cache: DashMap::new(),
        }
    }

    pub(crate) fn clone_without_cache(&self) -> Self {
        ConnectionHandler {
            packet_processor: self.packet_processor.clone(),
            forwarding_channel: self.forwarding_channel.clone(),
            vpn_key_cache: DashMap::new(),
        }
    }

    async fn handle_received_packet(self: Arc<Self>, sphinx_packet: SphinxPacket) {
        let shared_secret = sphinx_packet.shared_secret();
        //
        // TODO: here be replay attack detection - it will require similar key cache,
        // question: can it also be per connection vs global?
        //

        let pre_processed_packet = if let Some(cached_keys) = self.vpn_key_cache.get(&shared_secret)
        {
            match self
                .packet_processor
                .perform_initial_processing_with_cached_keys(sphinx_packet, cached_keys.value())
            {
                Err(e) => {
                    debug!("We failed to process received sphinx packet - {:?}", e);
                    return;
                }
                Ok(processed_packet) => processed_packet,
            }
        } else {
            let processed_packet = match self
                .packet_processor
                .perform_initial_processing(sphinx_packet)
            {
                Err(e) => {
                    debug!("We failed to process received sphinx packet - {:?}", e);
                    return;
                }
                Ok(processed_packet) => processed_packet,
            };

            // TODO:
            // TODO:
            // TODO: THIS CHECK IS A BAD ONE AS IT WILL NOT WORK IF MIX IS A FINAL HOP (so a gateway)
            // will be replaced by changing framing and passing some metadata along (somehow...)
            if self.packet_processor.is_vpn_packet(&processed_packet) {
                let new_shared_secret = processed_packet.shared_secret();
                let routing_keys = self.packet_processor.recompute_routing_keys(&shared_secret);
                self.vpn_key_cache
                    .insert(shared_secret, (new_shared_secret, routing_keys));
            }
            processed_packet
        };

        // all processing incl. delay was done, the only thing left is to forward it
        match self
            .packet_processor
            .perform_final_processing(pre_processed_packet)
            .await
        {
            Err(e) => debug!("We failed to process received sphinx packet - {:?}", e),
            Ok(res) => match res {
                MixProcessingResult::ForwardHop(hop_address, forward_packet) => {
                    // send our data to tcp client for forwarding. If forwarding fails, then it fails,
                    // it's not like we can do anything about it
                    //
                    // in unbounded_send() failed it means that the receiver channel was disconnected
                    // and hence something weird must have happened without a way of recovering
                    self.forwarding_channel
                        .unbounded_send((hop_address, forward_packet))
                        .unwrap();
                    self.packet_processor.report_sent(hop_address);
                }
                MixProcessingResult::LoopMessage => {
                    warn!("Somehow processed a loop cover message that we haven't implemented yet!")
                }
            },
        }
    }

    pub(crate) async fn handle_connection(self, conn: TcpStream, remote: SocketAddr) {
        debug!("Starting connection handler for {:?}", remote);
        let this = Arc::new(self);
        let mut framed_conn = Framed::new(conn, SphinxCodec);
        while let Some(sphinx_packet) = framed_conn.next().await {
            match sphinx_packet {
                Ok(sphinx_packet) => {
                    // TODO: benchmark spawning tokio task with full processing vs just processing it
                    // synchronously (without delaying inside of course,
                    // delay could be moved to a per-connection DelayQueue. The delay queue future
                    // could automatically just forward packet that is done being delayed)
                    // under higher load in single and multi-threaded situation.
                    //
                    // My gut feeling is saying that we might get some nice performance boost
                    // with the the change
                    let this = Arc::clone(&this);
                    tokio::spawn(this.handle_received_packet(sphinx_packet));
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
