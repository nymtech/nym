use nym_client_core::client::{
    base_client::{ClientInput, ClientOutput, ClientState},
    inbound_messages::InputMessage,
    key_manager::KeyManager,
    received_buffer::ReconstructedMessagesReceiver,
};
use nym_sphinx::{
    addressing::clients::{ClientIdentity, Recipient},
    params::PacketType,
    receiver::ReconstructedMessage,
};
use nym_task::{
    connections::{ConnectionCommandSender, LaneQueueLengths, TransmissionLane},
    TaskManager,
};

use futures::StreamExt;
use nym_topology::NymTopology;

use crate::mixnet::client::{IncludedSurbs, MixnetClientBuilder};
use crate::Result;

/// Client connected to the Nym mixnet.
pub struct MixnetClient {
    /// The nym address of this connected client.
    pub(crate) nym_address: Recipient,

    /// Keys handled by the client
    pub(crate) key_manager: KeyManager,

    /// Input to the client from the users perspective. This can be either data to send or controll
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

    /// The task manager that controlls all the spawned tasks that the clients uses to do it's job.
    pub(crate) task_manager: TaskManager,
    pub(crate) packet_type: Option<PacketType>,
}

impl MixnetClient {
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
        MixnetClientBuilder::new()
            .build::<crate::mixnet::EmptyReplyStorage>()
            .await?
            .connect_to_mixnet()
            .await
    }

    /// Get the client identity, which is the public key of the identity key pair.
    pub fn identity(&self) -> ClientIdentity {
        *self.key_manager.identity_keypair().public_key()
    }

    /// Get the nym address for this client, if it is available. The nym address is composed of the
    /// client identity, the client encryption key, and the gateway identity.
    pub fn nym_address(&self) -> &Recipient {
        &self.nym_address
    }

    /// Get a shallow clone of [`MixnetClientSender`]. Useful if you want split the send and
    /// receive logic in different locations.
    pub fn sender(&self) -> MixnetClientSender {
        MixnetClientSender {
            client_input: self.client_input.clone(),
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

    /// Sends stringy data to the supplied Nym address
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nym_sdk::mixnet;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let address = "foobar";
    ///     let recipient = mixnet::Recipient::try_from_base58_string(address).unwrap();
    ///     let mut client = mixnet::MixnetClient::connect_new().await.unwrap();
    ///     client.send_str(recipient, "hi").await;
    /// }
    /// ```
    pub async fn send_str(&self, address: Recipient, message: &str) {
        let message_bytes = message.to_string().into_bytes();
        self.send_bytes(address, message_bytes, IncludedSurbs::default())
            .await;
    }

    /// Sends bytes to the supplied Nym address. There is the option to specify the number of
    /// reply-SURBs to include.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nym_sdk::mixnet;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let address = "foobar";
    ///     let recipient = mixnet::Recipient::try_from_base58_string(address).unwrap();
    ///     let mut client = mixnet::MixnetClient::connect_new().await.unwrap();
    ///     let surbs = mixnet::IncludedSurbs::default();
    ///     client.send_bytes(recipient, "hi".to_owned().into_bytes(), surbs).await;
    /// }
    /// ```
    pub async fn send_bytes(&self, address: Recipient, message: Vec<u8>, surbs: IncludedSurbs) {
        let lane = TransmissionLane::General;
        let input_msg = match surbs {
            IncludedSurbs::Amount(surbs) => {
                InputMessage::new_anonymous(address, message, surbs, lane, self.packet_type)
            }
            IncludedSurbs::ExposeSelfAddress => {
                InputMessage::new_regular(address, message, lane, self.packet_type)
            }
        };
        self.send(input_msg).await
    }

    /// Sends a [`InputMessage`] to the mixnet. This is the most low-level sending function, for
    /// full customization.
    async fn send(&self, message: InputMessage) {
        if self.client_input.send(message).await.is_err() {
            log::error!("Failed to send message");
        }
    }

    /// Sends a [`InputMessage`] to the mixnet. This is the most low-level sending function, for
    /// full customization.
    ///
    /// Waits until the message is actually sent, or close to being sent, until returning.
    ///
    /// NOTE: this not yet implemented.
    #[allow(unused)]
    async fn send_wait(&self, _message: InputMessage) {
        todo!();
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

    /// Disconnect from the mixnet. Currently it is not supported to reconnect a disconnected
    /// client.
    pub async fn disconnect(&mut self) {
        self.task_manager.signal_shutdown().ok();
        self.task_manager.wait_for_shutdown().await;
    }
}

pub struct MixnetClientSender {
    client_input: ClientInput,
}

impl MixnetClientSender {
    pub async fn send_input_message(&mut self, message: InputMessage) {
        if self.client_input.send(message).await.is_err() {
            log::error!("Failed to send message");
        }
    }
}
