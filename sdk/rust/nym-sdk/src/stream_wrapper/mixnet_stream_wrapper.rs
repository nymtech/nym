// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet::InputMessage;
use crate::mixnet::MixnetMessageSender;
use crate::mixnet::{MixnetClient, MixnetClientSender, Recipient};
use crate::Error;
use bytes::BytesMut;
use nym_client_core::client::inbound_messages::InputMessageCodec;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use nym_sphinx::receiver::{ReconstructedMessage, ReconstructedMessageCodec};
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::sync::oneshot;
use tokio_util::codec::{Encoder, Framed, FramedRead, FramedWrite};
use tracing::{debug, info, warn};

/// High-level methods for MixnetClient
impl MixnetClient {
    /// Send data to a recipient with reply SURBs.
    ///
    /// This is a high-level method that abstracts away codec details.
    ///
    /// # Arguments
    /// * `recipient` - The Nym address to send to
    /// * `data` - The message payload
    /// * `reply_surbs` - Number of Single Use Reply Blocks to include for anonymous replies
    pub async fn send_to(
        &mut self,
        recipient: &Recipient,
        data: &[u8],
        reply_surbs: Option<u32>, // TODO make this option - if None then use default
    ) -> Result<(), Error> {
        let msg = InputMessage::Anonymous {
            recipient: *recipient,
            data: data.to_vec(),
            reply_surbs: reply_surbs.unwrap_or(10),
            lane: nym_task::connections::TransmissionLane::General,
            max_retransmissions: Some(5),
        };
        MixnetMessageSender::send(self, msg).await
    }

    /// Send a reply using a previously received SURB tag.
    ///
    /// This enables anonymous replies without knowing the original sender's address.
    ///
    /// # Arguments
    /// * `recipient_tag` - The SURB tag from a received message
    /// * `data` - The reply payload
    pub async fn send_reply(
        &mut self,
        recipient_tag: AnonymousSenderTag,
        data: &[u8],
    ) -> Result<(), Error> {
        let msg = InputMessage::Reply {
            recipient_tag,
            data: data.to_vec(),
            lane: nym_task::connections::TransmissionLane::General,
            max_retransmissions: Some(5),
        };
        MixnetMessageSender::send(self, msg).await
    }

    /// Receive the next message, awaiting until one arrives.
    ///
    /// This method blocks until a complete message is received and decoded.
    /// Uses framed reading internally for efficient message handling.
    ///
    /// # Returns
    /// A `ReconstructedMessage` containing the data and optional sender tag
    pub async fn recv(&mut self) -> Result<ReconstructedMessage, Error> {
        use futures::StreamExt;

        let mut framed = self.framed_read();
        framed
            .next()
            .await
            .ok_or_else(|| {
                Error::IoError(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Connection closed",
                ))
            })?
            .map_err(Error::from)
    }

    /// Try to receive a message without blocking.
    ///
    /// Returns `None` immediately if no complete message is available.
    ///
    /// # Returns
    /// `Some(ReconstructedMessage)` if a message is available, `None` otherwise
    pub async fn try_recv(&mut self) -> Result<Option<ReconstructedMessage>, Error> {
        Ok(self.wait_for_messages().await.and_then(|mut msgs| {
            if msgs.is_empty() {
                None
            } else {
                Some(msgs.remove(0))
            }
        }))
    }

    /// Convert into a framed reader for stream-like message handling.
    ///
    /// Returns a `FramedRead` that automatically decodes `ReconstructedMessage`s.
    ///
    /// # Example
    /// ```no_run
    /// use futures::StreamExt;
    ///
    /// let mut framed = client.into_framed_read();
    /// while let Some(msg) = framed.next().await {
    ///     let msg = msg?;
    ///     println!("Received: {:?}", msg.message);
    /// }
    /// ```
    pub fn into_framed_read(self) -> FramedRead<Self, ReconstructedMessageCodec> {
        FramedRead::new(self, ReconstructedMessageCodec {})
    }

    /// Create a framed reader without consuming self.
    ///
    /// Useful when you need to keep the client for other operations.
    pub fn framed_read(&mut self) -> FramedRead<&mut Self, ReconstructedMessageCodec> {
        FramedRead::new(self, ReconstructedMessageCodec {})
    }
}

