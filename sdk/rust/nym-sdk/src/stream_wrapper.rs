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
use tokio::sync::Mutex;
use tokio_util::codec::{Decoder, Encoder};
use tracing::field::debug;
use tracing::{debug, info, warn};

/**
 * TODO
 * - check all works
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
        (
            MixStreamReader {
                client: self.client,
                peer: self.peer.expect("No Peer set"),
            },
            MixStreamWriter {
                sender,
                peer: self.peer.expect("No Peer set"),
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
        codec
            .encode(input_message, &mut serialized_bytes)
            .map_err(|_| Error::MessageSendingFailure)?;
        info!("Serialized bytes: {:?}", serialized_bytes);

        self.write_all(&serialized_bytes)
            .await
            .map_err(|_| Error::MessageSendingFailure)?;
        info!("Wrote serialized bytes");
        self.flush()
            .await
            .map_err(|_| Error::MessageSendingFailure)?;
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

// TODO make peer + peer surbs optional as in Stream
pub struct MixStreamReader {
    client: MixnetClient,
    peer: Recipient,
}
impl MixStreamReader {
    /// Nym address of StreamReader's Recipient (Nym Client it will communicate with).
    pub fn peer_addr(&self) -> &Recipient {
        &self.peer
    }

    /// Our Nym address.
    pub fn local_addr(&self) -> &Recipient {
        self.client.nym_address()
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

// TODO make peer + peer surbs optional as in Stream
pub struct MixStreamWriter {
    sender: MixnetClientSender,
    peer: Recipient,
}

impl MixStreamWriter {
    // Convenience method for just piping bytes into the Mixnet.
    //
    // Commented out until new peer + surb model is added
    // pub async fn write_bytes(&mut self, data: &[u8]) -> Result<(), Error> {
    //     let input_message = if self.peer_surbs.is_some() {
    //         InputMessage::Reply {
    //             recipient_tag: (self.peer_surbs.expect("No Peer SURBs set")),
    //             data: (data.to_owned()),
    //             lane: (nym_task::connections::TransmissionLane::General),
    //             max_retransmissions: (Some(5)), // TODO check with Drazen - guessing here
    //         }
    //     } else {
    //         InputMessage::Anonymous {
    //             recipient: (self.peer.expect("No Peer set")),
    //             data: (data.to_owned()),
    //             reply_surbs: (10),
    //             lane: (nym_task::connections::TransmissionLane::General),
    //             max_retransmissions: (Some(5)), // TODO check with Drazen - guessing here
    //         }
    //     };

    //     let mut codec = InputMessageCodec {};
    //     let mut serialized_bytes = BytesMut::new();
    //     codec
    //         .encode(input_message, &mut serialized_bytes)
    //         .map_err(|_| Error::MessageSendingFailure)?;
    //     info!("Serialized bytes: {:?}", serialized_bytes);

    //     self.write_all(&serialized_bytes)
    //         .await
    //         .map_err(|_| Error::MessageSendingFailure)?;
    //     info!("Wrote serialized bytes");
    //     self.flush()
    //         .await
    //         .map_err(|_| Error::MessageSendingFailure)?;
    //     debug!("Flushed");

    //     Ok(())
    // }
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
 *
 * STREAM + STREAMREADER + STREAMWRITER
 * - anonymous replies
 * - make sure we can do TLS through this (aka get around the 'superinsecuredontuseinprod mode' flags)
 *
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

    // #[tokio::test]
    // async fn simple_send_and_receive() -> Result<(), Box<dyn std::error::Error>> {
    //     // init_logging();
    //     let receiver_socket = MixSocket::new_test().await?; // TODO change once socket impl is done
    //     let receiver_address = receiver_socket.nym_address().clone();
    //     let receiver_stream = MixStream::new(Some(receiver_socket), receiver_address.clone()).await;
    //     let mut sender_stream = MixStream::new(None, receiver_address).await;

    //     let message = b"Hello, Mixnet!";

    //     let mut receiver_for_task = receiver_stream;
    //     let receiver_task = tokio::spawn(async move {
    //         let mut buffer = [0u8; 1024];
    //         match receiver_for_task.read(&mut buffer).await {
    //             Ok(bytes_read) => {
    //                 if bytes_read > 0 {
    //                     let mut codec = ReconstructedMessageCodec {};
    //                     let mut buf = BytesMut::from(&buffer[..bytes_read]);

    //                     match codec.decode(&mut buf) {
    //                         Ok(Some(decoded_message)) => {
    //                             let received_payload = decoded_message.message;
    //                             let received_surbs = decoded_message.sender_tag;
    //                             let payload_length = received_payload.len();

    //                             info!(
    //                                 "Received {} bytes: {:?} from {:?}",
    //                                 payload_length, received_payload, received_surbs
    //                             );
    //                             return Ok((received_payload, payload_length));
    //                         }
    //                         Ok(None) => println!(
    //                             "ReconstructedMessageCodec returned None - incomplete message?"
    //                         ),
    //                         // TODO make panic
    //                         Err(e) => println!("ReconstructedMessageCodec decode error: {:?}", e),
    //                     }
    //                 }
    //                 // TODO make panic
    //                 Err(("No bytes read".to_string(), 0))
    //             }
    //             // TODO make panic
    //             Err(e) => Err((format!("Read error: {}", e), 0)),
    //         }
    //     });

    //     sender_stream.write_bytes(message).await?;
    //     info!("Sent {} bytes", message.len());

    //     let result =
    //         tokio::time::timeout(tokio::time::Duration::from_secs(15), receiver_task).await;
    //     sender_stream.disconnect().await;

    //     match result {
    //         Ok(Ok(Ok((received_payload, paylod_length)))) => {
    //             assert_eq!(
    //                 paylod_length,
    //                 message.len(),
    //                 "Length mismatch: expected {}, got {}",
    //                 message.len(),
    //                 paylod_length
    //             );

    //             assert_eq!(received_payload.as_slice(), message, "Content mismatch");
    //         }
    //         Ok(Ok(Err((error_msg, _)))) => {
    //             panic!("Receiver task failed: {}", error_msg);
    //         }
    //         Ok(Err(e)) => {
    //             panic!("Receiver task panicked: {:?}", e);
    //         }
    //         Err(_) => {
    //             panic!("Test timed out");
    //         }
    //     }
    //     Ok(())
    // }

    // #[tokio::test]
    // async fn simple_send_receive_split() -> Result<(), Box<dyn std::error::Error>> {
    //     // init_logging();
    //     let receiver_socket = MixSocket::new_test().await?; // TODO change once socket impl is done
    //     let receiver_address = receiver_socket.nym_address().clone();
    //     let sender_socket = MixSocket::new_test().await?;
    //     let sender_address = sender_socket.nym_address().clone();
    //     let receiver_stream = MixStream::new(Some(receiver_socket), sender_address.clone()).await;
    //     let sender_stream = MixStream::new(Some(sender_socket), receiver_address.clone()).await;
    //     let (mut reader, _receiver_writer) = receiver_stream.split();
    //     let (_sender_reader, mut writer) = sender_stream.split();

    //     let message = b"Hello, Mixnet Split!";

    //     let receiver_task = tokio::spawn(async move {
    //         let mut buffer = [0u8; 1024];
    //         match reader.read(&mut buffer).await {
    //             Ok(bytes_read) => {
    //                 if bytes_read > 0 {
    //                     let mut codec = ReconstructedMessageCodec {};
    //                     let mut buf = BytesMut::from(&buffer[..bytes_read]);

    //                     match codec.decode(&mut buf) {
    //                         Ok(Some(decoded_message)) => {
    //                             let received_payload = decoded_message.message;
    //                             let payload_length = received_payload.len();

    //                             info!("Received {} bytes: {:?}", payload_length, received_payload);
    //                             return Ok((received_payload, payload_length));
    //                         }
    //                         Ok(None) => println!(
    //                             "ReconstructedMessageCodec returned None - incomplete message?"
    //                         ),
    //                         Err(e) => println!("ReconstructedMessageCodec decode error: {:?}", e),
    //                     }
    //                 }
    //                 Err(("No bytes read".to_string(), 0))
    //             }
    //             Err(e) => Err((format!("Read error: {}", e), 0)),
    //         }
    //     });

    //     writer.write_bytes(message).await?;
    //     info!("Sent {} bytes", message.len());

    //     let result =
    //         tokio::time::timeout(tokio::time::Duration::from_secs(15), receiver_task).await;

    //     match result {
    //         Ok(Ok(Ok((received_payload, payload_length)))) => {
    //             assert_eq!(
    //                 payload_length,
    //                 message.len(),
    //                 "Length mismatch: expected {}, got {}",
    //                 message.len(),
    //                 payload_length
    //             );

    //             assert_eq!(received_payload.as_slice(), message, "Content mismatch");
    //         }
    //         Ok(Ok(Err((error_msg, _)))) => {
    //             panic!("Receiver task failed: {}", error_msg);
    //         }
    //         Ok(Err(e)) => {
    //             panic!("Receiver task panicked: {:?}", e);
    //         }
    //         Err(_) => {
    //             panic!("Test timed out");
    //         }
    //     }
    //     Ok(())
    // }

    // #[tokio::test]
    // async fn concurrent_send_receive() -> Result<(), Box<dyn std::error::Error>> {
    //     // init_logging();
    //     let socket = MixSocket::new_test().await?; // TODO change once socket impl is done
    //     let addr = socket.nym_address().clone();
    //     let stream = MixStream::new(Some(socket), addr.clone()).await;
    //     let (mut reader, mut writer) = stream.split();

    //     let writer_task = tokio::spawn(async move {
    //         for i in 0..20 {
    //             let msg = format!("Message {}", i);
    //             match writer.write_bytes(msg.as_bytes()).await {
    //                 Ok(()) => {}
    //                 Err(e) => {
    //                     return Err(format!("Write failed: {:?}", e));
    //                 }
    //             }
    //         }
    //         Ok(())
    //     });

    //     let reader_task = tokio::spawn(async move {
    //         let mut received = 0;
    //         let mut buffer = [0u8; 1024];

    //         loop {
    //             match tokio::time::timeout(
    //                 tokio::time::Duration::from_secs(5),
    //                 reader.read(&mut buffer),
    //             )
    //             .await
    //             {
    //                 Ok(Ok(n)) if n > 0 => {
    //                     let mut codec = ReconstructedMessageCodec {};
    //                     let mut buf = BytesMut::from(&buffer[..n]);
    //                     if let Ok(Some(_)) = codec.decode(&mut buf) {
    //                         received += 1;
    //                     }
    //                 }
    //                 _ => break,
    //             }
    //         }
    //         received
    //     });

    //     match writer_task.await {
    //         Ok(Ok(())) => {}
    //         Ok(Err(e)) => panic!("Writer task failed: {}", e),
    //         Err(e) => panic!("Writer task panicked: {:?}", e),
    //     }

    //     let count = reader_task.await?;

    //     info!("Sent 20 messages, received {}", count);
    //     assert!(count == 20);
    //     Ok(())
    // }

    #[tokio::test]
    async fn simple_surb_reply() -> Result<(), Box<dyn std::error::Error>> {
        init_logging();

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
            }
            _ => panic!("Failed to receive reply"),
        }

        receiver_stream.disconnect().await;
        sender_stream.disconnect().await;

        Ok(())
    }
}
