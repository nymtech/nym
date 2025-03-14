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

    // The time we have to topup a payload before we send, regardless
    payload_topup_interval: tokio::time::Interval,

    // The sender to the mixnet. It's a framed writer that bundles IP packets together to fill out
    // the sphinx packet payload before sending.
    mixnet_ip_packet_sender:
        FramedWrite<MixnetClientIpPacketSender<BytesToInputMessageImpl>, MultiIpPacketCodec>,
}

pub fn packet_translator(
    reply_to: &ConnectedClientId,
    client_version: ClientVersion,
    packet: Bytes,
) -> Result<InputMessage> {
    // Create a IPR packet response that the recipient can understand
    let response_packet = create_ip_packet_response(packet, client_version)?;

    // Wrap the response packet in a mixnet input message
    Ok(crate::util::create_message::create_input_message(
        reply_to,
        response_packet,
    ))
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

        let mut payload_topup_interval = interval(buffer_timeout);
        payload_topup_interval.reset();

        let bytes_to_input_message = BytesToInputMessageImpl {
            send_to: client_id.clone(),
            client_version,
        };

        let mixnet_client_sender_writer = MixnetClientIpPacketSender::new(
            mixnet_client_sender,
            // client_id.clone(),
            // client_version,
            bytes_to_input_message,
        );

        let encoder = MultiIpPacketCodec::new();
        let framed_writer = FramedWrite::new(mixnet_client_sender_writer, encoder);

        let connected_client_handler = ConnectedClientHandler {
            sent_by: client_id,
            forward_from_tun_rx,
            close_rx,
            activity_timeout,
            payload_topup_interval,
            mixnet_ip_packet_sender: framed_writer,
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
        self.payload_topup_interval.reset();

        self.mixnet_ip_packet_sender
            .send(Bytes::from(packet))
            .await
            .map_err(|source| IpPacketRouterError::FailedToEncodeMixnetMessage { source })
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
                _ = self.payload_topup_interval.tick() => {
                    // Send an empty packet to trigger the buffer timeout
                    if let Err(err) = self.handle_packet(Vec::new()).await {
                        log::error!("client handler: failed to handle packet: {err}");
                    }
                },
                packet = self.forward_from_tun_rx.recv() => match packet {
                    Some(packet) => {
                        // I don't think this should ever happen, so log this so we are aware of it
                        if packet.is_empty() {
                            log::warn!("client handler: received empty packet");
                            continue;
                        }
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

fn create_ip_packet_response(packets: Bytes, client_version: ClientVersion) -> Result<Vec<u8>> {
    match client_version {
        ClientVersion::V6 => IpPacketResponseV6::new_ip_packet(packets).to_bytes(),
        ClientVersion::V7 => IpPacketResponseV7::new_ip_packet(packets).to_bytes(),
        ClientVersion::V8 => IpPacketResponseV8::new_ip_packet(packets).to_bytes(),
    }
    .map_err(|err| IpPacketRouterError::FailedToSerializeResponsePacket { source: err })
}

// Wrapper around the mixnet sender that takes bundled IP packet payloads, wraps them in a IPR IP
// packet response, and sends them to the mixnet.
//
// It is used together with the `FramedWrite` to bundle IP packets together to fill out the sphinx
struct MixnetClientIpPacketSender<F>
where
    F: BytesToInputMessage,
{
    packet_translator: F,

    tx: PollSender<InputMessage>,
    send_task: JoinHandle<()>,
}

trait BytesToInputMessage: Unpin {
    fn to_input_message(&self, bytes: bytes::Bytes) -> Result<InputMessage>;
}

struct BytesToInputMessageImpl {
    send_to: ConnectedClientId,
    client_version: ClientVersion,
}

impl BytesToInputMessage for BytesToInputMessageImpl {
    fn to_input_message(&self, bytes: bytes::Bytes) -> Result<InputMessage> {
        // Create a IPR packet response that the recipient can understand
        let response_packet = create_ip_packet_response(bytes, self.client_version)?;

        // Wrap the response packet in a mixnet input message
        Ok(crate::util::create_message::create_input_message(
            &self.send_to,
            response_packet,
        ))
    }
}

impl<F> MixnetClientIpPacketSender<F>
where
    F: BytesToInputMessage,
{
    fn new<Sender>(mixnet_client_sender: Sender, packet_translator: F) -> Self
    where
        Sender: MixnetMessageSender + Send + 'static,
    {
        // We keep the buffer size relatively small to signal backpressure early
        let (tx, mut rx) = mpsc::channel(8);

        let send_task = tokio::spawn(async move {
            while let Some(input_message) = rx.recv().await {
                if let Err(err) = mixnet_client_sender.send(input_message).await {
                    log::error!("failed to send packet to mixnet: {err}");
                }
            }
        });

        MixnetClientIpPacketSender {
            packet_translator,
            tx: PollSender::new(tx),
            send_task,
        }
    }
}

impl<F> Drop for MixnetClientIpPacketSender<F>
where
    F: BytesToInputMessage,
{
    fn drop(&mut self) {
        self.send_task.abort();
    }
}

impl<F> AsyncWrite for MixnetClientIpPacketSender<F>
where
    F: BytesToInputMessage,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::result::Result<usize, std::io::Error>> {
        ready!(self.tx.poll_ready_unpin(cx)).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::Other, "failed to send packet to mixnet")
        })?;

        let packet = BytesMut::from(buf).freeze();
        let input_message = self
            .packet_translator
            .to_input_message(packet)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;

        // Pass it to the mixnet sender
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

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    use async_trait::async_trait;
    use nym_sdk::mixnet::AnonymousSenderTag;
    use tokio::sync::Notify;

    #[derive(Clone)]
    struct MockMixnetClientSender {
        sent_messages: Arc<Mutex<Vec<InputMessage>>>,
        notify: Arc<Notify>,
    }

    impl MockMixnetClientSender {
        fn new() -> Self {
            MockMixnetClientSender {
                sent_messages: Arc::new(Mutex::new(Vec::new())),
                notify: Arc::new(Notify::new()),
            }
        }

        fn sent_messages(&self) -> Vec<String> {
            let sent_messages = self.sent_messages.lock().unwrap();
            sent_messages
                .iter()
                .map(|msg| format!("{msg:?}").to_owned())
                .collect()
        }

        async fn wait_for_messages(&self, count: usize) {
            loop {
                if self.sent_messages.lock().unwrap().len() >= count {
                    break;
                }
                self.notify.notified().await;
            }
        }
    }

    #[async_trait]
    impl MixnetMessageSender for MockMixnetClientSender {
        async fn send(&self, message: InputMessage) -> std::result::Result<(), nym_sdk::Error> {
            let mut sent_messages = self.sent_messages.lock().unwrap();
            sent_messages.push(message);
            self.notify.notify_one();
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_combining_framed_write_and_mixnet_client_ip_packet_sender() {
        let mixnet_client_sender = MockMixnetClientSender::new();
        let sender_tag = AnonymousSenderTag::new_random(&mut rand::thread_rng());
        let client_id = ConnectedClientId::AnonymousSenderTag(sender_tag);
        let client_version = ClientVersion::V8;

        let bytes_to_input_message = BytesToInputMessageImpl {
            send_to: client_id.clone(),
            client_version,
        };

        let mixnet_ip_packet_sender =
            MixnetClientIpPacketSender::new(mixnet_client_sender.clone(), bytes_to_input_message);

        let mut ip_packet_sender =
            FramedWrite::new(mixnet_ip_packet_sender, MultiIpPacketCodec::new());

        assert!(mixnet_client_sender.sent_messages().is_empty());

        // Send two packets. These will be bundled together by the codec
        ip_packet_sender
            .send(Bytes::from("hello".to_owned()))
            .await
            .expect("failed to send");

        ip_packet_sender
            .send(Bytes::from("world".to_owned()))
            .await
            .expect("failed to send");

        // Packets are still being collected by the codec
        assert!(mixnet_client_sender.sent_messages().is_empty());

        // The codec will bundle packets together until it fills out the sphinx packet payload, but
        // we can trigger sending what it has accumulated so far by sending an empty packet
        ip_packet_sender
            .send(Bytes::from("").to_owned())
            .await
            .expect("failed to send");
        assert!(mixnet_client_sender.sent_messages().is_empty());

        // This will never been seen by the mixnet sender as it never gets further than the codec
        ip_packet_sender
            .send(Bytes::from("never seen".to_owned()))
            .await
            .expect("failed to send");

        assert!(mixnet_client_sender.sent_messages().is_empty());
        mixnet_client_sender.wait_for_messages(1).await;
        assert_eq!(mixnet_client_sender.sent_messages().len(), 1);
    }
}