/// High-level methods for MixnetClientSender
impl MixnetClientSender {
    /// Send data to a recipient with optional reply SURBs.
    ///
    /// This is a high-level method that abstracts away codec details.
    /// Identical to `MixnetClient::send_to` but for the split sender.
    ///
    /// # Arguments
    /// * `recipient` - The Nym address to send to
    /// * `data` - The message payload
    /// * `reply_surbs` - Number of Single Use Reply Blocks to include
    pub async fn send_to(
        &mut self,
        recipient: &Recipient,
        data: &[u8],
        reply_surbs: u32, // TODO make this option - if None then use default
    ) -> Result<(), Error> {
        let msg = InputMessage::Anonymous {
            recipient: *recipient,
            data: data.to_vec(),
            reply_surbs,
            lane: nym_task::connections::TransmissionLane::General,
            max_retransmissions: Some(5),
        };
        MixnetMessageSender::send(self, msg).await
    }

    /// Send a reply using a previously received SURB tag.
    ///
    /// Identical to `MixnetClient::send_reply` but for the split sender.
    ///
    /// # Arguments
    /// * `recipient_tag` - The SURB tag from a received message
    /// * `data` - The reply payload
    pub async fn send_reply(
        &mut self,
        recipient_tag: AnonymousSenderTag,
        data: &[u8],
    ) -> Result<(), Error> {
        let msg = InputMessage::Reply {
            recipient_tag,
            data: data.to_vec(),
            lane: nym_task::connections::TransmissionLane::General,
            max_retransmissions: Some(5),
        };
        MixnetMessageSender::send(self, msg).await
    }
}

/// A mixnet socket, similar to `TcpSocket`.
///
/// Provides a high-level interface for creating mixnet connections
/// without dealing with codecs or low-level message handling.
///
/// MixSocket follows the structure of something like `Tokio::net::TcpSocket`
/// with regards to setup and interface, breakdown from TcpSocket to TcpStream, etc.
/// However, we can't map this one to one onto the TcpSocket as there isn't really a
/// concept of binding to a port with the MixnetClient; it connects to its Gateway
/// and then just accepts incoming messages from the Gw via the Websocket connection.
pub struct MixSocket {
    pub inner: MixnetClient,
}

impl MixSocket {
    // TODO MAKE CONFIGURABLE RE NETWORK - SEE TCPPROXY CONFIG
    /// Create a new socket connected to the mixnet.
    ///
    /// Initializes a new mixnet client and prepares it for connections.
    pub async fn new() -> Result<Self, Error> {
        let inner = MixnetClient::connect_new().await?;
        Ok(MixSocket { inner })
    }

    /// Connect to a specific peer and return a `MixStream`.
    ///
    /// Similar to `TcpSocket::connect`, establishes a connection
    /// to a peer identified by their Nym address.
    ///
    /// # Arguments
    /// * `recipient` - The Nym address of the peer to connect to
    pub async fn connect(self, recipient: Recipient) -> Result<MixStream, Error> {
        Ok(MixStream {
            client: self.inner,
            peer: Some(recipient),
            peer_surb_tag: None,
        })
    }

    /// Convert socket into a listening stream without a specific peer.
    ///
    /// Creates a stream that can receive messages from anyone and reply
    /// anonymously using SURBs. This is the mixnet equivalent of a listening socket.
    ///
    /// # Example
    /// ```no_run
    /// let socket = MixSocket::new().await?;
    /// let mut listener = socket.into_stream();
    ///
    /// // Receive from anyone
    /// let msg = listener.recv().await?;
    ///
    /// // Reply anonymously using SURB
    /// if let Some(surb) = msg.sender_tag {
    ///     listener.store_surb_tag(surb);
    ///     listener.send(b"Reply").await?;
    /// }
    /// ```
    pub fn into_stream(self) -> MixStream {
        MixStream {
            client: self.inner,
            peer: None,
            peer_surb_tag: None,
        }
    }

