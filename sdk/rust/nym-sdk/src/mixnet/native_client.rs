use crate::mixnet::client::MixnetClientBuilder;
use crate::mixnet::client::DEFAULT_NUMBER_OF_SURBS;
use crate::mixnet::stream::{MixnetListener, MixnetStream};
use crate::mixnet::traits::MixnetMessageSender;
use crate::{Error, Result};
use async_trait::async_trait;
use futures::{ready, Stream, StreamExt};
use log::{debug, error};
use nym_client_core::client::base_client::GatewayConnection;
use nym_client_core::client::mix_traffic::ClientRequestSender;
use nym_client_core::client::{
    base_client::{ClientInput, ClientOutput, ClientState},
    inbound_messages::InputMessage,
    received_buffer::ReconstructedMessagesReceiver,
};
use nym_client_core::config::{ForgetMe, RememberMe};
use nym_crypto::asymmetric::ed25519;
use nym_gateway_requests::ClientRequest;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::{params::PacketType, receiver::ReconstructedMessage};
use nym_statistics_common::clients::{ClientStatsEvents, ClientStatsSender};
use nym_task::connections::{ConnectionCommandSender, LaneQueueLengths};
use nym_task::ShutdownTracker;
use nym_topology::{NymRouteProvider, NymTopology};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::sync::RwLockReadGuard;
use tokio_util::sync::CancellationToken;

/// Client connected to the Nym mixnet.
///
/// `MixnetClient` operates in one of two mutually exclusive modes:
///
/// - **Message mode** (default) — send/receive discrete payloads via
///   [`send_plain_message`](MixnetMessageSender::send_plain_message) and
///   [`wait_for_messages`](Self::wait_for_messages).
/// - **[Stream mode](super::stream)** — persistent
///   [`AsyncRead`](tokio::io::AsyncRead) + [`AsyncWrite`](tokio::io::AsyncWrite)
///   byte channels via [`open_stream`](Self::open_stream) and
///   [`listener`](Self::listener). Activated on first stream call;
///   message-mode methods return
///   [`Error::StreamModeActive`](crate::Error::StreamModeActive) thereafter.
///
/// # Quick start — messages
///
/// ```no_run
/// use nym_sdk::mixnet::{self, MixnetMessageSender};
///
/// # #[tokio::main]
/// # async fn main() {
/// let mut client = mixnet::MixnetClient::connect_new().await.unwrap();
/// let addr = *client.nym_address();
///
/// client.send_plain_message(addr, "hello").await.unwrap();
///
/// if let Some(msgs) = client.wait_for_messages().await {
///     for m in msgs {
///         println!("{}", String::from_utf8_lossy(&m.message));
///     }
/// }
/// client.disconnect().await;
/// # }
/// ```
///
/// # Quick start — streams
///
/// ```no_run
/// use nym_sdk::mixnet;
/// use tokio::io::{AsyncReadExt, AsyncWriteExt};
///
/// # #[tokio::main]
/// # async fn main() {
/// let mut sender = mixnet::MixnetClient::connect_new().await.unwrap();
/// let mut receiver = mixnet::MixnetClient::connect_new().await.unwrap();
/// let recv_addr = *receiver.nym_address();
///
/// let mut listener = receiver.listener().unwrap();
/// let mut tx = sender.open_stream(recv_addr, None).await.unwrap();
/// let mut rx = listener.accept().await.unwrap();
///
/// tx.write_all(b"hello stream").await.unwrap();
/// tx.flush().await.unwrap();
///
/// let mut buf = vec![0u8; 1024];
/// let n = rx.read(&mut buf).await.unwrap();
/// println!("read {} bytes", n);
///
/// sender.disconnect().await;
/// receiver.disconnect().await;
/// # }
/// ```
///
/// # Shutdown
///
/// **Always call [`disconnect`](Self::disconnect) before dropping.** The client
/// runs background tasks (gateway connection, topology refresh, SURB management)
/// that need a coordinated shutdown. Dropping without disconnecting will leak
/// these tasks and may leave state files in an inconsistent state.
pub struct MixnetClient {
    /// The nym address of this connected client.
    pub(crate) nym_address: Recipient,

    pub(crate) identity_keys: Arc<ed25519::KeyPair>,

    /// Input to the client from the users perspective. This can be either data to send or control
    /// messages.
    pub(crate) client_input: ClientInput,

    /// Output from the client from the users perspective. This is typically messages arriving from
    /// the mixnet.
    #[allow(dead_code)]
    pub(crate) client_output: ClientOutput,

