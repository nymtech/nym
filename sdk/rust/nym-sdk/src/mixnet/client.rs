use std::{path::Path, sync::Arc};

use client_connections::TransmissionLane;
use client_core::{
    client::{
        base_client::{
            BaseClientBuilder, ClientInput, ClientOutput, ClientState, CredentialsToggle,
        },
        inbound_messages::InputMessage,
        key_manager::KeyManager,
        received_buffer::ReconstructedMessagesReceiver,
        replies::reply_storage::ReplyStorageBackend,
    },
    config::{persistence::key_pathfinder::ClientKeyPathfinder, GatewayEndpointConfig},
};
use nym_crypto::asymmetric::identity;
use nym_sphinx::{
    addressing::clients::{ClientIdentity, Recipient},
    receiver::ReconstructedMessage,
};
use nym_task::TaskManager;

use futures::StreamExt;
use validator_client::nyxd::SigningNyxdClient;

use crate::{Error, Result};

use super::{connection_state::BuilderState, Config, GatewayKeyMode, Keys, KeysArc, StoragePaths};

// The number of surbs to include in a message by default
const DEFAULT_NUMBER_OF_SURBS: u32 = 5;

#[derive(Default)]
pub struct MixnetClientBuilder {
    config: Option<Config>,
    storage_paths: Option<StoragePaths>,
    keys: Option<Keys>,
    gateway_config: Option<GatewayEndpointConfig>,
}

impl MixnetClientBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn config(mut self, config: Config) -> Self {
        self.config = Some(config);
        self
    }

    /// Enabled storage
    #[must_use]
    pub fn enable_storage(mut self, paths: StoragePaths) -> Self {
        self.storage_paths = Some(paths);
        self
    }

    #[must_use]
    pub fn keys(mut self, keys: Keys) -> Self {
        self.keys = Some(keys);
        self
    }

    #[must_use]
    pub fn gateway_config(mut self, gateway_config: GatewayEndpointConfig) -> Self {
        self.gateway_config = Some(gateway_config);
        self
    }

    /// Construct a [`DisconnectedMixnetClient`] from the setup specified.
    pub async fn build<B>(self) -> Result<DisconnectedMixnetClient<B>>
    where
        B: ReplyStorageBackend + Send + Sync + 'static,
        <B as ReplyStorageBackend>::StorageError: Send + Sync,
    {
        let config = self.config.unwrap_or_default();
        let storage_paths = self.storage_paths;

        let mut client = DisconnectedMixnetClient::new(Some(config), storage_paths).await?;

        if let Some(keys) = self.keys {
            client.set_keys(keys);
        }

        // If we have a gateway config, we can move the client into a registered state. This will
        // fail if no gateway key is set.
        if let Some(gateway_config) = self.gateway_config {
            client.register_gateway_with_config(gateway_config)?;
        }

        Ok(client)
    }
}

/// Represents a client that is not yet connected to the mixnet. You typically create one when you
/// want to have a separate configuration and connection phase. Once the mixnet client builder is
/// configured, call [`MixnetClientBuilder::connect_to_mixnet()`] to transition to a connected
/// client.
pub struct DisconnectedMixnetClient<B>
where
    B: ReplyStorageBackend + Sync + Send + 'static,
{
    /// Keys handled by the client
    key_manager: KeyManager,

    /// Client configuration
    config: Config,

    /// Paths for client keys, including identity, encryption, ack and shared gateway keys.
    storage_paths: Option<StoragePaths>,

    /// The client can be in one of multiple states, depending on how it is created and if it's
    /// connected to the mixnet.
    state: BuilderState,

    /// The storage backend for reply-SURBs
    reply_storage_backend: B,
}