    /// Get our Nym address (like `TcpSocket::local_addr`).
    pub fn local_addr(&self) -> &Recipient {
        self.inner.nym_address()
    }

    /// Get a reference to the underlying `MixnetClient`.
    pub fn get_ref(&self) -> &MixnetClient {
        &self.inner
    }

    /// Get a mutable reference to the underlying `MixnetClient`.
    pub fn get_mut(&mut self) -> &mut MixnetClient {
        &mut self.inner
    }

    /// Consume the socket and return the underlying `MixnetClient`.
    pub fn into_inner(self) -> MixnetClient {
        self.inner
    }
}

/// A mixnet stream, similar to `TcpStream`.
///
/// Provides bidirectional communication with a peer over the mixnet.
/// Can operate in two modes:
/// - Connected mode: Has a specific peer address
/// - Listening mode: No peer, receives from anyone and replies via SURBs
pub struct MixStream {
    pub client: MixnetClient,
    peer: Option<Recipient>,
    peer_surb_tag: Option<AnonymousSenderTag>,
}

impl MixStream {
    // TODO MAKE CONFIGURABLE RE NETWORK - see TCPPROXY SETUP
    /// Create a `MixStream` from an optional socket and optional peer.
    ///
    /// If no socket is provided, creates a new one automatically.
    /// If no peer is provided, creates a listening stream.
    ///
    /// # Arguments
    /// * `socket` - Optional existing socket to use
    /// * `peer` - Optional Nym address to connect to (None = listening mode)
    pub async fn new(socket: Option<MixSocket>, peer: Option<Recipient>) -> Self {
        let client = match socket {
            Some(socket) => socket.into_inner(),
            None => MixnetClient::connect_new().await.unwrap(),
        };
        Self {
            client,
            peer,
            peer_surb_tag: None,
        }
    }

    /// Create a listening stream that receives from anyone.
    ///
    /// This stream has no specific peer and relies on SURBs for replies.
    /// Perfect for server-like applications that respond to anonymous requests.
    ///
    /// # Example
    /// ```no_run
    /// let mut listener = MixStream::listen().await?;
    /// let msg = listener.recv().await?;
    /// if let Some(surb) = msg.sender_tag {
    ///     listener.store_surb_tag(surb);
    ///     listener.send(b"Response").await?;
    /// }
    /// ```
    pub async fn listen() -> Result<Self, Error> {
        let client = MixnetClient::connect_new().await?;
        Ok(Self {
            client,
            peer: None,
            peer_surb_tag: None,
        })
    }

    /// Create a new `MixStream` and connect to a peer (like `TcpStream::connect`).
    ///
    /// This is a convenience method that creates both the socket and stream.
    ///
    /// # Arguments
    /// * `peer` - The Nym address to connect to
    pub async fn connect(peer: Recipient) -> Result<Self, Error> {
        let socket = MixSocket::new().await?;
        Ok(socket.connect(peer).await?)
    }

    /// Get the peer's Nym address (like `TcpStream::peer_addr`).
    ///
    /// Returns `None` if this is a listening stream.
    pub fn peer_addr(&self) -> Option<&Recipient> {
        self.peer.as_ref()
    }

    /// Get our local Nym address (like `TcpStream::local_addr`).
    pub fn local_addr(&self) -> &Recipient {
        self.client.nym_address()
    }

    /// Store a SURB tag for sending anonymous replies.
    ///
    /// SURB tags are typically extracted from received messages and enable
    /// replying without knowing the original sender's address.
    ///
    /// # Arguments
    /// * `surbs` - The sender tag from a received message
    pub fn store_surb_tag(&mut self, surbs: AnonymousSenderTag) {
        self.peer_surb_tag = Some(surbs);
    }