    /// The current state of the client that is exposed to the user. This includes things like
    /// current message send queue length.
    pub(crate) client_state: ClientState,

    /// A channel for messages arriving from the mixnet after they have been reconstructed.
    /// Taken by the stream router on stream mode activation, `None` thereafter.
    pub(crate) reconstructed_receiver: Option<ReconstructedMessagesReceiver>,

    /// A channel for sending stats event to be reported.
    pub(crate) stats_events_reporter: ClientStatsSender,

    /// The task manager that controls all the spawned tasks that the clients uses to do it's job.
    pub(crate) shutdown_handle: ShutdownTracker,
    pub(crate) packet_type: Option<PacketType>,

    /// Internal state used for the `Stream` implementation
    _buffered: Vec<ReconstructedMessage>,

    pub(crate) forget_me: ForgetMe,
    pub(crate) remember_me: RememberMe,

    /// Set to `true` when the stream router is active, preventing
    /// message-based functions from being used concurrently.
    pub(crate) stream_mode: Arc<AtomicBool>,

    /// Opaque stream multiplexing state (lazily initialized by stream module).
    pub(crate) streams: Option<super::stream::StreamState>,

    /// How long a stream can be idle before the router cleans it up.
    pub(crate) stream_idle_timeout: Duration,
}

impl MixnetClient {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        nym_address: Recipient,
        identity_keys: Arc<ed25519::KeyPair>,
        client_input: ClientInput,
        client_output: ClientOutput,
        client_state: ClientState,
        reconstructed_receiver: ReconstructedMessagesReceiver,
        stats_events_reporter: ClientStatsSender,
        task_handle: ShutdownTracker,
        packet_type: Option<PacketType>,
        forget_me: ForgetMe,
        remember_me: RememberMe,
    ) -> Self {
        Self {
            nym_address,
            identity_keys,
            client_input,
            client_output,
            client_state,
            reconstructed_receiver: Some(reconstructed_receiver),
            stats_events_reporter,
            shutdown_handle: task_handle,
            packet_type,
            _buffered: Vec::new(),
            forget_me,
            remember_me,
            stream_mode: Arc::new(AtomicBool::new(false)),
            streams: None,
            stream_idle_timeout: super::stream::DEFAULT_STREAM_IDLE_TIMEOUT,
        }
    }

    /// Create a new client and connect to the mixnet using ephemeral in-memory keys that are
    /// discarded at application close.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nym_sdk::mixnet;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut client = mixnet::MixnetClient::connect_new().await.unwrap();
    /// }
    ///
    /// ```
    pub async fn connect_new() -> Result<Self> {
        MixnetClientBuilder::new_ephemeral()
            .build()?
            .connect_to_mixnet()
            .await
    }

    /// Get the nym address for this client, if it is available. The nym address is composed of the
    /// client identity, the client encryption key, and the gateway identity.
    pub fn nym_address(&self) -> &Recipient {
        &self.nym_address
    }

    /// Get a child token of the root, to monitor unexpected shutdown, without causing one
    pub fn cancellation_token(&self) -> CancellationToken {
        self.shutdown_handle.child_shutdown_token().inner().clone()
    }

    pub fn client_request_sender(&self) -> ClientRequestSender {
        self.client_input.client_request_sender.clone()
    }

    /// Get the client's identity keys.
    pub fn identity_keypair(&self) -> Arc<ed25519::KeyPair> {
        self.identity_keys.clone()
    }

    /// Sign a message with the client's private identity key.
    pub fn sign(&self, data: &[u8]) -> ed25519::Signature {
        self.identity_keys.private_key().sign(data)
    }

    /// Sign a message with the client's private identity key and return it as a base58 encoded
    /// signature.
    pub fn sign_text(&self, text: &str) -> String {
        self.identity_keys.private_key().sign_text(text)
    }

    /// Get gateway connection information, like the file descriptor of the WebSocket
    pub fn gateway_connection(&self) -> GatewayConnection {
        self.client_state.gateway_connection
    }

    /// Get a shallow clone of [`MixnetClientSender`]. Useful if you want split the send and
    /// receive logic in different locations.
    pub fn split_sender(&self) -> MixnetClientSender {
        MixnetClientSender {
            client_input: self.client_input.clone(),
            packet_type: self.packet_type,
            stream_mode: self.stream_mode.clone(),
        }
    }

    /// Get a shallow clone of [`ConnectionCommandSender`]. This is useful if you want to e.g
    /// explicitly close a transmission lane that is still sending data even though it should
    /// cancel.
    pub fn connection_command_sender(&self) -> ConnectionCommandSender {
        self.client_input.connection_command_sender.clone()
    }

    /// Get a shallow clone of [`LaneQueueLengths`]. This is useful to manually implement some form
    /// of backpressure logic.
    pub fn shared_lane_queue_lengths(&self) -> LaneQueueLengths {
        self.client_state.shared_lane_queue_lengths.clone()
    }

    /// Change the network topology used by this client for constructing sphinx packets into the
    /// provided one.
    pub async fn manually_overwrite_topology(&self, new_topology: NymTopology) {
        self.client_state
            .topology_accessor
            .manually_change_topology(new_topology)
            .await
    }

    /// Gets the value of the currently used network topology.
    pub async fn read_current_route_provider(
        &self,
    ) -> Option<RwLockReadGuard<'_, NymRouteProvider>> {
        self.client_state
            .topology_accessor
            .current_route_provider()
            .await
    }

    /// Restore default topology refreshing behaviour of this client.
    pub fn restore_automatic_topology_refreshing(&self) {
        self.client_state.topology_accessor.release_manual_control()
    }

    /// Wait for messages from the mixnet.
    ///
    /// # Cancel safety
    ///
    /// This method is cancel safe. If cancelled before a batch is available,
    /// no messages are lost — they remain in the channel for the next call.
    pub async fn wait_for_messages(&mut self) -> Option<Vec<ReconstructedMessage>> {
        if self.stream_mode.load(Ordering::SeqCst) {
            tracing::warn!("wait_for_messages() called after stream mode activated");
            return None;
        }
        self.reconstructed_receiver.as_mut()?.next().await
    }

    /// Provide a callback to execute on incoming messages from the mixnet.
    pub async fn on_messages<F>(&mut self, fun: F)
    where
        F: Fn(ReconstructedMessage),
    {
        while let Some(msgs) = self.wait_for_messages().await {
            for msg in msgs {
                fun(msg)
            }
        }
    }

    pub fn send_stats_event(&self, event: ClientStatsEvents) {
        self.stats_events_reporter.report(event);
    }

    /// Get a clone of stats_events_reporter for easier use
    pub fn stats_events_reporter(&self) -> ClientStatsSender {
        self.stats_events_reporter.clone()
    }

    /// Disconnect from the mixnet. Currently, it is not supported to reconnect a disconnected
    /// client.
    ///
    /// # Cancel safety
    ///
    /// This method is **not** cancel safe. If cancelled mid-shutdown,
    /// background tasks may be left running and state files may not be
    /// flushed. Always let this future run to completion.
    pub async fn disconnect(self) {
        if self.forget_me.any() {
            log::debug!("Sending forget me request: {:?}", self.forget_me);
            match self.send_forget_me().await {
                Ok(_) => (),
                Err(e) => error!("Failed to send forget me request: {e}"),
            };
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        } else if self.remember_me.stats() {
            log::debug!("Sending remember me request: {:?}", self.remember_me);
            match self.send_remember_me().await {
                Ok(_) => (),
                Err(e) => error!("Failed to send remember me request: {e}"),
            };
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }

        self.shutdown_handle.shutdown().await;
    }

    pub async fn send_forget_me(&self) -> Result<()> {
        let client_request = ClientRequest::ForgetMe {
            client: self.forget_me.client(),
            stats: self.forget_me.stats(),
        };
        match self
            .client_input
            .client_request_sender
            .send(client_request)
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("Failed to send forget me request: {e}");
                Err(Error::MessageSendingFailure)
            }
        }
    }

    pub async fn send_remember_me(&self) -> Result<()> {
        let client_request = ClientRequest::RememberMe {
            session_type: self.remember_me.session_type(),
        };
        match self
            .client_input
            .client_request_sender
            .send(client_request)
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("Failed to send remember me request: {e}");
                Err(Error::MessageSendingFailure)
            }
        }
    }

    /// Open a stream to a remote peer.
    ///
    /// Returns a [`MixnetStream`] implementing `AsyncRead + AsyncWrite`.
    /// `reply_surbs` controls how many reply SURBs are included with each
    /// outbound message so the peer can reply. Defaults to 10 if `None`.
    ///
    /// This is a one-way transition: once stream mode is active,
    /// message-mode methods like [`send_plain_message`](MixnetMessageSender::send_plain_message)
    /// return [`Error::StreamModeActive`](crate::Error::StreamModeActive).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nym_sdk::mixnet;
    /// use tokio::io::AsyncWriteExt;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut sender = mixnet::MixnetClient::connect_new().await.unwrap();
    /// let mut receiver = mixnet::MixnetClient::connect_new().await.unwrap();
    /// let recv_addr = *receiver.nym_address();
    ///
    /// let mut stream = sender.open_stream(recv_addr, None).await.unwrap();
    /// stream.write_all(b"hello").await.unwrap();
    /// stream.flush().await.unwrap();
    /// # }
    /// ```
    ///
    /// # Cancel safety
    ///
    /// This method is **not** cancel safe. Cancelling after the `Open`
    /// message is sent but before the `MixnetStream` is returned will
    /// leave the stream registered in the routing table with no owner.
    pub async fn open_stream(
        &mut self,
        recipient: Recipient,
        reply_surbs: Option<u32>,
    ) -> Result<MixnetStream> {
        super::stream::open_stream(
            self,
            recipient,
            reply_surbs.unwrap_or(DEFAULT_NUMBER_OF_SURBS),
        )
        .await
    }

    /// Create a listener that accepts inbound streams from remote peers.
    ///
    /// Can only be called once per client. Returns
    /// [`Error::ListenerAlreadyTaken`](crate::Error::ListenerAlreadyTaken) on subsequent calls.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nym_sdk::mixnet;
    /// use tokio::io::AsyncReadExt;
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut client = mixnet::MixnetClient::connect_new().await.unwrap();
    /// let mut listener = client.listener().unwrap();
    ///
    /// // Blocks until a remote peer opens a stream
    /// if let Some(mut stream) = listener.accept().await {
    ///     let mut buf = vec![0u8; 1024];
    ///     let n = stream.read(&mut buf).await.unwrap();
    ///     println!("received: {}", String::from_utf8_lossy(&buf[..n]));
    /// }
    /// # }
    /// ```
    pub fn listener(&mut self) -> Result<MixnetListener> {
        super::stream::listener(self)
    }
}