impl<B> DisconnectedMixnetClient<B>
where
    B: ReplyStorageBackend + Sync + Send + 'static,
{
    /// Create a new mixnet client in a disconnected state. If no config options are supplied,
    /// creates a new client with ephemeral keys stored in RAM, which will be discarded at
    /// application close.
    ///
    /// Callers have the option of supplying futher parameters to store persistent identities at a
    /// location on-disk, if desired.
    async fn new(
        config: Option<Config>,
        paths: Option<StoragePaths>,
    ) -> Result<DisconnectedMixnetClient<B>>
    where
        <B as ReplyStorageBackend>::StorageError: Send + Sync,
    {
        let config = config.unwrap_or_default();
        let reply_surb_database_path = paths.as_ref().map(|p| p.reply_surb_database_path.clone());

        // The reply storage backend is generic, and can be set by the caller/instantiator
        let reply_storage_backend = B::new(&config.debug_config, reply_surb_database_path)
            .await
            .map_err(|err| Error::StorageError {
                source: Box::new(err),
            })?;

        // If we are provided paths to keys, use them if they are available. And if they are
        // not, write the generated keys back to storage.
        let key_manager = if let Some(ref paths) = paths {
            let path_finder = ClientKeyPathfinder::from(paths.clone());

            // Try load keys
            match KeyManager::load_keys_but_gateway_is_optional(&path_finder) {
                Ok(key_manager) => {
                    log::debug!("Keys loaded");
                    key_manager
                }
                Err(err) => {
                    log::debug!("Not loading keys: {err}");
                    if let Some(path) = path_finder.any_file_exists_and_return() {
                        if paths.operating_mode.is_keep() {
                            return Err(Error::DontOverwrite(path));
                        }
                    }

                    // Double check using a function that has slightly different internal logic. I
                    // know this is a bit defensive, but I don't want to overwrite
                    assert!(!(path_finder.any_file_exists() && paths.operating_mode.is_keep()));

                    // Create new keys and write to storage
                    let key_manager = client_core::init::new_client_keys();
                    // WARN: this will overwrite!
                    key_manager.store_keys(&path_finder)?;
                    key_manager
                }
            }
        } else {
            // Ephemeral keys that we only store in memory
            log::debug!("Creating new ephemeral keys");
            client_core::init::new_client_keys()
        };

        Ok(DisconnectedMixnetClient {
            key_manager,
            config,
            storage_paths: paths,
            state: BuilderState::New,
            reply_storage_backend,
        })
    }

    /// Client keys are generated at client creation if none were found. The gateway shared
    /// key, however, is created during the gateway registration handshake so it might not
    /// necessarily be available.
    fn has_gateway_key(&self) -> bool {
        self.key_manager.is_gateway_key_set()
    }

    /// Sets the keys of this [`MixnetClientBuilder<B>`].
    fn set_keys(&mut self, keys: Keys) {
        self.key_manager.set_identity_keypair(keys.identity_keypair);
        self.key_manager
            .set_encryption_keypair(keys.encryption_keypair);
        self.key_manager.set_ack_key(keys.ack_key);

        self.key_manager
            .insert_gateway_shared_key(Arc::new(keys.gateway_shared_key));
    }

    /// Returns the keys of this [`DisconnectedMixnetClient<B>`]. Client keys are always available
    /// since if none are specified at creation time, new random ones are generated.
    pub fn get_keys(&self) -> KeysArc {
        KeysArc::from(&self.key_manager)
    }

    /// Sets the gateway endpoint of this [`MixnetClientBuilder<B>`].
    ///
    /// NOTE: this will mark this builder as `Registered`, and the it is assumed that the keys are
    /// also explicitly set.
    pub fn register_gateway_with_config(
        &mut self,
        gateway_endpoint_config: GatewayEndpointConfig,
    ) -> Result<()> {
        if !self.has_gateway_key() {
            return Err(Error::NoGatewayKeySet);
        }

        self.state = BuilderState::Registered {
            gateway_endpoint_config,
        };

        Ok(())
    }

    /// Register with a gateway. If a gateway is provided in the config then that will try to be
    /// used. If none is specified, a gateway at random will be picked.
    ///
    /// # Errors
    ///
    /// This function will return an error if you try to re-register when in an already registered
    /// state.
    pub async fn register_and_authenticate_gateway(&mut self) -> Result<()> {
        if self.state != BuilderState::New {
            return Err(Error::ReregisteringGatewayNotSupported);
        }
        log::debug!("Registering with gateway");

        let user_chosen_gateway = self
            .config
            .user_chosen_gateway
            .as_ref()
            .map(identity::PublicKey::from_base58_string)
            .transpose()?;

        let gateway_config = client_core::init::register_with_gateway(
            &mut self.key_manager,
            self.config.nym_api_endpoints.clone(),
            user_chosen_gateway,
        )
        .await?;

        self.state = BuilderState::Registered {
            gateway_endpoint_config: gateway_config,
        };
        Ok(())
    }

    /// Returns the get gateway endpoint of this [`MixnetClientBuilder<B>`].
    pub fn get_gateway_endpoint(&self) -> Option<&GatewayEndpointConfig> {
        self.state.gateway_endpoint_config()
    }

    fn write_gateway_key(&self, paths: StoragePaths, key_mode: &GatewayKeyMode) -> Result<()> {
        let path_finder = ClientKeyPathfinder::from(paths);
        if path_finder.gateway_key_file_exists() && key_mode.is_keep() {
            return Err(Error::DontOverwriteGatewayKey(
                path_finder.gateway_shared_key().to_path_buf(),
            ));
        };
        self.key_manager.store_gateway_key(&path_finder)?;
        Ok(())
    }

    fn write_gateway_endpoint_config(&self, gateway_endpoint_config_path: &Path) -> Result<()> {
        let gateway_endpoint_config = toml::to_string(
            self.get_gateway_endpoint()
                .ok_or(Error::GatewayNotAvailableForWriting)?,
        )?;

        // Ensure the whole directory structure exists
        if let Some(parent_dir) = gateway_endpoint_config_path.parent() {
            std::fs::create_dir_all(parent_dir)?;
        }
        std::fs::write(gateway_endpoint_config_path, gateway_endpoint_config)?;
        Ok(())
    }

    fn read_gateway_endpoint_config(&mut self, gateway_endpoint_config_path: &Path) -> Result<()> {
        let gateway_endpoint_config: GatewayEndpointConfig =
            std::fs::read_to_string(gateway_endpoint_config_path)
                .map(|str| toml::from_str(&str))??;

        self.state = BuilderState::Registered {
            gateway_endpoint_config,
        };
        Ok(())
    }

    /// Connect the client to the mixnet.
    ///
    /// - If the client is already registered with a gateway, use that gateway.
    /// - If no gateway is registered, but there is an existing configuration and key, use that.
    /// - If no gateway is registered, and there is no pre-existing configuration or key, try to
    /// register a new gateway.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nym_sdk::mixnet;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = mixnet::MixnetClientBuilder::new()
    ///         .build::<mixnet::EmptyReplyStorage>()
    ///         .await
    ///         .unwrap();
    ///     let client = client.connect_to_mixnet().await.unwrap();
    /// }
    /// ```
    pub async fn connect_to_mixnet(mut self) -> Result<MixnetClient>
    where
        <B as ReplyStorageBackend>::StorageError: Sync + Send,
    {
        // For some simple cases we can figure how to setup gateway without it having to have been
        // called in advance.
        if matches!(self.state, BuilderState::New) {
            if let Some(paths) = &self.storage_paths {
                let paths = paths.clone();
                if self.has_gateway_key() {
                    // If we have a gateway key from client, then we can just read the corresponding
                    // config
                    log::trace!("Gateway key found: loading");
                    self.read_gateway_endpoint_config(&paths.gateway_endpoint_config)?;
                } else {
                    // If we didn't find any shared gateway key during creation, that means we first
                    // need to register a gateway
                    log::trace!("Gateway key NOT found: registering new");
                    self.register_and_authenticate_gateway().await?;
                    self.write_gateway_key(paths.clone(), &GatewayKeyMode::Overwrite)?;
                    self.write_gateway_endpoint_config(&paths.gateway_endpoint_config)?;
                }
            } else {
                // If we don't have any key paths, just use ephemeral keys
                self.register_and_authenticate_gateway().await?;
            }
        }

        // If the gateway is in a registered state, but without the gateway key set.
        if matches!(self.state, BuilderState::Registered { .. }) && !self.has_gateway_key() {
            return Err(Error::NoGatewayKeySet);
        }

        // At this point we should be in a registered state, either at function entry or by the
        // above convenience logic.
        let BuilderState::Registered { gateway_endpoint_config } = self.state else {
            return Err(Error::FailedToTransitionToRegisteredState);
        };

        let nym_address =
            client_core::init::get_client_address(&self.key_manager, &gateway_endpoint_config);

        // TODO: we currently don't support having a bandwidth controller
        let bandwidth_controller = None;

        let base_builder: BaseClientBuilder<'_, _, SigningNyxdClient> = BaseClientBuilder::new(
            &gateway_endpoint_config,
            &self.config.debug_config,
            self.key_manager.clone(),
            bandwidth_controller,
            self.reply_storage_backend,
            CredentialsToggle::Disabled,
            self.config.nym_api_endpoints.clone(),
        );

        let mut started_client = base_builder.start_base().await?;
        let client_input = started_client.client_input.register_producer();
        let mut client_output = started_client.client_output.register_consumer();
        let client_state = started_client.client_state;

        // Register our receiver
        let reconstructed_receiver = client_output.register_receiver()?;

        Ok(MixnetClient {
            nym_address,
            key_manager: self.key_manager,
            client_input,
            client_output,
            client_state,
            reconstructed_receiver,
            task_manager: started_client.task_manager,
        })
    }
}

