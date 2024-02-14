// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::{Client, Config, SendWithoutResponse};
use futures::channel::mpsc;
use futures::StreamExt;
use log::*;
use nym_crypto::asymmetric::encryption;
use nym_sphinx::forwarding::packet::MixPacket;
use nym_topology_control::accessor::TopologyAccessor;
use nym_validator_client::NymApiClient;
use std::sync::Arc;

pub type MixForwardingSender = mpsc::UnboundedSender<MixPacket>;
type MixForwardingReceiver = mpsc::UnboundedReceiver<MixPacket>;

/// A specialisation of client such that it forwards any received packets on the channel into the
/// mix network immediately, i.e. will not try to listen for any responses.
pub struct PacketForwarder {
    mixnet_client: Client,
    packet_receiver: MixForwardingReceiver,
    shutdown: nym_task::TaskClient,
}

impl PacketForwarder {
    pub fn new(
        client_config: Config,
        topology_access: TopologyAccessor,
        api_client: NymApiClient,
        local_identity: Arc<encryption::KeyPair>,
        shutdown: nym_task::TaskClient,
    ) -> (PacketForwarder, MixForwardingSender) {
        let (packet_sender, packet_receiver) = mpsc::unbounded();

        (
            PacketForwarder {
                mixnet_client: Client::new(
                    client_config,
                    topology_access,
                    api_client,
                    local_identity,
                ),
                packet_receiver,
                shutdown,
            },
            packet_sender,
        )
    }

    pub async fn run(&mut self) {
        while !self.shutdown.is_shutdown() {
            tokio::select! {
                biased;
                _ = self.shutdown.recv() => {
                    log::trace!("PacketForwarder: Received shutdown");
                }
                Some(mix_packet) = self.packet_receiver.next() => {
                     trace!("Going to forward packet to {}", mix_packet.next_hop());

                    let next_hop = mix_packet.next_hop();
                    let packet_type = mix_packet.packet_type();
                    let packet = mix_packet.into_packet();
                    // we don't care about responses, we just want to fire packets
                    // as quickly as possible

                    if let Err(err) =
                        self.mixnet_client
                            .send_without_response(next_hop, packet, packet_type)
                    {
                        debug!("failed to forward the packet - {err}")
                    }
                }
            }
        }
    }
}
