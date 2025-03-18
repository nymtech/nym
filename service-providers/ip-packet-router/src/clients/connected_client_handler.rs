// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::time::Duration;

use bytes::BytesMut;
use futures::SinkExt;
use nym_ip_packet_requests::{
    codec::{IprPacket, MultiIpPacketCodec},
    v6::response::IpPacketResponse as IpPacketResponseV6,
    v7::response::IpPacketResponse as IpPacketResponseV7,
    v8::response::IpPacketResponse as IpPacketResponseV8,
};
use nym_sdk::mixnet::{
    InputMessage, MixnetClientSender, MixnetMessageSink, MixnetMessageSinkTranslator,
};
use tokio::{
    sync::{mpsc, oneshot},
    time::interval,
};
use tokio_util::codec::FramedWrite;

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
    mixnet_ip_packet_sink: FramedWrite<MixnetMessageSink<ToIprDataResponse>, MultiIpPacketCodec>,
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

        // The mixnet sink takes bytes, create IPR response types that the recipient can
        // understand, and sends them as InputMessages to the mixnet.
        let mixnet_client_sink = MixnetMessageSink::new_with_custom_translator(
            mixnet_client_sender,
            ToIprDataResponse {
                send_to: client_id.clone(),
                client_version,
            },
        );

        // The mixnet ip packet sink takes IP packets, bundles them together, and sends them to the
        // mixnet client sink
        let mixnet_ip_packet_sink = FramedWrite::new(mixnet_client_sink, MultiIpPacketCodec::new());

        let connected_client_handler = ConnectedClientHandler {
            sent_by: client_id,
            forward_from_tun_rx,
            close_rx,
            activity_timeout,
            payload_topup_interval,
            mixnet_ip_packet_sink,
        };

        let handle = tokio::spawn(async move {
            if let Err(err) = connected_client_handler.run().await {
                log::error!("connected client handler has failed: {err}")
            }
        });

        (forward_from_tun_tx, close_tx, handle)
    }

    async fn handle_packet(&mut self, packet: IprPacket) -> Result<()> {
        self.activity_timeout.reset();
        self.payload_topup_interval.reset();

        self.mixnet_ip_packet_sink
            .send(packet)
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
                    if let Err(err) = self.handle_packet(IprPacket::Flush).await {
                        log::error!("client handler: failed to handle packet: {err}");
                    }
                },
                packet = self.forward_from_tun_rx.recv() => match packet {
                    Some(packet) => {
                        if let Err(err) = self.handle_packet(IprPacket::from(packet)).await {
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

fn create_ip_packet_response(
    packets: &[u8],
    client_version: ClientVersion,
) -> std::result::Result<Vec<u8>, bincode::Error> {
    let packets = BytesMut::from(packets).freeze();
    match client_version {
        ClientVersion::V6 => IpPacketResponseV6::new_ip_packet(packets).to_bytes(),
        ClientVersion::V7 => IpPacketResponseV7::new_ip_packet(packets).to_bytes(),
        ClientVersion::V8 => IpPacketResponseV8::new_ip_packet(packets).to_bytes(),
    }
}

// This struct is used by the sink to translate the the bundled IP packets into a IPR packet
// responses that can be sent to the mixnet.
struct ToIprDataResponse {
    send_to: ConnectedClientId,
    client_version: ClientVersion,
}

impl MixnetMessageSinkTranslator for ToIprDataResponse {
    fn to_input_message(
        &self,
        bundled_ip_packets: &[u8],
    ) -> std::result::Result<InputMessage, nym_sdk::Error> {
        // Create a IPR packet response that the recipient can understand
        let response_packet = create_ip_packet_response(bundled_ip_packets, self.client_version)?;

        // Wrap the response packet in a mixnet input message
        let input_message =
            crate::util::create_message::create_input_message(&self.send_to, response_packet)
                .with_max_retransmissions(0);

        Ok(input_message)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use bytes::Bytes;
    use nym_sdk::mixnet::{AnonymousSenderTag, MixnetMessageSender};
    use tokio::sync::Notify;

    use super::*;

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

        let bytes_to_input_message = ToIprDataResponse {
            send_to: client_id.clone(),
            client_version,
        };

        let mixnet_ip_packet_sender = MixnetMessageSink::new_with_custom_translator(
            mixnet_client_sender.clone(),
            bytes_to_input_message,
        );

        let mut ip_packet_sender =
            FramedWrite::new(mixnet_ip_packet_sender, MultiIpPacketCodec::new());

        assert!(mixnet_client_sender.sent_messages().is_empty());

        // Send two packets. These will be bundled together by the codec
        ip_packet_sender
            .send(IprPacket::Data(Bytes::from("hello".to_owned())))
            .await
            .expect("failed to send");

        ip_packet_sender
            .send(IprPacket::Data(Bytes::from("world".to_owned())))
            .await
            .expect("failed to send");

        // Packets are still being collected by the codec
        assert!(mixnet_client_sender.sent_messages().is_empty());

        // The codec will bundle packets together until it fills out the sphinx packet payload, but
        // we can trigger sending what it has accumulated so far by sending an explicit flush
        ip_packet_sender
            .send(IprPacket::Flush)
            .await
            .expect("failed to send");

        // This will never been seen by the mixnet sender as it never gets further than the codec
        ip_packet_sender
            .send(IprPacket::Data(Bytes::from("never seen".to_owned())))
            .await
            .expect("failed to send");

        mixnet_client_sender.wait_for_messages(1).await;
        assert_eq!(mixnet_client_sender.sent_messages().len(), 1);
    }
}