    /// Get the currently stored SURB tag, if any.
    pub fn surbs(&self) -> Option<AnonymousSenderTag> {
        self.peer_surb_tag
    }

    /// Send data to the peer or via SURB.
    ///
    /// Behavior depends on stream state:
    /// - If SURB tag is stored: Uses SURBs from denoted bucket for anonymous reply
    /// - If peer is set: Sends to peer with new reply SURBs
    /// - If neither: Returns error
    ///
    /// # Arguments
    /// * `data` - The message payload to send
    pub async fn send(&mut self, data: &[u8]) -> Result<(), Error> {
        match (self.peer_surb_tag, self.peer) {
            (Some(tag), _) => {
                // Have SURB tag - use it for anonymous reply
                self.client.send_reply(tag, data).await
            }
            (None, Some(peer)) => {
                // Have peer - send with default # of SURBs
                self.client.send_to(&peer, data, None).await
            }
            (None, None) => {
                // No peer, no SURB - can't send
                Err(Error::MixStreamNoPeerOrSurb)
            }
        }
    }

    /// Receive the next message, awaiting until one arrives.
    ///
    /// Blocks until a complete message is received.
    ///
    /// # Returns
    /// A `ReconstructedMessage` containing the data and optional sender tag
    pub async fn recv(&mut self) -> Result<ReconstructedMessage, Error> {
        self.client.recv().await
    }

    /// Try to receive a message without blocking.
    ///
    /// Returns `None` immediately if no message is available.
    /// Low-level method primarily for debugging.
    ///
    /// # Returns
    /// `Some(ReconstructedMessage)` if available, `None` otherwise
    pub async fn try_recv(&mut self) -> Result<Option<ReconstructedMessage>, Error> {
        self.client.try_recv().await
    }

    /// Split the stream for concurrent read/write operations (like `TcpStream::split`).
    ///
    /// Returns separate reader and writer halves that can be used concurrently.
    /// The reader and writer share SURB tags via an internal channel.
    pub fn split(self) -> (MixStreamReader, MixStreamWriter) {
        debug!("Splitting MixStream");
        let sender = self.client.split_sender();
        debug!("Split MixStream into Reader and Writer");
        let (surb_tx, surb_rx) = oneshot::channel();
        (
            MixStreamReader {
                client: self.client,
                peer: self.peer,
                peer_surb_tag: self.peer_surb_tag,
                surb_tx: Some(surb_tx),
            },
            MixStreamWriter {
                sender,
                peer: self.peer,
                peer_surb_tag: self.peer_surb_tag,
                surb_rx: Some(surb_rx),
            },
        )
    }

    /// Convert into a framed stream for bidirectional message handling.
    ///
    /// Returns a `Framed` that automatically encodes/decodes messages.
    ///
    /// # Example
    /// ```no_run
    /// use futures::{SinkExt, StreamExt};
    ///
    /// let mut framed = stream.into_framed();
    ///
    /// // Send
    /// framed.send(InputMessage::Anonymous { ... }).await?;
    ///
    /// // Receive
    /// if let Some(msg) = framed.next().await {
    ///     let msg = msg?;
    ///     println!("Received: {:?}", msg.message);
    /// }
    /// ```
    pub fn into_framed(self) -> Framed<MixnetClient, ReconstructedMessageCodec> {
        Framed::new(self.client, ReconstructedMessageCodec {})
    }

    /// Convert into a framed reader for receiving messages.
    ///
    /// Returns a `FramedRead` that automatically decodes `ReconstructedMessage`s.
    pub fn into_framed_read(self) -> FramedRead<MixnetClient, ReconstructedMessageCodec> {
        FramedRead::new(self.client, ReconstructedMessageCodec {})
    }

    /// Create a framed reader without consuming self.
    pub fn framed_read(&mut self) -> FramedRead<&mut MixnetClient, ReconstructedMessageCodec> {
        FramedRead::new(&mut self.client, ReconstructedMessageCodec {})
    }

