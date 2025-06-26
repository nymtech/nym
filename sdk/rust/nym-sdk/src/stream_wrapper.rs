use crate::mixnet::InputMessage;
use crate::mixnet::{MixnetClient, MixnetClientSender, Recipient};
use crate::Error;
use bytes::BytesMut;
use futures::SinkExt;
use nym_client_core::client::inbound_messages::InputMessageCodec;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use nym_sphinx::receiver::{ReconstructedMessage, ReconstructedMessageCodec};
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::sync::oneshot;
use tokio_util::codec::{Decoder, Encoder};
use tracing::field::debug;
use tracing::{debug, info, warn};

/**
 * TODO
 * - Convenience methods? Depends on what we want to put in here and what might be used / impl-ed in consuming libraries
 * - https://github.com/nymtech/nym-vpn-client/tree/develop/nym-vpn-core/crates/nym-ip-packet-client/src - hook into IPR
 * - builder pattern via MixSocket + tests
 */

/// MixSocket is following the structure of something like Tokio::net::TcpSocket with regards to setup and interface, breakdown from TcpSocket to TcpStream, etc.
/// However, we can't map this one to one onto the TcpSocket as there isn't really a concept of binding to a port with the MixnetClient; it connects to its Gateway and then just accepts incoming messages from the Gw via the Websocket connection. However, we can stick with the idea of creating a Socket in an unconnected state, either using it to make a new Stream (connecting it to its EntryGw) or connecting it *to* something (once the IPR functionality is enabled, this will mean the creation of a Stream + kicking off the creation of a tunnel to an ExitGw + IPR).
/// The cause for a MixSocket > going striaght to a MixStream is creating a Nym Client disconnected from the Mixnet first, then upgrading to a Stream when connecting it. Once LP is implemented, this will also allow us to follow something like what is implemented for the Tokio::net::UdpFramed abstraction, where we can create multiple MixStream instances from a single MixSocket, all connected to different Recipients.
pub struct MixSocket {
    inner: MixnetClient,
}

impl MixSocket {
    /// Create a new socket that is disconnected from the Mixnet - kick off the Mixnet client with config for builder.
    /// Following idea of having single client with multiple concurrent connections represented by per-Recipient MixStream instance.
    pub async fn new() -> Result<Self, Error> {
        todo!()
    }

    /// Connect to a specific peer (Nym Client) and return a Stream (cf TcpSocket::connect() / TcpStream::new()).
    pub async fn connect_to(_recipient: Recipient) -> Result<MixStream, Error> {
        todo!()
    }

    /// Get our Nym address.
    pub fn nym_address(&self) -> &Recipient {
        self.inner.nym_address()
    }

    pub fn get_ref(&self) -> &MixnetClient {
        &self.inner
    }

    pub fn get_mut(&mut self) -> &mut MixnetClient {
        &mut self.inner
    }

    pub fn into_inner(self) -> MixnetClient {
        self.inner
    }
}

pub struct MixStream {
    client: MixnetClient,
    peer: Option<Recipient>, // We might be accepting incoming messages and replying, so might not have a Nym addr to talk to..
    peer_surbs: Option<AnonymousSenderTag>, // ..since we might just be using SURBs instead
}

impl MixStream {
    /// Create a MixStream instance and immediately connect (convenience method) or pass in a MixSocket (pre-configured DisconnectedMixnetClient).
    // TODO in future take config from MixSocket if exists in Option<> param, else spin up ephemeral client. Just doing ephemeral for initial sketch.
    pub async fn new(socket: Option<MixSocket>, peer: Recipient) -> Self {
        let client = match socket {
            Some(socket) => socket.into_inner(),
            None => MixnetClient::connect_new().await.unwrap(),
        };
        Self {
            client,
            peer: Some(peer),
            peer_surbs: None,
        }
    }

    /// Nym address of Stream's peer (Nym Client it will communicate with).
    pub fn peer_addr(&self) -> Recipient {
        let peer = &self.peer.expect("No Peer set");
        peer.clone()
    }

    /// Our Nym address.
    pub fn local_addr(&self) -> &Recipient {
        self.client.nym_address()
    }

    pub fn store_surbs(&mut self, surbs: AnonymousSenderTag) {
        self.peer_surbs = Some(surbs);
    }

