use futures::channel::mpsc;
use futures::StreamExt;
use std::{path::Path, sync::Arc};
use url::Url;

use nym_bandwidth_controller::BandwidthController;
use nym_client_core::client::base_client::BaseClient;
use nym_client_core::config::DebugConfig;
use nym_client_core::{
    client::{
        base_client::{BaseClientBuilder, CredentialsToggle},
        key_manager::KeyManager,
        replies::reply_storage::ReplyStorageBackend,
    },
    config::{persistence::key_pathfinder::ClientKeyPathfinder, GatewayEndpointConfig},
};
use nym_credential_storage::ephemeral_storage::EphemeralStorage;
use nym_credential_storage::initialise_ephemeral_storage;
use nym_crypto::asymmetric::identity;
use nym_network_defaults::NymNetworkDetails;

use nym_socks5_client_core::config::Socks5;
use nym_task::manager::TaskStatus;
use nym_topology::provider_trait::TopologyProvider;
use nym_validator_client::nyxd::QueryNyxdClient;
use nym_validator_client::Client;

use crate::bandwidth::BandwidthAcquireClient;
use crate::mixnet::native_client::MixnetClient;
use crate::mixnet::socks5_client::Socks5MixnetClient;
use crate::mixnet::Recipient;
use crate::{Error, Result};

use super::{connection_state::BuilderState, Config, GatewayKeyMode, Keys, KeysArc, StoragePaths};

// The number of surbs to include in a message by default
const DEFAULT_NUMBER_OF_SURBS: u32 = 5;

#[derive(Default)]
pub struct MixnetClientBuilder {
    config: Config,
    storage_paths: Option<StoragePaths>,
    keys: Option<Keys>,
    gateway_config: Option<GatewayEndpointConfig>,
    socks5_config: Option<Socks5>,
    custom_topology_provider: Option<Box<dyn TopologyProvider>>,
}

impl MixnetClientBuilder {
    /// Create a client builder with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Request a specific gateway instead of a random one.
    #[must_use]
    pub fn request_gateway(mut self, user_chosen_gateway: String) -> Self {
        self.config.user_chosen_gateway = Some(user_chosen_gateway);
        self
    }

    /// Use a specific network instead of the default (mainnet) one.
    #[must_use]
    pub fn network_details(mut self, network_details: NymNetworkDetails) -> Self {
        self.config.network_details = network_details;
        self
    }

    /// Enable paid coconut bandwidth credentials mode.
    #[must_use]
    pub fn enable_credentials_mode(mut self) -> Self {
        self.config.enabled_credentials_mode = true;
        self
    }

    /// Use a custom debugging configuration.
    #[must_use]
    pub fn debug_config(mut self, debug_config: DebugConfig) -> Self {
        self.config.debug_config = debug_config;
        self
    }

    /// Enabled storage.
    #[must_use]
    pub fn enable_storage(mut self, paths: StoragePaths) -> Self {
        self.storage_paths = Some(paths);
        self
    }

    /// Use a previously generated set of client keys.
    #[must_use]
    pub fn keys(mut self, keys: Keys) -> Self {
        self.keys = Some(keys);
        self
    }

    /// Use a gateway that you previously registered with.
    #[must_use]
    pub fn registered_gateway(mut self, gateway_config: GatewayEndpointConfig) -> Self {
        self.gateway_config = Some(gateway_config);
        self
    }

    /// Configure the SOCKS5 mode.
    #[must_use]
    pub fn socks5_config(mut self, socks5_config: Socks5) -> Self {
        self.socks5_config = Some(socks5_config);
        self
    }

    /// Use a custom topology provider.
    #[must_use]
    pub fn custom_topology_provider(
        mut self,
        topology_provider: Box<dyn TopologyProvider>,
    ) -> Self {
        self.custom_topology_provider = Some(topology_provider);
        self
    }