    /// Write bytes using the codec.
    ///
    /// This is a low-level method for debugging.
    /// Prefer using `send()` for normal operations.
    ///
    /// # Arguments
    /// * `data` - Raw bytes to encode and send
    pub async fn write_bytes(&mut self, data: &[u8]) -> Result<(), Error> {
        let input_message = match (self.peer_surb_tag, self.peer) {
            (Some(tag), _) => {
                info!("Writing {} bytes, sending with SURBs", data.len());
                InputMessage::Reply {
                    recipient_tag: tag,
                    data: data.to_owned(),
                    lane: nym_task::connections::TransmissionLane::General,
                    max_retransmissions: Some(5),
                }
            }
            (None, Some(peer)) => {
                info!("Writing {} bytes", data.len());
                InputMessage::Anonymous {
                    recipient: peer,
                    data: data.to_owned(),
                    reply_surbs: 10,
                    lane: nym_task::connections::TransmissionLane::General,
                    max_retransmissions: Some(5),
                }
            }
            (None, None) => {
                return Err(Error::MixStreamNoPeerOrSurb);
            }
        };

        let mut codec = InputMessageCodec {};
        let mut serialized_bytes = BytesMut::new();
        codec.encode(input_message, &mut serialized_bytes)?;
        self.write_all(&serialized_bytes).await?;
        debug!("Wrote serialized bytes");
        self.flush().await?;
        debug!("Flushed");

        Ok(())
    }

    /// Wait for messages using the low-level interface.
    ///
    /// This is a method for debugging only.
    /// Prefer using `recv()` for normal operations.
    pub async fn wait_for_messages(&mut self) -> Option<Vec<ReconstructedMessage>> {
        self.client.wait_for_messages().await
    }

