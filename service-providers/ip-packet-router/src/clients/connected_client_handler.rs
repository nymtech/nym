// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use bytes::{Bytes, BytesMut};
use futures::{ready, SinkExt};
use nym_ip_packet_requests::{
    codec::MultiIpPacketCodec, v6::response::IpPacketResponse as IpPacketResponseV6,
    v7::response::IpPacketResponse as IpPacketResponseV7,
    v8::response::IpPacketResponse as IpPacketResponseV8,
};
use nym_sdk::mixnet::{InputMessage, MixnetClientSender, MixnetMessageSender};
use tokio::{
    io::AsyncWrite,
    sync::{mpsc, oneshot},
    task::JoinHandle,
    time::interval,
};
use tokio_util::{codec::FramedWrite, sync::PollSender};

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

    // Channel to receive close signal
    close_rx: oneshot::Receiver<()>,

    // Interval to check for activity timeout
    activity_timeout: tokio::time::Interval,

    // Encoder to bundle multiple packets into a single one
    // encoder: MultiIpPacketCodec,
    framed_writer: FramedWrite<MixnetClientSenderWriter, MultiIpPacketCodec>,

    buffer_timeout: Duration,
}

struct MixnetClientSenderWriter {
    send_to: ConnectedClientId,
    client_version: ClientVersion,

    tx: PollSender<InputMessage>,
    send_task: JoinHandle<()>,
}

impl MixnetClientSenderWriter {
    fn new(
        mixnet_client_sender: MixnetClientSender,
        send_to: ConnectedClientId,
        client_version: ClientVersion,
    ) -> Self {
        // We keep the buffer size relatively small to signal backpressure early
        let (tx, mut rx) = mpsc::channel(8);

        let send_task = tokio::spawn(async move {
            while let Some(input_message) = rx.recv().await {
                if let Err(err) = mixnet_client_sender.send(input_message).await {
                    log::error!("failed to send packet to mixnet: {err}");
                }
            }
        });

        MixnetClientSenderWriter {
            send_to,
            client_version,
            tx: PollSender::new(tx),
            send_task,
        }
    }
}

impl Drop for MixnetClientSenderWriter {
    fn drop(&mut self) {
        self.send_task.abort();
    }
}

impl AsyncWrite for MixnetClientSenderWriter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::result::Result<usize, std::io::Error>> {
        ready!(self.tx.poll_ready_unpin(cx)).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::Other, "failed to send packet to mixnet")
        })?;

        let packet = BytesMut::from(buf).freeze();
        let response_packet = create_ip_packet(packet, self.client_version).map_err(|err| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("failed to create ip packet: {err}"),
            )
        })?;
        let input_message = create_input_message(&self.send_to, response_packet);

        self.tx.start_send_unpin(input_message).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::Other, "failed to send packet to mixnet")
        })?;

        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<std::result::Result<(), std::io::Error>> {
        ready!(self.tx.poll_flush_unpin(cx)).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::Other, "failed to send packet to mixnet")
        })?;
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<std::result::Result<(), std::io::Error>> {
        self.poll_flush(cx)
    }
}

impl ConnectedClientHandler {
    pub(crate) fn start(
        client_id: ConnectedClientId,
        buffer_timeout: Duration,
        client_version: ClientVersion,
        mixnet_client_sender: MixnetClientSender,
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

        let mixnet_client_sender_writer =
            MixnetClientSenderWriter::new(mixnet_client_sender, client_id.clone(), client_version);

        let encoder = MultiIpPacketCodec::new();
        let framed_writer = FramedWrite::new(mixnet_client_sender_writer, encoder);

        let connected_client_handler = ConnectedClientHandler {
            sent_by: client_id,
            forward_from_tun_rx,
            close_rx,
            activity_timeout,
            framed_writer,
            buffer_timeout,
        };

        let handle = tokio::spawn(async move {
            if let Err(err) = connected_client_handler.run().await {
                log::error!("connected client handler has failed: {err}")
            }
        });

        (forward_from_tun_tx, close_tx, handle)
    }

    async fn handle_packet(&mut self, packet: Vec<u8>) -> Result<()> {
        self.activity_timeout.reset();
        self.framed_writer
            .send(Bytes::from(packet))
            .await
            .map_err(|source| IpPacketRouterError::FailedToEncodeMixnetMessage { source })
    }

    async fn run(mut self) -> Result<()> {
        let mut payload_topup_interval = interval(self.buffer_timeout);
        payload_topup_interval.reset();
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
                _ = payload_topup_interval.tick() => {
                    // Send an empty packet to trigger the buffer timeout
                    if let Err(err) = self.handle_packet(Vec::new()).await {
                        log::error!("client handler: failed to handle packet: {err}");
                    }
                },
                packet = self.forward_from_tun_rx.recv() => match packet {
                    Some(packet) => {
                        payload_topup_interval.reset();
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

fn create_ip_packet(packets: Bytes, client_version: ClientVersion) -> Result<Vec<u8>> {
    match client_version {
        ClientVersion::V6 => IpPacketResponseV6::new_ip_packet(packets).to_bytes(),
        ClientVersion::V7 => IpPacketResponseV7::new_ip_packet(packets).to_bytes(),
        ClientVersion::V8 => IpPacketResponseV8::new_ip_packet(packets).to_bytes(),
    }
    .map_err(|err| IpPacketRouterError::FailedToSerializeResponsePacket { source: err })
}
