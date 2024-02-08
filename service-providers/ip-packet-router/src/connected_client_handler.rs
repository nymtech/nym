// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_ip_packet_requests::response::IpPacketResponse;
use nym_sdk::mixnet::{MixnetMessageSender, Recipient};

use crate::{
    error::{IpPacketRouterError, Result},
    util::create_message::create_input_message,
};

// Data flow
// Out: mixnet_listener -> decode -> handle_packet -> write_to_tun
// In: tun_listener -> [connected_client_handler -> encode] -> mixnet_sender

// This handler is spawned as a task, and it listens to IP packets passed from the tun_listener,
// encodes it, and then sends to mixnet.
pub(crate) struct ConnectedClientHandler {
    nym_address: Recipient,
    mix_hops: Option<u8>,
    forward_from_tun_rx: tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
    mixnet_client_sender: nym_sdk::mixnet::MixnetClientSender,
    close_rx: tokio::sync::oneshot::Receiver<()>,
}

impl ConnectedClientHandler {
    pub(crate) fn launch(
        reply_to: Recipient,
        reply_to_hops: Option<u8>,
        mixnet_client_sender: nym_sdk::mixnet::MixnetClientSender,
    ) -> (
        tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
        tokio::sync::oneshot::Sender<()>,
    ) {
        let (close_tx, close_rx) = tokio::sync::oneshot::channel();
        let (forward_from_tun_tx, forward_from_tun_rx) = tokio::sync::mpsc::unbounded_channel();

        let connected_client_handler = ConnectedClientHandler {
            nym_address: reply_to,
            mix_hops: reply_to_hops,
            forward_from_tun_rx,
            mixnet_client_sender,
            close_rx,
        };

        tokio::spawn(async move {
            if let Err(err) = connected_client_handler.run().await {
                log::error!("connected client handler has failed: {err}")
            }
        });

        (forward_from_tun_tx, close_tx)
    }

    async fn handle_packet(&mut self, packet: Vec<u8>) -> Result<()> {
        let response_packet = IpPacketResponse::new_ip_packet(packet.into())
            .to_bytes()
            .map_err(|err| IpPacketRouterError::FailedToSerializeResponsePacket { source: err })?;
        let input_message = create_input_message(self.nym_address, response_packet, self.mix_hops);

        self.mixnet_client_sender
            .send(input_message)
            .await
            .map_err(|err| IpPacketRouterError::FailedToSendPacketToMixnet { source: err })
    }

    async fn run(mut self) -> Result<()> {
        loop {
            tokio::select! {
                _ = &mut self.close_rx => {
                    // WIP(JON): downgrade to trace once confirmed to work
                    log::warn!("ConnectedClientHandler: received shutdown");
                    break;
                },
                packet = self.forward_from_tun_rx.recv() => match packet {
                    Some(packet) => {
                        if let Err(err) = self.handle_packet(packet).await {
                            log::error!("connected client handler: failed to handle packet: {err}");
                        }
                    },
                    None => {
                        log::debug!("connected client handler: tun channel closed");
                        break;
                    }
                }
            }
        }

        // WIP(JON): downgrade to debug once confirmed to work
        log::warn!("ConnectedClientHandler: exiting");
        Ok(())
    }
}