    /// Disconnect from the mixnet (like `TcpStream::shutdown`).
    ///
    /// Gracefully closes the connection.
    pub async fn shutdown(self) -> Result<(), Error> {
        debug!("Disconnecting");
        self.client.disconnect().await;
        debug!("Disconnected");
        Ok(())
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

/// Read half of a split `MixStream`.
///
/// Provides read-only access to the stream with automatic SURB tag handling.
pub struct MixStreamReader {
    client: MixnetClient,
    peer: Option<Recipient>,
    peer_surb_tag: Option<AnonymousSenderTag>,
    surb_tx: Option<oneshot::Sender<AnonymousSenderTag>>,
}

impl MixStreamReader {
    /// Get the peer's Nym address.
    pub fn peer_addr(&self) -> Option<&Recipient> {
        self.peer.as_ref()
    }

    /// Get our local Nym address.
    pub fn local_addr(&self) -> &Recipient {
        self.client.nym_address()
    }

    /// Store a SURB tag and forward it to the writer half.
    ///
    /// Automatically sends the tag to the paired `MixStreamWriter` via
    /// an internal channel for seamless anonymous replies.
    ///
    /// # Arguments
    /// * `surbs` - The sender tag to store and forward
    pub fn store_surb_tag(&mut self, surbs: AnonymousSenderTag) {
        self.peer_surb_tag = Some(surbs);
        if let Some(tx) = self.surb_tx.take() {
            match tx.send(surbs) {
                Ok(()) => debug!("Sent SURBs to MixStreamWriter"),
                Err(e) => warn!("Could not send SURBs to MixStreamWriter with err: {}", e),
            }
        }
    }

    /// Get the currently stored SURB tag, if any.
    pub fn surbs(&self) -> Option<AnonymousSenderTag> {
        self.peer_surb_tag
    }

    /// Receive the next message, awaiting until one arrives.
    ///
    /// Automatically extracts and stores
    /// SURB tags from received messages.
    ///
    /// # Returns
    /// A `ReconstructedMessage` containing the data and optional sender tag
    pub async fn recv(&mut self) -> Result<ReconstructedMessage, Error> {
        use futures::StreamExt;

        let mut framed = self.framed();
        let msg = framed
            .next()
            .await
            .ok_or_else(|| {
                Error::IoError(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Connection closed",
                ))
            })?
            .map_err(Error::from)?;

        // Auto-store SURB tag if present
        if let Some(surb) = msg.sender_tag {
            self.store_surb_tag(surb);
        }

        Ok(msg)
    }

    /// Try to receive a message without blocking.
    ///
    /// Useful for debugging. Automatically stores
    /// SURB tags if present. Prefer `recv()` in
    /// normal use.
    ///
    /// # Returns
    /// `Some(ReconstructedMessage)` if available, `None` otherwise
    pub async fn try_recv(&mut self) -> Result<Option<ReconstructedMessage>, Error> {
        let msg_opt = self.client.wait_for_messages().await.and_then(|mut msgs| {
            if msgs.is_empty() {
                None
            } else {
                Some(msgs.remove(0))
            }
        });

        if let Some(ref msg) = msg_opt {
            if let Some(surb) = msg.sender_tag {
                self.store_surb_tag(surb);
            }
        }

        Ok(msg_opt)
    }

    /// Convert into a framed reader for stream-like message handling.
    ///
    /// Returns a `FramedRead` that automatically decodes `ReconstructedMessage`s
    /// and handles SURB tag extraction.
    ///
    /// # Example
    /// ```no_run
    /// use futures::StreamExt;
    ///
    /// let mut framed = reader.into_framed();
    /// while let Some(msg) = framed.next().await {
    ///     let msg = msg?;
    ///     if let Some(surb) = msg.sender_tag {
    ///         // SURB automatically stored
    ///     }
    /// }
    /// ```
    pub fn into_framed(self) -> FramedRead<MixnetClient, ReconstructedMessageCodec> {
        FramedRead::new(self.client, ReconstructedMessageCodec {})
    }

    /// Create a framed reader without consuming self.
    pub fn framed(&mut self) -> FramedRead<&mut MixnetClient, ReconstructedMessageCodec> {
        FramedRead::new(&mut self.client, ReconstructedMessageCodec {})
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

/// Write half of a split `MixStream`.
///
/// Provides write-only access to the stream with automatic SURB tag
/// synchronization from the reader half.
pub struct MixStreamWriter {
    sender: MixnetClientSender,
    peer: Option<Recipient>,
    peer_surb_tag: Option<AnonymousSenderTag>,
    surb_rx: Option<oneshot::Receiver<AnonymousSenderTag>>,
}

impl MixStreamWriter {
    /// Send data to the connected peer or via SURB.
    ///
    /// Automatically checks for SURB tags
    /// received by the paired reader and uses them for replies.
    ///
    /// # Arguments
    /// * `data` - The message payload to send
    pub async fn send(&mut self, data: &[u8]) -> Result<(), Error> {
        // Check for SURB updates from reader
        if self.peer_surb_tag.is_none() {
            if let Some(rx) = self.surb_rx.as_mut() {
                if let Ok(surbs) = rx.try_recv() {
                    self.peer_surb_tag = Some(surbs);
                }
            }
        }

        match (self.peer_surb_tag, self.peer) {
            (Some(tag), _) => {
                // Have SURB tag - use it for anonymous reply
                self.sender.send_reply(tag, data).await
            }
            (None, Some(peer)) => {
                // Have peer - send with SURBs
                self.sender.send_to(&peer, data, 10).await
            }
            (None, None) => {
                // No peer, no SURB tag - can't send
                Err(Error::MixStreamNoPeerOrSurb)
            }
        }
    }

    /// Write bytes using the codec.
    ///
    /// Used for debugging and compatibility.
    /// Prefer using `send()` for normal operations.
    ///
    /// # Arguments
    /// * `data` - Raw bytes to encode and send
    pub async fn write_bytes(&mut self, data: &[u8]) -> Result<(), Error> {
        if self.peer_surb_tag.is_none() {
            if let Some(rx) = self.surb_rx.as_mut() {
                if let Ok(surbs) = rx.try_recv() {
                    self.peer_surb_tag = Some(surbs);
                }
            }
        }

        let input_message = match (self.peer_surb_tag, self.peer) {
            (Some(tag), _) => InputMessage::Reply {
                recipient_tag: tag,
                data: data.to_owned(),
                lane: nym_task::connections::TransmissionLane::General,
                max_retransmissions: Some(5),
            },
            (None, Some(peer)) => InputMessage::Anonymous {
                recipient: peer,
                data: data.to_owned(),
                reply_surbs: 10,
                lane: nym_task::connections::TransmissionLane::General,
                max_retransmissions: Some(5),
            },
            (None, None) => {
                return Err(Error::MixStreamNoPeerOrSurb);
            }
        };

        let mut codec = InputMessageCodec {};
        let mut serialized_bytes = BytesMut::new();
        codec.encode(input_message, &mut serialized_bytes)?;

        self.write_all(&serialized_bytes).await?;
        debug!("Wrote serialized bytes");
        self.flush().await?;
        debug!("Flushed");

        Ok(())
    }

    /// Convert into a framed writer for sending messages.
    ///
    /// Returns a `FramedWrite` that automatically encodes `InputMessage`s.
    ///
    /// # Example
    /// ```no_run
    /// use futures::SinkExt;
    ///
    /// let mut framed = writer.into_framed();
    /// framed.send(InputMessage::Anonymous {
    ///     recipient,
    ///     data: b"Hello".to_vec(),
    ///     reply_surbs: 10,
    ///     lane: TransmissionLane::General,
    ///     max_retransmissions: Some(5),
    /// }).await?;
    /// ```
    pub fn into_framed(self) -> FramedWrite<MixnetClientSender, InputMessageCodec> {
        FramedWrite::new(self.sender, InputMessageCodec {})
    }

    /// Create a framed writer without consuming self.
    pub fn framed(&mut self) -> FramedWrite<&mut MixnetClientSender, InputMessageCodec> {
        FramedWrite::new(&mut self.sender, InputMessageCodec {})
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn init_logging() {
        if tracing::dispatcher::has_been_set() {
            return;
        }
        INIT.call_once(|| {
            nym_bin_common::logging::setup_tracing_logger();
        });
    }

    impl MixSocket {
        pub async fn new_test() -> Result<Self, Error> {
            let inner = MixnetClient::connect_new().await?;
            Ok(MixSocket { inner })
        }
    }

    #[tokio::test]
    async fn simple_send_recv() -> Result<(), Box<dyn std::error::Error>> {
        init_logging();

        // Create listener (no peer)
        let listener_socket = MixSocket::new_test().await?;
        let listener_address = *listener_socket.local_addr();
        let mut listener_stream = listener_socket.into_stream();

        // Create sender connected to listener
        let mut sender_stream = MixStream::connect(listener_address).await?;

        // Sender initiates with SURBs
        sender_stream.send(b"Hello, Mixnet!").await?;
        info!("Sent initial message");

        // Listener receives and extracts SURB
        let msg =
            tokio::time::timeout(tokio::time::Duration::from_secs(30), listener_stream.recv())
                .await??;

        assert_eq!(msg.message, b"Hello, Mixnet!");
        info!("Received initial message");

        // Store SURB and reply anonymously
        if let Some(surbs) = msg.sender_tag {
            listener_stream.store_surb_tag(surbs);
            listener_stream.send(b"Hello back!").await?;
            info!("Sent reply using SURB");
        }

        // Sender receives anonymous reply
        let reply =
            tokio::time::timeout(tokio::time::Duration::from_secs(30), sender_stream.recv())
                .await??;

        assert_eq!(reply.message, b"Hello back!");
        info!("Received SURB reply");

        Ok(())
    }

    #[tokio::test]
    async fn framed() -> Result<(), Box<dyn std::error::Error>> {
        init_logging();
        use futures::StreamExt;

        let receiver_socket = MixSocket::new_test().await?;
        let receiver_address = *receiver_socket.local_addr();
        let mut receiver_stream = receiver_socket.into_stream();

        let sender_socket = MixSocket::new_test().await?;
        let sender_stream = sender_socket.connect(receiver_address).await?;

        let (sender_reader, mut sender_writer) = sender_stream.split();
        let mut sender_framed = sender_reader.into_framed();

        sender_writer.send(b"Hello via framed!").await?;
        info!("Sent message");

        let msg =
            tokio::time::timeout(tokio::time::Duration::from_secs(30), receiver_stream.recv())
                .await??;

        assert_eq!(msg.message, b"Hello via framed!");

        if let Some(surb) = msg.sender_tag {
            receiver_stream.store_surb_tag(surb);
            receiver_stream.send(b"Reply via framed!").await?;
            info!("Sent reply");
        }

        let reply =
            tokio::time::timeout(tokio::time::Duration::from_secs(30), sender_framed.next())
                .await?
                .ok_or("No reply received")??;

        assert_eq!(reply.message, b"Reply via framed!");

        Ok(())
    }

    #[tokio::test]
    async fn split_concurrent() -> Result<(), Box<dyn std::error::Error>> {
        init_logging();

        let sender_socket = MixSocket::new_test().await?;
        let receiver_socket = MixSocket::new_test().await?;
        let receiver_address = *receiver_socket.local_addr();

        let sender_stream = MixStream::new(Some(sender_socket), Some(receiver_address)).await;
        let receiver_stream = receiver_socket.into_stream();

        let (mut sender_reader, mut sender_writer) = sender_stream.split();
        let (mut receiver_reader, mut receiver_writer) = receiver_stream.split();

        let message_back_and_forth = 5;

        let sender_task = tokio::spawn(async move {
            for i in 0..message_back_and_forth {
                let msg = format!("Message {}", i);
                sender_writer.send(msg.as_bytes()).await?;
                info!("Sent message {}", i);
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            }
            Ok::<_, Error>(sender_writer)
        });

        let receiver_task = tokio::spawn(async move {
            let mut received_count = 0;
            while received_count < 5 {
                match tokio::time::timeout(
                    tokio::time::Duration::from_secs(15),
                    receiver_reader.recv(),
                )
                .await
                {
                    Ok(Ok(msg)) => {
                        info!("Received: {}", String::from_utf8_lossy(&msg.message));
                        received_count += 1;
                        let reply = format!("Reply {}", received_count);
                        receiver_writer.send(reply.as_bytes()).await?;
                        info!("Sent reply {}", received_count);
                        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                    }
                    Err(_) => {
                        warn!("Timeout waiting for message {}", received_count);
                        break;
                    }
                    Ok(Err(e)) => {
                        warn!("Error receiving message: {}", e);
                        break;
                    }
                }
            }

            Ok::<_, Error>((receiver_reader, receiver_writer, received_count))
        });

        let reply_reader_task = tokio::spawn(async move {
            let mut reply_count = 0;
            while reply_count < message_back_and_forth {
                match tokio::time::timeout(
                    tokio::time::Duration::from_secs(20),
                    sender_reader.recv(),
                )
                .await
                {
                    Ok(Ok(msg)) => {
                        let reply_text = String::from_utf8_lossy(&msg.message);
                        info!("Received reply: {}", reply_text);
                        assert!(reply_text.contains("Reply"));
                        reply_count += 1;
                    }
                    Err(_) => {
                        warn!("Timeout waiting for reply {}", reply_count);
                        if reply_count == 0 {
                            panic!("No replies received");
                        }
                        break;
                    }
                    Ok(Err(e)) => {
                        warn!("Error receiving reply: {}", e);
                        break;
                    }
                }
            }

            Ok::<_, Error>(reply_count)
        });

        let _ = sender_task.await??;
        let (_, _, received_count) = receiver_task.await??;
        let reply_count = reply_reader_task.await??;

        info!(
            "Received {} messages, {} replies",
            received_count, reply_count
        );
        assert!(received_count >= 3, "Should receive at least 3 messages");
        assert!(reply_count >= 1, "Should receive at least 1 reply");

        Ok(())
    }
}
