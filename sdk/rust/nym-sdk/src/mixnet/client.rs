use std::path::Path;

use futures::StreamExt;
use nymsphinx::{
    addressing::clients::{ClientIdentity, Recipient},
    receiver::ReconstructedMessage,
};
use tap::TapOptional;

use client_connections::TransmissionLane;
use client_core::{
    client::{
        base_client::{non_wasm_helpers, BaseClientBuilder, CredentialsToggle},
        inbound_messages::InputMessage,
        key_manager::KeyManager,
    },
    config::{persistence::key_pathfinder::ClientKeyPathfinder, GatewayEndpointConfig},
};

use crate::error::{Error, Result};

use super::{connection_state::ConnectionState, Config, GatewayKeyMode, KeyPaths, Keys};

pub struct Client {
    /// Keys handled by the client
    key_manager: KeyManager,

    /// Client configuration
    config: Config,

    /// Paths for client keys, including identity, encryption, ack and shared gateway keys.
    key_paths: Option<KeyPaths>,

    /// The client can be in one of multiple states, depending on how it is created and if it's
    /// connected to the mixnet.
    connection_state: ConnectionState,
}

impl Client {
    /// Create a new mixnet client. If no config options are supplied, creates a new client with
    /// ephemeral keys stored in RAM, which will be discarded at application close.
    ///
    /// Callers have the option of supplying futher parameters to store persistent identities at a
    /// location on-disk, if desired.
    pub fn new(config_option: Option<Config>, key_paths: Option<KeyPaths>) -> Result<Client> {
        let config = config_option.unwrap_or_default();

        // If we are provided paths to keys, use them if they are available. And if they are
        // not, write the generated keys back to storage.
        let key_manager = if let Some(ref key_paths) = key_paths {
            let path_finder = ClientKeyPathfinder::from(key_paths.clone());

            // Try load keys
            match KeyManager::load_keys_maybe_gateway(&path_finder) {
                Ok(key_manager) => key_manager,
                Err(err) => {
                    log::debug!("Not loading keys: {err}");
                    if path_finder.any_file_exists() && key_paths.operating_mode.is_keep() {
                        return Err(Error::DontOverwrite);
                    }

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

        Ok(Client {
            key_manager,
            config,
            key_paths,
            connection_state: ConnectionState::New,
        })
    }

    /// Get the client identity, which is the public key of the identity key pair.
    pub fn identity(&self) -> ClientIdentity {
        *self.key_manager.identity_keypair().public_key()
    }

    /// Get the nym address for this client, if it is available. The nym address is composed of the
    /// client identity, the client encryption key, and the gateway identity.
    pub fn nym_address(&self) -> Option<&Recipient> {
        self.connection_state.nym_address()
    }

    /// Client keys are generated at client creation if none were found. The gateway shared
    /// key, however, is created during the gateway registration handshake so it might not
    /// necessarily be available.
    pub fn has_gateway_key(&self) -> bool {
        self.key_manager.gateway_key_set()
    }

    pub fn set_keys(&mut self, _keys: &Keys) {
        todo!();
    }

    pub fn get_keys(&self) -> &Keys {
        todo!();
    }

    pub fn set_gateway_endpoint(&mut self, _gateway_endpoint_config: &GatewayEndpointConfig) {
        todo!();
    }

    pub fn get_gateway_endpoint(&self) -> Option<&GatewayEndpointConfig> {
        self.connection_state.gateway_endpoint_config()
    }

    pub async fn register_with_gateway(&mut self) -> Result<()> {
        assert!(
            matches!(self.connection_state, ConnectionState::New),
            "can only setup gateway when in `New` connection state"
        );

        let gateway_config = client_core::init::register_with_gateway(
            &mut self.key_manager,
            self.config.nym_api_endpoints.clone(),
            self.config.user_chosen_gateway.clone(),
        )
        .await?;

        let nym_address = client_core::init::get_client_address(&self.key_manager, &gateway_config);

        self.connection_state = ConnectionState::Registered {
            gateway_endpoint_config: gateway_config,
            nym_address,
        };
        Ok(())
    }

    fn write_gateway_key(&self, key_paths: KeyPaths, key_mode: &GatewayKeyMode) -> Result<()> {
        let path_finder = ClientKeyPathfinder::from(key_paths);
        if path_finder.gateway_key_file_exists() && key_mode.is_keep() {
            return Err(Error::DontOverwriteGatewayKey);
        };
        self.key_manager.store_key_gateway_only(&path_finder)?;
        Ok(())
    }

    fn write_gateway_endpoint_config(gateway_endpoint_config_path: &Path) -> Result<()> {
        let gateway_endpoint_config = toml::to_string(gateway_endpoint_config_path)?;

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

        let nym_address =
            client_core::init::get_client_address(&self.key_manager, &gateway_endpoint_config);

        self.connection_state = ConnectionState::Registered {
            gateway_endpoint_config,
            nym_address,
        };
        Ok(())
    }

    /// Connects to the mixnet via the gateway in the client config
    pub async fn connect_to_mixnet(&mut self) -> Result<()> {
        // For some simple cases we can figure how to setup gateway without it having to have been
        // called in advance.
        if matches!(self.connection_state, ConnectionState::New) {
            if let Some(key_paths) = &self.key_paths {
                let key_paths = key_paths.clone();
                if self.has_gateway_key() {
                    // If we have a gateway key from client, then we can just read the corresponding
                    // config
                    println!("Has gateway key: loading");
                    self.read_gateway_endpoint_config(&key_paths.gateway_endpoint_config)?;
                } else {
                    // If we didn't find any shared gateway key during creation, that means we first
                    // need to register a gateway
                    println!("NO gateway key: registering new");
                    self.register_with_gateway().await?;
                    self.write_gateway_key(key_paths.clone(), &GatewayKeyMode::Overwrite)?;
                    Self::write_gateway_endpoint_config(&key_paths.gateway_endpoint_config)?;
                }
            } else {
                // If we don't have any key paths, just use ephemeral keys
                self.register_with_gateway().await?;
            }
        }

        // At this point we should be in a registered state, either at function entry or by the
        // above convenience logic.
        assert!(matches!(
            self.connection_state,
            ConnectionState::Registered { .. }
        ));

        let gateway_config = self.connection_state.gateway_endpoint_config().unwrap();

        // TODO: we currently don't support having a bandwidth controller
        let bandwidth_controller = None;

        // TODO: currently we only support in-memory reply surb storage.
        let reply_storage_backend =
            non_wasm_helpers::setup_empty_reply_surb_backend(&self.config.debug_config);

        let base_builder = BaseClientBuilder::new(
            gateway_config,
            &self.config.debug_config,
            self.key_manager.clone(),
            bandwidth_controller,
            reply_storage_backend,
            CredentialsToggle::Disabled,
            self.config.nym_api_endpoints.clone(),
        );

        let nym_address = base_builder.as_mix_recipient();

        let mut started_client = base_builder.start_base().await.unwrap();
        let client_input = started_client.client_input.register_producer();
        let mut client_output = started_client.client_output.register_consumer();
        let client_state = started_client.client_state;

        // Register our receiver
        let reconstructed_receiver = client_output.register_receiver().unwrap();

        self.connection_state = ConnectionState::Connected {
            nym_address,
            client_input,
            client_output,
            client_state,
            reconstructed_receiver,
            task_manager: started_client.task_manager,
        };
        Ok(())
    }

    /// Sends stringy data to the supplied Nym address
    pub async fn send_str(&self, address: &str, message: &str) {
        log::debug!("send_str");
        let message_bytes = message.to_string().into_bytes();
        self.send_bytes(address, message_bytes).await;
    }

    /// Sends bytes to the supplied Nym address
    pub async fn send_bytes(&self, address: &str, message: Vec<u8>) {
        log::debug!("send_bytes");
        let Some(client_input) = self.connection_state.client_input() else {
            log::error!("Error: trying to send without being connected");
            return;
        };

        let lane = TransmissionLane::General;
        let recipient = Recipient::try_from_base58_string(address).unwrap();
        let input_msg = InputMessage::new_regular(recipient, message, lane);
        client_input.input_sender.send(input_msg).await.unwrap();
    }

    /// Wait for messages from the mixnet
    pub async fn wait_for_messages(&mut self) -> Option<Vec<ReconstructedMessage>> {
        let receiver = self
            .connection_state
            .reconstructed_receiver()
            .tap_none(|| log::error!("Error: trying to wait without being connected"))?;

        receiver.next().await
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
        let Some(task_manager) = self.connection_state.task_manager() else {
            log::error!("Trying to disconnect when not connected!");
            return;
        };

        task_manager.signal_shutdown().ok();
        task_manager.wait_for_shutdown().await;
        self.connection_state = ConnectionState::Disconnected;
    }
}
