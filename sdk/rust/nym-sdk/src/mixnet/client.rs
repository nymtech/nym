// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::{connection_state::BuilderState, Config, StoragePaths};
use crate::bandwidth::BandwidthAcquireClient;
use crate::mixnet::socks5_client::Socks5MixnetClient;
use crate::mixnet::{CredentialStorage, MixnetClient, Recipient};
use crate::{Error, Result};
use futures::channel::mpsc;
use futures::StreamExt;
use nym_bandwidth_controller::BandwidthController;
use nym_client_core::client::base_client::storage::{
    Ephemeral, MixnetClientStorage, OnDiskPersistent,
};
use nym_client_core::client::base_client::BaseClient;
use nym_client_core::client::key_manager::persistence::KeyStore;
use nym_client_core::client::key_manager::ManagedKeys;
use nym_client_core::config::DebugConfig;
use nym_client_core::init::GatewaySetup;
use nym_client_core::{
    client::{
        base_client::{BaseClientBuilder, CredentialsToggle},
        replies::reply_storage::ReplyStorageBackend,
    },
    config::GatewayEndpointConfig,
};
use nym_crypto::asymmetric::identity;
use nym_network_defaults::NymNetworkDetails;
use nym_socks5_client_core::config::Socks5;
use nym_task::manager::TaskStatus;
use nym_topology::provider_trait::TopologyProvider;
use nym_validator_client::nyxd::QueryNyxdClient;
use nym_validator_client::Client;
use rand::thread_rng;
use std::path::Path;
use std::path::PathBuf;
use url::Url;

// The number of surbs to include in a message by default
const DEFAULT_NUMBER_OF_SURBS: u32 = 5;

#[derive(Default)]
pub struct MixnetClientBuilder<S: MixnetClientStorage = Ephemeral> {
    config: Config,
    storage_paths: Option<StoragePaths>,
    gateway_config: Option<GatewayEndpointConfig>,
    socks5_config: Option<Socks5>,
    custom_topology_provider: Option<Box<dyn TopologyProvider>>,

    // TODO: incorporate it properly into `MixnetClientStorage` (I will need it in wasm anyway)
    gateway_endpoint_config_path: Option<PathBuf>,

    storage: S,
}

impl MixnetClientBuilder<Ephemeral> {
    /// Creates a client builder with ephemeral storage.
    #[must_use]
    pub fn new_ephemeral() -> Self {
        MixnetClientBuilder {
            ..Default::default()
        }
    }

    /// Create a client builder with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::new_ephemeral()
    }
}

impl MixnetClientBuilder<OnDiskPersistent> {
    pub async fn new_with_default_storage(storage_paths: StoragePaths) -> Result<Self> {
        Ok(MixnetClientBuilder {
            config: Default::default(),
            storage_paths: None,
            gateway_config: None,
            socks5_config: None,
            custom_topology_provider: None,
            storage: storage_paths
                .initialise_default_persistent_storage()
                .await?,
            gateway_endpoint_config_path: None,
        })
    }
}