    /// Construct a [`DisconnectedMixnetClient`] from the setup specified.
    pub async fn build<B>(self) -> Result<DisconnectedMixnetClient<B>>
    where
        B: ReplyStorageBackend + Send + Sync + 'static,
        <B as ReplyStorageBackend>::StorageError: Send + Sync,
    {
        let storage_paths = self.storage_paths;

        let mut client = DisconnectedMixnetClient::new(
            self.config,
            self.socks5_config,
            storage_paths,
            self.custom_topology_provider,
        )
        .await?;

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
/// configured, call [`MixnetClientBuilder::connect_to_mixnet()`] or
/// [`MixnetClientBuilder::connect_to_mixnet_via_socks5()`] to transition to a connected
/// client.
pub struct DisconnectedMixnetClient<B>
where
    B: ReplyStorageBackend + Sync + Send + 'static,
{
    /// Keys handled by the client
    key_manager: KeyManager,

    /// Client configuration
    config: Config,

    /// Socks5 configuration
    socks5_config: Option<Socks5>,

    /// Paths for client keys, including identity, encryption, ack and shared gateway keys.
    storage_paths: Option<StoragePaths>,

    /// The client can be in one of multiple states, depending on how it is created and if it's
    /// connected to the mixnet.
    state: BuilderState,

    /// Controller of bandwidth credentials that the mixnet client can use to connect
    bandwidth_controller: BandwidthController<Client<QueryNyxdClient>, EphemeralStorage>,

    /// The storage backend for reply-SURBs
    reply_storage_backend: B,

    /// Alternative provider of network topology used for constructing sphinx packets.
    custom_topology_provider: Option<Box<dyn TopologyProvider>>,
}

impl<B> DisconnectedMixnetClient<B>
where
    B: ReplyStorageBackend + Sync + Send + 'static,
{
    /// Create a new mixnet client in a disconnected state. The default configuration,
    /// creates a new mainnet client with ephemeral keys stored in RAM, which will be discarded at
    /// application close.
    ///
    /// Callers have the option of supplying further parameters to:
    /// - store persistent identities at a location on-disk, if desired;
    /// - use SOCKS5 mode
    async fn new(
        config: Config,
        socks5_config: Option<Socks5>,
        paths: Option<StoragePaths>,
        custom_topology_provider: Option<Box<dyn TopologyProvider>>,
    ) -> Result<DisconnectedMixnetClient<B>>
    where
        <B as ReplyStorageBackend>::StorageError: Send + Sync,
    {
        let reply_surb_database_path = paths.as_ref().map(|p| p.reply_surb_database_path.clone());

        let client_config =
            nym_validator_client::Config::try_from_nym_network_details(&config.network_details)?;
        let client = nym_validator_client::Client::new_query(client_config)?;
        let bandwidth_controller = BandwidthController::new(initialise_ephemeral_storage(), client);

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
                    let key_manager = nym_client_core::init::new_client_keys();
                    // WARN: this will overwrite!
                    key_manager.store_keys(&path_finder)?;
                    key_manager
                }
            }
        } else {
            // Ephemeral keys that we only store in memory
            log::debug!("Creating new ephemeral keys");
            nym_client_core::init::new_client_keys()
        };