    /// Stored SURBs (if any).
    pub fn surbs(&self) -> Option<AnonymousSenderTag> {
        self.peer_surbs
    }

    /// Split for concurrent read/write (like TcpStream::Split) into MixnetStreamReader and MixnetStreamWriter.
    pub fn split(self) -> (MixStreamReader, MixStreamWriter) {
        debug!("Splitting MixStream");
        let sender = self.client.split_sender();
        debug!("Split MixStream into Reader and Writer");
        let (surb_tx, surb_rx) = oneshot::channel();
        (
            MixStreamReader {
                client: self.client,
                peer: self.peer,
                peer_surbs: self.peer_surbs,
                surb_tx: Some(surb_tx),
            },
            MixStreamWriter {
                sender,
                peer: self.peer.expect("No Peer set"),
                peer_surbs: self.peer_surbs,
                surb_rx: Some(surb_rx),
            },
        )
    }

    /// Convenience method for just piping bytes into the Mixnet.
    pub async fn write_bytes(&mut self, data: &[u8]) -> Result<(), Error> {
        let input_message = if self.peer_surbs.is_some() {
            info!("Writing reply with SURBs");
            InputMessage::Reply {
                recipient_tag: (self.peer_surbs.expect("No Peer SURBs set")),
                data: (data.to_owned()),
                lane: (nym_task::connections::TransmissionLane::General),
                max_retransmissions: (Some(5)), // TODO check with Drazen - guessing here
            }
        } else {
            info!("Writing outgoing reply using Nym address");
            InputMessage::Anonymous {
                recipient: (self.peer.expect("No Peer set")),
                data: (data.to_owned()),
                reply_surbs: (10),
                lane: (nym_task::connections::TransmissionLane::General),
                max_retransmissions: (Some(5)), // TODO check with Drazen - guessing here
            }
        };

        let mut codec = InputMessageCodec {};
        let mut serialized_bytes = BytesMut::new();
        codec.encode(input_message, &mut serialized_bytes)?;
        info!("Serialized bytes: {:?}", serialized_bytes);

        self.write_all(&serialized_bytes).await?;
        info!("Wrote serialized bytes");
        self.flush().await?;
        debug!("Flushed");

        Ok(())
    }

    /// Disconnect client from the Mixnet - note that disconnected clients cannot currently be reconnected.
    pub async fn disconnect(self) {
        debug!("Disconnecting");
        self.client.disconnect().await;
        debug!("Disconnected");
    }
}

impl AsyncRead for MixStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.client).poll_read(cx, buf)
    }
}

impl AsyncWrite for MixStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.client).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.client).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.client).poll_shutdown(cx)
    }
}

pub struct MixStreamReader {
    client: MixnetClient,
    peer: Option<Recipient>, // We might be accepting incoming messages and replying, so might not have a Nym addr to talk to..
    peer_surbs: Option<AnonymousSenderTag>, // ..since we might just be using SURBs instead
    surb_tx: Option<oneshot::Sender<AnonymousSenderTag>>,
}

impl MixStreamReader {
    /// Nym address of Stream's peer (Nym Client it will communicate with).
    pub fn peer_addr(&self) -> Recipient {
        let peer = &self.peer.expect("No Peer set");
        peer.clone()
    }

    /// Our Nym address.
    pub fn local_addr(&self) -> &Recipient {
        self.client.nym_address()
    }

    /// Store SURBs in both own field + send to the other side of the split so they can be used for the connection.
    pub fn store_surbs(&mut self, surbs: AnonymousSenderTag) {
        self.peer_surbs = Some(surbs);
        if let Some(tx) = self.surb_tx.take() {
            tx.send(surbs); // TODO err handling
        }
    }

    /// Stored SURBs (if any).
    pub fn surbs(&self) -> Option<AnonymousSenderTag> {
        self.peer_surbs
    }
}

impl AsyncRead for MixStreamReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.client).poll_read(cx, buf)
    }
}

pub struct MixStreamWriter {
    sender: MixnetClientSender,
    peer: Recipient,
    peer_surbs: Option<AnonymousSenderTag>,
    surb_rx: Option<oneshot::Receiver<AnonymousSenderTag>>,
}