impl<S> MixnetClientBuilder<S>
where
    S: MixnetClientStorage + 'static,
    <S::ReplyStore as ReplyStorageBackend>::StorageError: Sync + Send,
    <S::CredentialStore as CredentialStorage>::StorageError: Send + Sync,
    S::ReplyStore: Send + Sync,
    <S::KeyStore as KeyStore>::StorageError: Send + Sync,
{
    /// Creates a client builder with the provided client storage implementation.
    #[must_use]
    pub fn new_with_storage(storage: S) -> MixnetClientBuilder<S> {
        MixnetClientBuilder {
            config: Default::default(),
            storage_paths: None,
            gateway_config: None,
            socks5_config: None,
            custom_topology_provider: None,
            gateway_endpoint_config_path: None,
            storage,
        }
    }

    /// Change the underlying storage implementation.
    #[must_use]
    pub fn set_storage<T: MixnetClientStorage>(self, storage: T) -> MixnetClientBuilder<T> {
        MixnetClientBuilder {
            config: self.config,
            storage_paths: self.storage_paths,
            gateway_config: self.gateway_config,
            socks5_config: self.socks5_config,
            custom_topology_provider: self.custom_topology_provider,
            gateway_endpoint_config_path: self.gateway_endpoint_config_path,
            storage,
        }
    }

    /// Change the underlying storage of this builder to use default implementation of on-disk persistence.
    #[must_use]
    pub fn set_default_storage(
        self,
        storage: OnDiskPersistent,
    ) -> MixnetClientBuilder<OnDiskPersistent> {
        self.set_storage(storage)
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

    /// Use specified file for storing gateway configuration.
    pub fn gateway_endpoint_config_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.gateway_endpoint_config_path = Some(path.as_ref().to_owned());
        self
    }

    /// Construct a [`DisconnectedMixnetClient`] from the setup specified.
    pub async fn build(self) -> Result<DisconnectedMixnetClient<S>> {
        let mut client = DisconnectedMixnetClient::new(
            self.config,
            self.socks5_config,
            self.storage,
            self.custom_topology_provider,
            self.gateway_endpoint_config_path,
        )
        .await?;

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
pub struct DisconnectedMixnetClient<S>
where
    S: MixnetClientStorage,
{
    /// Client configuration
    config: Config,

    /// Socks5 configuration
    socks5_config: Option<Socks5>,

    /// The client can be in one of multiple states, depending on how it is created and if it's
    /// connected to the mixnet.
    state: BuilderState,

    // TODO: refactor storages and combine everything into a single struct
    /// Controller of bandwidth credentials that the mixnet client can use to connect
    bandwidth_controller: Option<BandwidthController<Client<QueryNyxdClient>, S::CredentialStore>>,

    /// The storage backend for reply-SURBs
    reply_storage_backend: S::ReplyStore,

    /// The storage backend for cryptographic keys
    key_store: S::KeyStore,

    /// Keys handled by the client
    managed_keys: ManagedKeys,

    // TODO: incorporate it properly into `MixnetClientStorage` (I will need it in wasm anyway)
    /// Path to optionally persist gateway configuration. Note that it's required if one were to use persistent keys.
    gateway_endpoint_config_path: Option<PathBuf>,

    /// Alternative provider of network topology used for constructing sphinx packets.
    custom_topology_provider: Option<Box<dyn TopologyProvider>>,
}

impl<S> DisconnectedMixnetClient<S>
where
    S: MixnetClientStorage + 'static,
    <S::ReplyStore as ReplyStorageBackend>::StorageError: Sync + Send,
    <S::CredentialStore as CredentialStorage>::StorageError: Send + Sync,
    S::ReplyStore: Send + Sync,
    <S::KeyStore as KeyStore>::StorageError: Send + Sync,
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
        storage: S,
        custom_topology_provider: Option<Box<dyn TopologyProvider>>,
        gateway_endpoint_config_path: Option<PathBuf>,
    ) -> Result<DisconnectedMixnetClient<S>> {
        let (key_store, reply_storage_backend, credential_store) = storage.into_split();

        // don't create bandwidth controller if credentials are disabled
        let bandwidth_controller = if config.enabled_credentials_mode {
            let client_config = nym_validator_client::Config::try_from_nym_network_details(
                &config.network_details,
            )?;
            let client = nym_validator_client::Client::new_query(client_config)?;
            Some(BandwidthController::new(credential_store, client))
        } else {
            None
        };

        let mut rng = thread_rng();
        let managed_keys = ManagedKeys::load_or_generate(&mut rng, &key_store).await;

        Ok(DisconnectedMixnetClient {
            config,
            socks5_config,
            state: BuilderState::New,
            reply_storage_backend,
            key_store,
            bandwidth_controller,
            custom_topology_provider,
            managed_keys,
            gateway_endpoint_config_path,
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
        matches!(self.managed_keys, ManagedKeys::FullyDerived(..))
    }

    fn remove_gateway_key(&mut self) {
        assert!(self.has_gateway_key());
        let ManagedKeys::FullyDerived(keys) = std::mem::replace(&mut self.managed_keys, ManagedKeys::Invalidated) else {
            unreachable!()
        };
        self.managed_keys = ManagedKeys::Initial(keys.remove_gateway_key())
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

        let api_endpoints = self.get_api_endpoints();
        let gateway_setup = GatewaySetup::new(None, self.config.user_chosen_gateway.clone(), None);

        let gateway_config = nym_client_core::init::get_registered_gateway::<S>(
            api_endpoints,
            &self.key_store,
            gateway_setup,
            !self.config.key_mode.is_keep(),
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

    fn read_gateway_endpoint_config<P: AsRef<Path>>(
        &mut self,
        gateway_endpoint_config_path: P,
    ) -> Result<()> {
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
    pub fn create_bandwidth_client(
        &self,
        mnemonic: String,
    ) -> Result<BandwidthAcquireClient<S::CredentialStore>> {
        if !self.config.enabled_credentials_mode {
            return Err(Error::DisabledCredentialsMode);
        }
        if let Some(bandwidth_controller) = &self.bandwidth_controller {
            BandwidthAcquireClient::new(
                self.config.network_details.clone(),
                mnemonic,
                bandwidth_controller.storage(),
            )
        } else {
            Err(Error::DisabledCredentialsMode)
        }
    }

    async fn connect_to_mixnet_common(mut self) -> Result<(BaseClient, Recipient)> {
        // For some simple cases we can figure how to setup gateway without it having to have been
        // called in advance.
        if matches!(self.state, BuilderState::New) {
            let already_registered = self.has_gateway_key();
            if already_registered {
                if let Some(gateway_endpoint_path) = self.gateway_endpoint_config_path.clone() {
                    self.read_gateway_endpoint_config(gateway_endpoint_path)?;
                } else if !self.config.key_mode.is_keep() {
                    // if we don't have gateway configuration available and we're not keeping the keys,
                    // purge them
                    self.remove_gateway_key();
                }
            } else {
                // TODO: that is redundant since the base client will perform gateway registration
                self.register_and_authenticate_gateway().await?;
                if let Some(gateway_endpoint_path) = &self.gateway_endpoint_config_path {
                    self.write_gateway_endpoint_config(gateway_endpoint_path)?;
                }
            }
        }

        // If the gateway is in a registered state, but without the gateway key set.
        if matches!(self.state, BuilderState::Registered { .. }) && !self.has_gateway_key() {
            return Err(Error::NoGatewayKeySet);
        }

        // At this point we should be in a registered state, either at function entry or by the
        // above convenience logic.
        let BuilderState::Registered { gateway_endpoint_config } = &self.state else {
            return Err(Error::FailedToTransitionToRegisteredState);
        };

        let nym_api_endpoints = self.get_api_endpoints();

        let mut base_builder: BaseClientBuilder<_, S> = BaseClientBuilder::new(
            gateway_endpoint_config,
            &self.config.debug_config,
            self.key_store,
            self.bandwidth_controller,
            self.reply_storage_backend,
            CredentialsToggle::from(self.config.enabled_credentials_mode),
            nym_api_endpoints,
        );

        if let Some(topology_provider) = self.custom_topology_provider {
            base_builder = base_builder.with_topology_provider(topology_provider);
        }

        let started_client = base_builder.start_base().await?;
        let nym_address = started_client.address;

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
    ///     let client = mixnet::MixnetClientBuilder::new_ephemeral()
    ///         .socks5_config(socks5_config)
    ///         .build()
    ///         .await
    ///         .unwrap();
    ///     let client = client.connect_to_mixnet_via_socks5().await.unwrap();
    /// }
    /// ```
    pub async fn connect_to_mixnet_via_socks5(self) -> Result<Socks5MixnetClient> {
        let socks5_config = self
            .socks5_config
            .clone()
            .ok_or(Error::Socks5Config { set: false })?;
        let debug_config = self.config.debug_config;
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
    ///     let client = mixnet::MixnetClientBuilder::new_ephemeral()
    ///         .build()
    ///         .await
    ///         .unwrap();
    ///     let client = client.connect_to_mixnet().await.unwrap();
    /// }
    /// ```
    pub async fn connect_to_mixnet(self) -> Result<MixnetClient> {
        if self.socks5_config.is_some() {
            return Err(Error::Socks5Config { set: true });
        }
        let (mut started_client, nym_address) = self.connect_to_mixnet_common().await?;
        let client_input = started_client.client_input.register_producer();
        let mut client_output = started_client.client_output.register_consumer();
        let client_state = started_client.client_state;

        let reconstructed_receiver = client_output.register_receiver()?;

        Ok(MixnetClient {
            nym_address,
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