/// A clonable handle for sending messages through a connected [`MixnetClient`].
///
/// Obtained via [`MixnetClient::split_sender`]. Implements [`MixnetMessageSender`]
/// so it can send messages independently while another task handles receiving.
pub struct MixnetClientSender {
    client_input: ClientInput,
    packet_type: Option<PacketType>,
    stream_mode: Arc<AtomicBool>,
}

impl Clone for MixnetClientSender {
    fn clone(&self) -> Self {
        Self {
            client_input: self.client_input.clone(),
            packet_type: self.packet_type,
            stream_mode: self.stream_mode.clone(),
        }
    }
}

impl Stream for MixnetClient {
    type Item = ReconstructedMessage;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.stream_mode.load(Ordering::SeqCst) {
            tracing::warn!("Stream::poll_next() called after stream mode activated");
            return Poll::Ready(None);
        }
        if let Some(next) = self._buffered.pop() {
            cx.waker().wake_by_ref();
            return Poll::Ready(Some(next));
        }
        let receiver = match self.reconstructed_receiver.as_mut() {
            Some(rx) => rx,
            None => return Poll::Ready(None),
        };
        match ready!(Pin::new(receiver).poll_next(cx)) {
            None => Poll::Ready(None),
            Some(mut msgs) => {
                // the vector itself should never be empty
                if let Some(next) = msgs.pop() {
                    // there's more than a single message - buffer them and wake the waker
                    // to get polled again immediately
                    if !msgs.is_empty() {
                        self._buffered = msgs;
                        cx.waker().wake_by_ref();
                    }
                    Poll::Ready(Some(next))
                } else {
                    // I *think* this happens for SURBs, but I'm not 100% sure. Nonetheless it's
                    // beneign, but let's log it here anyway as a reminder
                    debug!("the reconstructed messages vector is empty");
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            }
        }
    }
}

#[async_trait]
impl MixnetMessageSender for MixnetClient {
    fn packet_type(&self) -> Option<PacketType> {
        self.packet_type
    }

    async fn send(&self, message: InputMessage) -> Result<()> {
        if self.stream_mode.load(Ordering::SeqCst) {
            tracing::warn!("send() called after stream mode activated");
            return Err(Error::StreamModeActive);
        }
        self.client_input
            .send(message)
            .await
            .map_err(|_| Error::MessageSendingFailure)
    }
}

#[async_trait]
impl MixnetMessageSender for MixnetClientSender {
    fn packet_type(&self) -> Option<PacketType> {
        self.packet_type
    }

    async fn send(&self, message: InputMessage) -> Result<()> {
        if self.stream_mode.load(Ordering::SeqCst) {
            tracing::warn!("send() called after stream mode activated");
            return Err(Error::StreamModeActive);
        }
        self.client_input
            .send(message)
            .await
            .map_err(|_| Error::MessageSendingFailure)
    }
}