pub enum IncludedSurbs {
    Amount(u32),
    ExposeSelfAddress,
}
impl Default for IncludedSurbs {
    fn default() -> Self {
        Self::Amount(DEFAULT_NUMBER_OF_SURBS)
    }
}

impl IncludedSurbs {
    pub fn new(reply_surbs: u32) -> Self {
        Self::Amount(reply_surbs)
    }

    pub fn none() -> Self {
        Self::Amount(0)
    }

    pub fn expose_self_address() -> Self {
        Self::ExposeSelfAddress
    }
}

/// Client connected to the Nym mixnet.
pub struct MixnetClient {
    /// The nym address of this connected client.
    nym_address: Recipient,

    /// Keys handled by the client
    key_manager: KeyManager,

    /// Input to the client from the users perspective. This can be either data to send or controll
    /// messages.
    client_input: ClientInput,

    /// Output from the client from the users perspective. This is typically messages arriving from
    /// the mixnet.
    #[allow(dead_code)]
    client_output: ClientOutput,

    /// The current state of the client that is exposed to the user. This includes things like
    /// current message send queue length.
    client_state: ClientState,

    /// A channel for messages arriving from the mixnet after they have been reconstructed.
    reconstructed_receiver: ReconstructedMessagesReceiver,

    /// The task manager that controlls all the spawned tasks that the clients uses to do it's job.
    task_manager: TaskManager,
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
    /// explictly close a transmission lane that is still sending data even though it should
    /// cancel.
    pub fn connection_command_sender(&self) -> client_connections::ConnectionCommandSender {
        self.client_input.connection_command_sender.clone()
    }

    /// Get a shallow clone of [`LaneQueueLengths`]. This is useful to manually implement some form
    /// of backpressure logic.
    pub fn shared_lane_queue_lengths(&self) -> client_connections::LaneQueueLengths {
        self.client_state.shared_lane_queue_lengths.clone()
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
                InputMessage::new_anonymous(address, message, surbs, lane)
            }
            IncludedSurbs::ExposeSelfAddress => InputMessage::new_regular(address, message, lane),
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