impl MixStreamWriter {
    /// Convenience method for just piping bytes into the Mixnet.
    pub async fn write_bytes(&mut self, data: &[u8]) -> Result<(), Error> {
        if self.peer_surbs.is_none() {
            if let Some(mut rx) = self.surb_rx.take() {
                if let Ok(surbs) = rx.try_recv() {
                    self.peer_surbs = Some(surbs);
                }
            }
        }

        let input_message = if self.peer_surbs.is_some() {
            InputMessage::Reply {
                recipient_tag: (self.peer_surbs.expect("No Peer SURBs set")),
                data: (data.to_owned()),
                lane: (nym_task::connections::TransmissionLane::General),
                max_retransmissions: (Some(5)), // TODO check with Drazen - guessing here
            }
        } else {
            InputMessage::Anonymous {
                recipient: (self.peer),
                data: (data.to_owned()),
                reply_surbs: (10),
                lane: (nym_task::connections::TransmissionLane::General),
                max_retransmissions: (Some(5)), // TODO check with Drazen - guessing here
            }
        };

        let mut codec = InputMessageCodec {};
        let mut serialized_bytes = BytesMut::new();
        codec.encode(input_message, &mut serialized_bytes)?;
        info!("Serialized bytes: {:?}", serialized_bytes);

        self.write_all(&serialized_bytes).await?;
        info!("Wrote serialized bytes");
        self.flush().await?;
        debug!("Flushed");

        Ok(())
    }
}

impl AsyncWrite for MixStreamWriter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.sender).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.sender).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.sender).poll_shutdown(cx)
    }
}

