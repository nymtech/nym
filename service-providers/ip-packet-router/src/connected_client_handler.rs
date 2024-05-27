// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use bytes::Bytes;
use nym_ip_packet_requests::{codec::MultiIpPacketCodec, v6::response::IpPacketResponse};
use nym_sdk::mixnet::{MixnetMessageSender, Recipient};

use crate::{
    constants::CLIENT_HANDLER_ACTIVITY_TIMEOUT,
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
    activity_timeout: tokio::time::Interval,
    encoder: MultiIpPacketCodec,
}

impl ConnectedClientHandler {
    pub(crate) fn start(
        reply_to: Recipient,
        reply_to_hops: Option<u8>,
        buffer_timeout: std::time::Duration,
        mixnet_client_sender: nym_sdk::mixnet::MixnetClientSender,
    ) -> (
        tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
        tokio::sync::oneshot::Sender<()>,
        tokio::task::JoinHandle<()>,
    ) {
        let (close_tx, close_rx) = tokio::sync::oneshot::channel();
        let (forward_from_tun_tx, forward_from_tun_rx) = tokio::sync::mpsc::unbounded_channel();

        // Reset so that we don't get the first tick immediately
        let mut activity_timeout = tokio::time::interval(CLIENT_HANDLER_ACTIVITY_TIMEOUT);
        activity_timeout.reset();

        let encoder = MultiIpPacketCodec::new(buffer_timeout);

        let connected_client_handler = ConnectedClientHandler {
            nym_address: reply_to,
            mix_hops: reply_to_hops,
            forward_from_tun_rx,
            mixnet_client_sender,
            close_rx,
            activity_timeout,
            encoder,
        };

        let handle = tokio::spawn(async move {
            if let Err(err) = connected_client_handler.run().await {
                log::error!("connected client handler has failed: {err}")
            }
        });

        (forward_from_tun_tx, close_tx, handle)
    }

    async fn send_packets_to_mixnet(&mut self, packets: Bytes) -> Result<()> {
        let response_packet = IpPacketResponse::new_ip_packet(packets)
            .to_bytes()
            .map_err(|err| IpPacketRouterError::FailedToSerializeResponsePacket { source: err })?;
        let input_message = create_input_message(self.nym_address, response_packet, self.mix_hops);

        self.mixnet_client_sender
            .send(input_message)
            .await
            .map_err(|err| IpPacketRouterError::FailedToSendPacketToMixnet { source: err })
    }

    async fn handle_buffer_timeout(&mut self, packets: Bytes) -> Result<()> {
        if !packets.is_empty() {
            self.send_packets_to_mixnet(packets).await
        } else {
            Ok(())
        }
    }

    async fn handle_packet(&mut self, packet: Vec<u8>) -> Result<()> {
        self.activity_timeout.reset();

        if let Some(bundled_packets) = self.encoder.append_packet(packet.into()) {
            self.send_packets_to_mixnet(bundled_packets).await
        } else {
            Ok(())
        }
    }

    async fn run(mut self) -> Result<()> {
        loop {
            tokio::select! {
                _ = &mut self.close_rx => {
                    log::info!("client handler stopping: received close: {}", self.nym_address);
                    break;
                },
                _ = self.activity_timeout.tick() => {
                    log::info!("client handler stopping: activity timeout: {}", self.nym_address);
                    break;
                },
                Some(packets) = self.encoder.buffer_timeout() => {
                    if let Err(err) = self.handle_buffer_timeout(packets).await {
                        log::error!("client handler: failed to handle buffer timeout: {err}");
                    }
                },
                packet = self.forward_from_tun_rx.recv() => match packet {
                    Some(packet) => {
                        if let Err(err) = self.handle_packet(packet).await {
                            log::error!("client handler: failed to handle packet: {err}");
                        }
                    },
                    None => {
                        log::info!("client handler stopping: tun channel closed");
                        break;
                    }
                },
            }
        }

        log::debug!("ConnectedClientHandler: exiting");
        Ok(())
    }
}
