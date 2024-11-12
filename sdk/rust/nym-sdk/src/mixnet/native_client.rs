use crate::mixnet::client::MixnetClientBuilder;
use crate::mixnet::traits::MixnetMessageSender;
use crate::{Error, Result};
use async_trait::async_trait;
use futures::{ready, Stream, StreamExt};
use log::error;
use nym_client_core::client::base_client::GatewayConnection;
use nym_client_core::client::{
    base_client::{ClientInput, ClientOutput, ClientState},
    inbound_messages::InputMessage,
    received_buffer::ReconstructedMessagesReceiver,
};
use nym_crypto::asymmetric::identity;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::{params::PacketType, receiver::ReconstructedMessage};
use nym_statistics_common::clients::{ClientStatsEvents, ClientStatsSender};
use nym_task::{
    connections::{ConnectionCommandSender, LaneQueueLengths},
    TaskHandle,
};
use nym_topology::NymTopology;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

/// Client connected to the Nym mixnet.
pub struct MixnetClient {
    /// The nym address of this connected client.
    pub(crate) nym_address: Recipient,

    pub(crate) identity_keys: Arc<identity::KeyPair>,

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
    pub(crate) reconstructed_receiver: ReconstructedMessagesReceiver,

    /// A channel for sending stats event to be reported.
    pub(crate) stats_events_reporter: ClientStatsSender,

    /// The task manager that controls all the spawned tasks that the clients uses to do it's job.
    pub(crate) task_handle: TaskHandle,
    pub(crate) packet_type: Option<PacketType>,

    // internal state used for the `Stream` implementation
    _buffered: Vec<ReconstructedMessage>,
}

impl MixnetClient {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        nym_address: Recipient,
        identity_keys: Arc<identity::KeyPair>,
        client_input: ClientInput,
        client_output: ClientOutput,
        client_state: ClientState,
        reconstructed_receiver: ReconstructedMessagesReceiver,
        stats_events_reporter: ClientStatsSender,
        task_handle: TaskHandle,
        packet_type: Option<PacketType>,
    ) -> Self {
        Self {
            nym_address,
            identity_keys,
            client_input,
            client_output,
            client_state,
            reconstructed_receiver,
            stats_events_reporter,
            task_handle,
            packet_type,
            _buffered: Vec::new(),
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
    ///     let mut client = mixnet::MixnetClient::connect_new().await;
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

    /// Sign a message with the client's private identity key.
    pub fn sign(&self, data: &[u8]) -> identity::Signature {
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
    pub async fn read_current_topology(&self) -> Option<NymTopology> {
        self.client_state.topology_accessor.current_topology().await
    }

    /// Restore default topology refreshing behaviour of this client.
    pub fn restore_automatic_topology_refreshing(&self) {
        self.client_state.topology_accessor.release_manual_control()
    }

    /// Wait for messages from the mixnet
    pub async fn wait_for_messages(&mut self) -> Option<Vec<ReconstructedMessage>> {
        self.reconstructed_receiver.next().await
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

    /// Disconnect from the mixnet. Currently it is not supported to reconnect a disconnected
    /// client.
    pub async fn disconnect(mut self) {
        if let TaskHandle::Internal(task_manager) = &mut self.task_handle {
            task_manager.signal_shutdown().ok();
            task_manager.wait_for_shutdown().await;
        }

        // note: it's important to take ownership of the struct as if the shutdown is `TaskHandle::External`,
        // it must be dropped to finalize the shutdown
    }
}

#[derive(Clone)]
pub struct MixnetClientSender {
    client_input: ClientInput,
    packet_type: Option<PacketType>,
}

impl Stream for MixnetClient {
    type Item = ReconstructedMessage;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Some(next) = self._buffered.pop() {
            cx.waker().wake_by_ref();
            return Poll::Ready(Some(next));
        }
        match ready!(Pin::new(&mut self.reconstructed_receiver).poll_next(cx)) {
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
                    error!("the reconstructed messages vector is empty - please let the developers know if you see this message");
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
        self.client_input
            .send(message)
            .await
            .map_err(|_| Error::MessageSendingFailure)
    }
}
