// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::time::Duration;

use nym_ip_packet_requests::response::IpPacketResponse;
use nym_sdk::mixnet::{MixnetMessageSender, Recipient};

use crate::{
    error::{IpPacketRouterError, Result},
    util::create_message::create_input_message,
};

const ACTIVITY_TIMEOUT_SEC: u64 = 15 * 60;

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
    finished_tx: Option<tokio::sync::oneshot::Sender<()>>,
    activity_timeout: tokio::time::Interval,
}

impl ConnectedClientHandler {
    pub(crate) fn start(
        reply_to: Recipient,
        reply_to_hops: Option<u8>,
        mixnet_client_sender: nym_sdk::mixnet::MixnetClientSender,
    ) -> (
        tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
        tokio::sync::oneshot::Sender<()>,
        tokio::sync::oneshot::Receiver<()>,
    ) {
        let (close_tx, close_rx) = tokio::sync::oneshot::channel();
        let (finished_tx, finished_rx) = tokio::sync::oneshot::channel();
        let (forward_from_tun_tx, forward_from_tun_rx) = tokio::sync::mpsc::unbounded_channel();

        let connected_client_handler = ConnectedClientHandler {
            nym_address: reply_to,
            mix_hops: reply_to_hops,
            forward_from_tun_rx,
            mixnet_client_sender,
            close_rx,
            finished_tx: Some(finished_tx),
            activity_timeout: tokio::time::interval(Duration::from_secs(ACTIVITY_TIMEOUT_SEC)),
        };

        tokio::spawn(async move {
            if let Err(err) = connected_client_handler.run().await {
                log::error!("connected client handler has failed: {err}")
            }
        });

        (forward_from_tun_tx, close_tx, finished_rx)
    }

    async fn handle_packet(&mut self, packet: Vec<u8>) -> Result<()> {
        self.activity_timeout.reset();
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
                    log::trace!("ConnectedClientHandler: received shutdown");
                    break;
                },
                _ = self.activity_timeout.tick() => {
                    log::debug!("ConnectedClientHandler: activity timeout reached for {}", self.nym_address);
                    break;
                }
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
                },
            }
        }

        log::debug!("ConnectedClientHandler: exiting");
        Ok(())
    }
}

impl Drop for ConnectedClientHandler {
    fn drop(&mut self) {
        log::trace!("ConnectedClientHandler: dropping");
        if let Some(finished_tx) = self.finished_tx.take() {
            let _ = finished_tx.send(());
        }
    }
}
