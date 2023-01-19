use std::{path::Path, sync::Arc};

use client_connections::TransmissionLane;
use client_core::{
    client::{
        base_client::{
            non_wasm_helpers, BaseClientBuilder, ClientInput, ClientOutput, ClientState,
            CredentialsToggle,
        },
        inbound_messages::InputMessage,
        key_manager::KeyManager,
        received_buffer::ReconstructedMessagesReceiver,
        replies::reply_storage::{self, ReplyStorageBackend},
    },
    config::{persistence::key_pathfinder::ClientKeyPathfinder, GatewayEndpointConfig},
};
use crypto::asymmetric::identity;
use nymsphinx::{
    addressing::clients::{ClientIdentity, Recipient},
    receiver::ReconstructedMessage,
};
use task::TaskManager;

use futures::StreamExt;
use validator_client::nyxd::SigningNyxdClient;

use super::{connection_state::BuilderState, Config, GatewayKeyMode, Keys, KeysArc, StoragePaths};
use crate::error::{Error, Result};

pub struct MixnetClientBuilder<B: ReplyStorageBackend> {
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

impl MixnetClientBuilder<reply_storage::fs_backend::Backend> {
    /// Create a new mixnet client. If no config options are supplied, creates a new client with
    /// ephemeral keys stored in RAM, which will be discarded at application close.
    ///
    /// Callers have the option of supplying futher parameters to store persistent identities at a
    /// location on-disk, if desired.
    pub async fn new(config: Option<Config>, paths: Option<StoragePaths>) -> Result<Self> {
        let config = config.unwrap_or_default();

        let reply_surb_database_path = paths.as_ref().map(|p| p.reply_surb_database_path.clone());

        let reply_storage_backend = non_wasm_helpers::setup_fs_reply_surb_backend(
            reply_surb_database_path,
            &config.debug_config,
        )
        .await?;

        MixnetClientBuilder::new_with_custom_storage(Some(config), paths, reply_storage_backend)
    }
}

impl MixnetClientBuilder<reply_storage::Empty> {
    /// Create a new mixnet client. If no config options are supplied, creates a new client with
    /// ephemeral keys stored in RAM, which will be discarded at application close.
    ///
    /// Callers have the option of supplying futher parameters to store persistent identities at a
    /// location on-disk, if desired.
    pub fn new_without_storage(
        config: Option<Config>,
        paths: Option<StoragePaths>,
    ) -> Result<Self> {
        let config = config.unwrap_or_default();

        let reply_storage_backend =
            non_wasm_helpers::setup_empty_reply_surb_backend(&config.debug_config);

        MixnetClientBuilder::new_with_custom_storage(Some(config), paths, reply_storage_backend)
    }
}

impl<B> MixnetClientBuilder<B>
where
    B: ReplyStorageBackend + Sync + Send + 'static,
{
    /// Create a new mixnet client. If no config options are supplied, creates a new client with
    /// ephemeral keys stored in RAM, which will be discarded at application close.
    ///
    /// Callers have the option of supplying futher parameters to store persistent identities at a
    /// location on-disk, if desired.
    ///
    /// A custom storage backend can be passed in.
    pub fn new_with_custom_storage(
        config_option: Option<Config>,
        paths: Option<StoragePaths>,
        reply_storage_backend: B,
    ) -> Result<Self> {
        let config = config_option.unwrap_or_default();

        // If we are provided paths to keys, use them if they are available. And if they are
        // not, write the generated keys back to storage.
        let key_manager = if let Some(ref paths) = paths {
            let path_finder = ClientKeyPathfinder::from(paths.clone());

            // Try load keys
            match KeyManager::load_keys_but_gateway_is_optional(&path_finder) {
                Ok(key_manager) => key_manager,
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
            client_core::init::new_client_keys()
        };

        Ok(Self {
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

    pub fn set_keys(&mut self, keys: Keys) {
        self.key_manager.set_identity_keypair(keys.identity_keypair);
        self.key_manager
            .set_encryption_keypair(keys.encryption_keypair);
        self.key_manager.set_ack_key(keys.ack_key);

        self.key_manager
            .insert_gateway_shared_key(Arc::new(keys.gateway_shared_key));
    }

    pub fn get_keys(&self) -> KeysArc {
        KeysArc::from(&self.key_manager)
    }

    pub fn set_gateway_endpoint(&mut self, gateway_endpoint_config: GatewayEndpointConfig) {
        self.state = BuilderState::Registered {
            gateway_endpoint_config,
        }
    }

    pub fn get_gateway_endpoint(&self) -> Option<&GatewayEndpointConfig> {
        self.state.gateway_endpoint_config()
    }

    pub async fn register_with_gateway(&mut self) -> Result<()> {
        assert!(
            matches!(self.state, BuilderState::New),
            "can only setup gateway when in `New` connection state"
        );

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

    /// Connects to the mixnet via the gateway in the client config
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
                    self.register_with_gateway().await?;
                    self.write_gateway_key(paths.clone(), &GatewayKeyMode::Overwrite)?;
                    self.write_gateway_endpoint_config(&paths.gateway_endpoint_config)?;
                }
            } else {
                // If we don't have any key paths, just use ephemeral keys
                self.register_with_gateway().await?;
            }
        }

        // At this point we should be in a registered state, either at function entry or by the
        // above convenience logic.
        let BuilderState::Registered { gateway_endpoint_config } = self.state else {
            todo!();
        };

        let nym_address =
            client_core::init::get_client_address(&self.key_manager, &gateway_endpoint_config);

        // TODO: we currently don't support having a bandwidth controller
        let bandwidth_controller = None;

        let base_builder: BaseClientBuilder<_, SigningNyxdClient> = BaseClientBuilder::new(
            &gateway_endpoint_config,
            &self.config.debug_config,
            self.key_manager.clone(),
            bandwidth_controller,
            self.reply_storage_backend,
            CredentialsToggle::Disabled,
            self.config.nym_api_endpoints.clone(),
        );

        let mut started_client = base_builder.start_base().await.unwrap();
        let client_input = started_client.client_input.register_producer();
        let mut client_output = started_client.client_output.register_consumer();
        let client_state = started_client.client_state;

        // Register our receiver
        let reconstructed_receiver = client_output.register_receiver().unwrap();

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

pub struct MixnetClient {
    nym_address: Recipient,

    /// Keys handled by the client
    key_manager: KeyManager,

    client_input: ClientInput,

    #[allow(dead_code)]
    client_output: ClientOutput,

    #[allow(dead_code)]
    client_state: ClientState,

    reconstructed_receiver: ReconstructedMessagesReceiver,

    task_manager: TaskManager,
}

impl MixnetClient {
    pub async fn connect() -> Result<Self> {
        let client = MixnetClientBuilder::new_without_storage(None, None)?;
        client.connect_to_mixnet().await
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

    /// Sends stringy data to the supplied Nym address
    pub async fn send_str(&self, address: &str, message: &str) {
        let message_bytes = message.to_string().into_bytes();
        self.send_bytes(address, message_bytes).await;
    }

    /// Sends stringy data to the supplied Nym address, and skip sending reply-SURBs
    pub async fn send_str_direct(&self, address: &str, message: &str) {
        let message_bytes = message.to_string().into_bytes();
        self.send_bytes(address, message_bytes).await;
    }

    /// Sends bytes to the supplied Nym address
    pub async fn send_bytes(&self, address: &str, message: Vec<u8>) {
        let lane = TransmissionLane::General;
        let recipient = Recipient::try_from_base58_string(address).unwrap();
        let input_msg = InputMessage::new_anonymous(recipient, message, 20, lane);
        self.client_input
            .input_sender
            .send(input_msg)
            .await
            .unwrap();
    }

    /// Sends bytes to the supplied Nym address, and skip sending reply-SURBs
    pub async fn send_bytes_direct(&self, address: &str, message: Vec<u8>) {
        let lane = TransmissionLane::General;
        let recipient = Recipient::try_from_base58_string(address).unwrap();
        let input_msg = InputMessage::new_regular(recipient, message, lane);
        self.client_input
            .input_sender
            .send(input_msg)
            .await
            .unwrap();
    }


    /// Wait for messages from the mixnet
    pub async fn wait_for_messages(&mut self) -> Option<Vec<ReconstructedMessage>> {
        self.reconstructed_receiver.next().await
    }

    pub fn wait_for_messages_split(&mut self) -> Option<ReconstructedMessage> {
        todo!();
    }

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
