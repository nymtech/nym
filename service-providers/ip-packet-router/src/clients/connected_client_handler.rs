// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::time::Duration;

use bytes::Bytes;
use nym_ip_packet_requests::{
    codec::MultiIpPacketCodec, v6::response::IpPacketResponse as IpPacketResponseV6,
    v7::response::IpPacketResponse as IpPacketResponseV7,
    v8::response::IpPacketResponse as IpPacketResponseV8,
};
use nym_sdk::mixnet::MixnetMessageSender;
use tokio::{
    sync::{mpsc, oneshot},
    time::interval,
};

use crate::{
    clients::ConnectedClientId,
    constants::CLIENT_HANDLER_ACTIVITY_TIMEOUT,
    error::{IpPacketRouterError, Result},
    messages::ClientVersion,
    util::create_message::create_input_message,
};

// Data flow
// Out: mixnet_listener -> decode -> handle_packet -> write_to_tun
// In: tun_listener -> [connected_client_handler -> encode] -> mixnet_sender

// This handler is spawned as a task, and it listens to IP packets passed from the tun_listener,
// encodes it, and then sends to mixnet.
pub(crate) struct ConnectedClientHandler {
    // The client that sent the packets
    sent_by: ConnectedClientId,

    // Channel to receive packets from the tun_listener
    forward_from_tun_rx: mpsc::UnboundedReceiver<Vec<u8>>,

    // Channel to send packets to the mixnet
    mixnet_client_sender: nym_sdk::mixnet::MixnetClientSender,

    // Channel to receive close signal
    close_rx: oneshot::Receiver<()>,

    // Interval to check for activity timeout
    activity_timeout: tokio::time::Interval,

    // Encoder to bundle multiple packets into a single one
    encoder: MultiIpPacketCodec,

    // The version of the client
    client_version: ClientVersion,
}

impl ConnectedClientHandler {
    pub(crate) fn start(
        client_id: ConnectedClientId,
        buffer_timeout: Duration,
        client_version: ClientVersion,
        mixnet_client_sender: nym_sdk::mixnet::MixnetClientSender,
    ) -> (
        mpsc::UnboundedSender<Vec<u8>>,
        oneshot::Sender<()>,
        tokio::task::JoinHandle<()>,
    ) {
        log::debug!("Starting connected client handler for: {}", client_id);
        log::debug!("client version: {:?}", client_version);
        let (close_tx, close_rx) = oneshot::channel();
        let (forward_from_tun_tx, forward_from_tun_rx) = mpsc::unbounded_channel();

        // Reset so that we don't get the first tick immediately
        let mut activity_timeout = interval(CLIENT_HANDLER_ACTIVITY_TIMEOUT);
        activity_timeout.reset();

        let encoder = MultiIpPacketCodec::new(buffer_timeout);

        let connected_client_handler = ConnectedClientHandler {
            sent_by: client_id,
            forward_from_tun_rx,
            mixnet_client_sender,
            close_rx,
            activity_timeout,
            encoder,
            client_version,
        };

        let handle = tokio::spawn(async move {
            if let Err(err) = connected_client_handler.run().await {
                log::error!("connected client handler has failed: {err}")
            }
        });

        (forward_from_tun_tx, close_tx, handle)
    }

    async fn create_ip_packet(&self, packets: Bytes) -> Result<Vec<u8>> {
        match self.client_version {
            ClientVersion::V6 => IpPacketResponseV6::new_ip_packet(packets).to_bytes(),
            ClientVersion::V7 => IpPacketResponseV7::new_ip_packet(packets).to_bytes(),
            ClientVersion::V8 => IpPacketResponseV8::new_ip_packet(packets).to_bytes(),
        }
        .map_err(|err| IpPacketRouterError::FailedToSerializeResponsePacket { source: err })
    }

    async fn send_packets_to_mixnet(&mut self, packets: Bytes) -> Result<()> {
        let response_packet = self.create_ip_packet(packets).await?;
        let input_message = create_input_message(&self.sent_by, response_packet);

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
                    log::info!("client handler stopping: received close: {}", self.sent_by);
                    break;
                },
                _ = self.activity_timeout.tick() => {
                    log::info!("client handler stopping: activity timeout: {}", self.sent_by);
                    break;
                },
                packets = self.encoder.buffer_timeout() => match packets {
                    Some(packets) => {
                        if let Err(err) = self.handle_buffer_timeout(packets).await {
                            log::error!("client handler: failed to handle buffer timeout: {err}");
                        }
                    },
                    None => log::trace!("no packets to send"),
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