        Ok(DisconnectedMixnetClient {
            key_manager,
            config,
            socks5_config,
            storage_paths: paths,
            state: BuilderState::New,
            reply_storage_backend,
            bandwidth_controller,
            custom_topology_provider,
        })
    }

    fn get_api_endpoints(&self) -> Vec<Url> {
        self.config
            .network_details
            .endpoints
            .iter()
            .filter_map(|details| details.api_url.as_ref())
            .filter_map(|s| Url::parse(s).ok())
            .collect()
    }

    /// Client keys are generated at client creation if none were found. The gateway shared
    /// key, however, is created during the gateway registration handshake so it might not
    /// necessarily be available.
    fn has_gateway_key(&self) -> bool {
        self.key_manager.is_gateway_key_set()
    }

    /// Sets the keys of this [`MixnetClientBuilder`].
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

    /// Sets the gateway endpoint of this [`MixnetClientBuilder`].
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

        let api_endpoints = self.get_api_endpoints();
        let gateway_config = nym_client_core::init::register_with_gateway::<EphemeralStorage>(
            &mut self.key_manager,
            api_endpoints,
            user_chosen_gateway,
            // TODO: this should probably be configurable with the config
            false,
        )
        .await?;

        self.state = BuilderState::Registered {
            gateway_endpoint_config: gateway_config,
        };
        Ok(())
    }

    /// Returns the get gateway endpoint of this [`MixnetClientBuilder`].
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

    /// Creates an associated [`BandwidthAcquireClient`] that can be used to acquire bandwidth
    /// credentials for this client to consume.
    pub fn create_bandwidth_client(&self, mnemonic: String) -> Result<BandwidthAcquireClient> {
        if !self.config.enabled_credentials_mode {
            return Err(Error::DisabledCredentialsMode);
        }
        BandwidthAcquireClient::new(
            self.config.network_details.clone(),
            mnemonic,
            self.bandwidth_controller.storage().clone(),
        )
    }

    async fn connect_to_mixnet_common(mut self) -> Result<(BaseClient, Recipient)>
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

        let api_endpoints = self.get_api_endpoints();

        // At this point we should be in a registered state, either at function entry or by the
        // above convenience logic.
        let BuilderState::Registered { gateway_endpoint_config } = self.state else {
            return Err(Error::FailedToTransitionToRegisteredState);
        };

        let nym_address =
            nym_client_core::init::get_client_address(&self.key_manager, &gateway_endpoint_config);

        let mut base_builder: BaseClientBuilder<'_, _, Client<QueryNyxdClient>, EphemeralStorage> =
            BaseClientBuilder::new(
                &gateway_endpoint_config,
                &self.config.debug_config,
                self.key_manager.clone(),
                Some(self.bandwidth_controller),
                self.reply_storage_backend,
                CredentialsToggle::from(self.config.enabled_credentials_mode),
                api_endpoints,
            );

        if let Some(topology_provider) = self.custom_topology_provider {
            base_builder = base_builder.with_topology_provider(topology_provider);
        }

        let started_client = base_builder.start_base().await?;

        Ok((started_client, nym_address))
    }

    /// Connect the client to the mixnet via SOCKS5. A SOCKS5 configuration must be specified
    /// before attempting to connect.
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
    ///     let receiving_client = mixnet::MixnetClient::connect_new().await.unwrap();
    ///     let socks5_config = mixnet::Socks5::new(receiving_client.nym_address().to_string());
    ///     let client = mixnet::MixnetClientBuilder::new()
    ///         .socks5_config(socks5_config)
    ///         .build::<mixnet::EmptyReplyStorage>()
    ///         .await
    ///         .unwrap();
    ///     let client = client.connect_to_mixnet_via_socks5().await.unwrap();
    /// }
    /// ```
    pub async fn connect_to_mixnet_via_socks5(self) -> Result<Socks5MixnetClient>
    where
        <B as ReplyStorageBackend>::StorageError: Sync + Send,
    {
        let key_manager = self.key_manager.clone();
        let socks5_config = self
            .socks5_config
            .clone()
            .ok_or(Error::Socks5Config { set: false })?;
        let debug_config = self.config.debug_config;
        let packet_type = self.config.packet_type();
        let (mut started_client, nym_address) = self.connect_to_mixnet_common().await?;
        let (socks5_status_tx, mut socks5_status_rx) = mpsc::channel(128);

        let client_input = started_client.client_input.register_producer();
        let client_output = started_client.client_output.register_consumer();
        let client_state = started_client.client_state;

        nym_socks5_client_core::NymClient::start_socks5_listener(
            &socks5_config,
            debug_config,
            client_input,
            client_output,
            client_state.clone(),
            nym_address,
            started_client.task_manager.subscribe(),
            packet_type,
        );
        started_client
            .task_manager
            .start_status_listener(socks5_status_tx)
            .await;
        match socks5_status_rx
            .next()
            .await
            .ok_or(Error::Socks5NotStarted)?
            .downcast_ref::<TaskStatus>()
            .ok_or(Error::Socks5NotStarted)?
        {
            TaskStatus::Ready => {
                log::debug!("Socks5 connected");
            }
        }

        Ok(Socks5MixnetClient {
            nym_address,
            key_manager,
            client_state,
            task_manager: started_client.task_manager,
            socks5_config,
        })
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
    pub async fn connect_to_mixnet(self) -> Result<MixnetClient>
    where
        <B as ReplyStorageBackend>::StorageError: Sync + Send,
    {
        if self.socks5_config.is_some() {
            return Err(Error::Socks5Config { set: true });
        }
        let key_manager = self.key_manager.clone();
        let (mut started_client, nym_address) = self.connect_to_mixnet_common().await?;
        let client_input = started_client.client_input.register_producer();
        let mut client_output = started_client.client_output.register_consumer();
        let client_state = started_client.client_state;

        let reconstructed_receiver = client_output.register_receiver()?;

        Ok(MixnetClient {
            nym_address,
            key_manager,
            client_input,
            client_output,
            client_state,
            reconstructed_receiver,
            task_manager: started_client.task_manager,
            packet_type: None,
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