/**
 * Tests TODO:
 * STREAM + STREAMREADER + STREAMWRITER
 * - make sure we can do TLS through this (aka get around the 'superinsecuredontuseinprod mode' flags)
 * SOCKET
 * - general tests: create new + various into() fns
 *
 */
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    // Quick test fn for easy testing of sending to self before writing Socket impl (see above todo)
    impl MixSocket {
        pub async fn new_test() -> Result<Self, Error> {
            let inner = MixnetClient::connect_new().await?;
            Ok(MixSocket { inner })
        }
    }

    static INIT: Once = Once::new();

    fn init_logging() {
        INIT.call_once(|| {
            nym_bin_common::logging::setup_tracing_logger();
        });
    }
    #[tokio::test]
    async fn simple_surb_reply_stream() -> Result<(), Box<dyn std::error::Error>> {
        // init_logging();

        let receiver_socket = MixSocket::new_test().await?;
        let receiver_address = receiver_socket.nym_address().clone();
        let sender_socket = MixSocket::new_test().await?;
        let sender_address = sender_socket.nym_address().clone();
        let mut receiver_stream =
            MixStream::new(Some(receiver_socket), sender_address.clone()).await;
        let mut sender_stream = MixStream::new(Some(sender_socket), receiver_address.clone()).await;

        sender_stream.write_bytes(b"Hello, Mixnet Split!").await?;

        let mut buffer = [0u8; 1024];
        match receiver_stream.read(&mut buffer).await {
            Ok(bytes_read) if bytes_read > 0 => {
                let mut codec = ReconstructedMessageCodec {};
                let mut buf = BytesMut::from(&buffer[..bytes_read]);

                if let Ok(Some(decoded_message)) = codec.decode(&mut buf) {
                    let payload_surbs = decoded_message.sender_tag;
                    assert!(payload_surbs.is_some());
                    receiver_stream.store_surbs(payload_surbs.unwrap());
                    receiver_stream.write_bytes(b"Hello, Mixnet reply!").await?;
                }
            }
            _ => panic!("Failed to receive initial message"),
        }

        let mut reply_buffer = [0u8; 1024];
        let reply_result = tokio::time::timeout(
            tokio::time::Duration::from_secs(30),
            sender_stream.read(&mut reply_buffer),
        )
        .await;

        match reply_result {
            Ok(Ok(bytes_read)) if bytes_read > 0 => {
                let mut codec = ReconstructedMessageCodec {};
                let mut buf = BytesMut::from(&reply_buffer[..bytes_read]);

                if let Ok(Some(decoded_message)) = codec.decode(&mut buf) {
                    assert_eq!(decoded_message.message.as_slice(), b"Hello, Mixnet reply!");
                }
                info!("Got reply!");
            }
            _ => panic!("Failed to receive reply"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn concurrent_surb_reply_split() -> Result<(), Box<dyn std::error::Error>> {
        // init_logging();

        let sender_socket = MixSocket::new_test().await?;
        let sender_address = sender_socket.nym_address().clone();
        let receiver_socket = MixSocket::new_test().await?;
        let receiver_address = receiver_socket.nym_address().clone();
        let sender_stream = MixStream::new(Some(sender_socket), receiver_address.clone()).await;
        let receiver_stream = MixStream::new(Some(receiver_socket), sender_address.clone()).await;

        let (mut sender_reader, mut sender_writer) = sender_stream.split();
        let (mut receiver_reader, mut receiver_writer) = receiver_stream.split();

        let sender_task = tokio::spawn(async move {
            for i in 0..5 {
                let msg = format!("Message {} requesting SURB reply", i);
                sender_writer.write_bytes(msg.as_bytes()).await?;
                info!("Sent message {}", i);
            }
            Ok::<_, Error>((sender_writer, 5))
        });

        let receiver_task = tokio::spawn(async move {
            let mut received_count = 0;
            let mut sent_replies = 0;
            let mut buffer = [0u8; 1024];

            while received_count < 5 {
                match tokio::time::timeout(
                    tokio::time::Duration::from_secs(10),
                    receiver_reader.read(&mut buffer),
                )
                .await
                {
                    Ok(Ok(bytes_read)) if bytes_read > 0 => {
                        let mut codec = ReconstructedMessageCodec {};
                        let mut buf = BytesMut::from(&buffer[..bytes_read]);

                        if let Ok(Some(decoded_message)) = codec.decode(&mut buf) {
                            info!(
                                "Received: {:?}",
                                String::from_utf8_lossy(&decoded_message.message)
                            );

                            if received_count == 0 && decoded_message.sender_tag.is_some() {
                                receiver_reader.store_surbs(decoded_message.sender_tag.unwrap());
                                info!("Stored SURBs");
                            }

                            received_count += 1;

                            if received_count == 3 && sent_replies == 0 {
                                for i in 0..3 {
                                    let reply = format!("SURB reply {}", i);
                                    receiver_writer.write_bytes(reply.as_bytes()).await?;
                                    info!("Sent SURB reply {}", i);
                                    sent_replies += 1;
                                    tokio::time::sleep(tokio::time::Duration::from_millis(200))
                                        .await;
                                }
                            }
                        }
                    }
                    _ => break,
                }
            }

            Ok::<_, Error>((
                receiver_reader,
                receiver_writer,
                received_count,
                sent_replies,
            ))
        });

        let reply_reader_task = tokio::spawn(async move {
            let mut reply_count = 0;
            let mut buffer = [0u8; 1024];

            while reply_count < 3 {
                match tokio::time::timeout(
                    tokio::time::Duration::from_secs(15),
                    sender_reader.read(&mut buffer),
                )
                .await
                {
                    Ok(Ok(bytes_read)) if bytes_read > 0 => {
                        let mut codec = ReconstructedMessageCodec {};
                        let mut buf = BytesMut::from(&buffer[..bytes_read]);

                        if let Ok(Some(decoded_message)) = codec.decode(&mut buf) {
                            let reply_text = String::from_utf8_lossy(&decoded_message.message);
                            info!("Received reply: {}", reply_text);
                            assert!(reply_text.contains("SURB reply"));
                            reply_count += 1;
                        }
                    }
                    _ => {
                        if reply_count == 0 {
                            panic!("No SURB replies received");
                        }
                        break;
                    }
                }
            }

            Ok::<_, Error>(reply_count)
        });

        let (_, sent_count) = sender_task.await??;
        let (_, _, received_count, sent_replies) = receiver_task.await??;
        let reply_count = reply_reader_task.await??;

        info!(
            "Sent {} messages, received {} messages, sent {} SURB replies, received {} SURB replies",
            sent_count, received_count, sent_replies, reply_count
        );
        assert!(received_count == 5, "Didn't receive all messages!");
        assert!(reply_count == 3, "Didn't receive all replies!");

        Ok(())
    }
}
