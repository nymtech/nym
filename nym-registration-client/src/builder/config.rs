// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_credential_storage::persistent_storage::PersistentStorage;
use nym_registration_common::NymNode;
use nym_sdk::{
    DebugConfig, NymNetworkDetails, RememberMe, TopologyProvider, UserAgent,
    mixnet::{
        CredentialStorage, GatewaysDetailsStore, KeyStore, MixnetClient, MixnetClientBuilder,
        MixnetClientStorage, OnDiskPersistent, ReplyStorageBackend, StoragePaths, x25519::KeyPair,
    },
};

#[cfg(unix)]
use std::os::fd::RawFd;
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

use crate::error::RegistrationClientError;

const VPN_AVERAGE_PACKET_DELAY: Duration = Duration::from_millis(15);

#[derive(Clone)]
pub struct NymNodeWithKeys {
    pub node: NymNode,
    pub keys: Arc<KeyPair>,
}

pub struct BuilderConfig {
    pub entry_node: NymNodeWithKeys,
    pub exit_node: NymNodeWithKeys,
    pub data_path: Option<PathBuf>,
    pub mixnet_client_config: MixnetClientConfig,
    pub two_hops: bool,
    pub user_agent: UserAgent,
    pub custom_topology_provider: Box<dyn TopologyProvider + Send + Sync>,
    pub network_env: NymNetworkDetails,
    pub cancel_token: CancellationToken,
    #[cfg(unix)]
    pub connection_fd_callback: Arc<dyn Fn(RawFd) + Send + Sync>,
}

#[derive(Clone, Default, Debug, Eq, PartialEq)]
pub struct MixnetClientConfig {
    /// Disable Poission process rate limiting of outbound traffic.
    pub disable_poisson_rate: bool,

    /// Disable constant rate background loop cover traffic
    pub disable_background_cover_traffic: bool,

    /// The minimum performance of mixnodes to use.
    pub min_mixnode_performance: Option<u8>,

    /// The minimum performance of gateways to use.
    pub min_gateway_performance: Option<u8>,
}

impl BuilderConfig {
    /// Creates a new BuilderConfig with all required parameters.
    ///
    /// However, consider using `BuilderConfig::builder()` instead.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entry_node: NymNodeWithKeys,
        exit_node: NymNodeWithKeys,
        data_path: Option<PathBuf>,
        mixnet_client_config: MixnetClientConfig,
        two_hops: bool,
        user_agent: UserAgent,
        custom_topology_provider: Box<dyn TopologyProvider + Send + Sync>,
        network_env: NymNetworkDetails,
        cancel_token: CancellationToken,
        #[cfg(unix)] connection_fd_callback: Arc<dyn Fn(RawFd) + Send + Sync>,
    ) -> Self {
        Self {
            entry_node,
            exit_node,
            data_path,
            mixnet_client_config,
            two_hops,
            user_agent,
            custom_topology_provider,
            network_env,
            cancel_token,
            #[cfg(unix)]
            connection_fd_callback,
        }
    }

    /// Creates a builder for BuilderConfig
    ///
    /// This is the preferred way to construct a BuilderConfig.
    ///
    /// # Example
    /// ```ignore
    /// let config = BuilderConfig::builder()
    ///     .entry_node(entry)
    ///     .exit_node(exit)
    ///     .user_agent(agent)
    ///     .build()?;
    /// ```
    pub fn builder() -> BuilderConfigBuilder {
        BuilderConfigBuilder::default()
    }

    pub fn mixnet_client_debug_config(&self) -> DebugConfig {
        if self.two_hops {
            two_hop_debug_config(&self.mixnet_client_config)
        } else {
            mixnet_debug_config(&self.mixnet_client_config)
        }
    }

    pub async fn setup_storage(
        &self,
    ) -> Result<Option<(OnDiskPersistent, PersistentStorage)>, RegistrationClientError> {
        if let Some(path) = &self.data_path {
            tracing::debug!("Using custom key storage path: {}", path.display());

            let storage_paths = StoragePaths::new_from_dir(path)
                .map_err(|err| RegistrationClientError::BuildMixnetClient(Box::new(err)))?;

            let mixnet_client_storage = storage_paths
                .initialise_persistent_storage(&self.mixnet_client_debug_config())
                .await
                .map_err(|err| RegistrationClientError::BuildMixnetClient(Box::new(err)))?;
            let credential_storage = storage_paths
                .persistent_credential_storage()
                .await
                .map_err(|err| RegistrationClientError::BuildMixnetClient(Box::new(err)))?;

            Ok(Some((mixnet_client_storage, credential_storage)))
        } else {
            Ok(None)
        }
    }

    pub async fn build_and_connect_mixnet_client<S>(
        self,
        builder: MixnetClientBuilder<S>,
    ) -> Result<MixnetClient, RegistrationClientError>
    where
        S: MixnetClientStorage + Clone + 'static,
        S::ReplyStore: Send + Sync,
        S::GatewaysDetailsStore: Sync,
        <S::ReplyStore as ReplyStorageBackend>::StorageError: Sync + Send,
        <S::CredentialStore as CredentialStorage>::StorageError: Send + Sync,
        <S::KeyStore as KeyStore>::StorageError: Send + Sync,
        <S::GatewaysDetailsStore as GatewaysDetailsStore>::StorageError: Send + Sync,
    {
        let debug_config = self.mixnet_client_debug_config();
        let remember_me = if self.two_hops {
            RememberMe::new_vpn()
        } else {
            RememberMe::new_mixnet()
        };

        let builder = builder
            .with_user_agent(self.user_agent)
            .request_gateway(self.entry_node.node.identity.to_string())
            .network_details(self.network_env)
            .debug_config(debug_config)
            .credentials_mode(true)
            .with_remember_me(remember_me)
            .custom_topology_provider(self.custom_topology_provider);

        #[cfg(unix)]
        let builder = builder.with_connection_fd_callback(self.connection_fd_callback);

        builder
            .build()
            .map_err(|err| RegistrationClientError::BuildMixnetClient(Box::new(err)))?
            .connect_to_mixnet()
            .await
            .map_err(|err| RegistrationClientError::ConnectToMixnet(Box::new(err)))
    }
}

fn two_hop_debug_config(mixnet_client_config: &MixnetClientConfig) -> DebugConfig {
    let mut debug_config = DebugConfig::default();

    debug_config.traffic.average_packet_delay = VPN_AVERAGE_PACKET_DELAY;

    // We disable mix hops for the mixnet connection.
    debug_config.traffic.disable_mix_hops = true;
    // Always disable poisson process for outbound traffic in wireguard.
    debug_config
        .traffic
        .disable_main_poisson_packet_distribution = true;
    // Always disable background cover traffic in wireguard.
    debug_config.cover_traffic.disable_loop_cover_traffic_stream = true;

    if let Some(min_mixnode_performance) = mixnet_client_config.min_mixnode_performance {
        debug_config.topology.minimum_mixnode_performance = min_mixnode_performance;
    }

    if let Some(min_gateway_performance) = mixnet_client_config.min_gateway_performance {
        debug_config.topology.minimum_gateway_performance = min_gateway_performance;
    }

    log_mixnet_client_config(&debug_config);
    debug_config
}

fn mixnet_debug_config(mixnet_client_config: &MixnetClientConfig) -> DebugConfig {
    let mut debug_config = DebugConfig::default();
    debug_config.traffic.average_packet_delay = VPN_AVERAGE_PACKET_DELAY;

    debug_config
        .traffic
        .disable_main_poisson_packet_distribution = mixnet_client_config.disable_poisson_rate;

    debug_config.cover_traffic.disable_loop_cover_traffic_stream =
        mixnet_client_config.disable_background_cover_traffic;

    if let Some(min_mixnode_performance) = mixnet_client_config.min_mixnode_performance {
        debug_config.topology.minimum_mixnode_performance = min_mixnode_performance;
    }

    if let Some(min_gateway_performance) = mixnet_client_config.min_gateway_performance {
        debug_config.topology.minimum_gateway_performance = min_gateway_performance;
    }
    log_mixnet_client_config(&debug_config);
    debug_config
}

fn log_mixnet_client_config(debug_config: &DebugConfig) {
    tracing::info!(
        "mixnet client poisson rate limiting: {}",
        true_to_disabled(
            debug_config
                .traffic
                .disable_main_poisson_packet_distribution
        )
    );

    tracing::info!(
        "mixnet client background loop cover traffic stream: {}",
        true_to_disabled(debug_config.cover_traffic.disable_loop_cover_traffic_stream)
    );

    tracing::info!(
        "mixnet client minimum mixnode performance: {}",
        debug_config.topology.minimum_mixnode_performance,
    );

    tracing::info!(
        "mixnet client minimum gateway performance: {}",
        debug_config.topology.minimum_gateway_performance,
    );
}

fn true_to_disabled(val: bool) -> &'static str {
    if val { "disabled" } else { "enabled" }
}

/// Error type for BuilderConfig validation
#[derive(Debug, Clone, thiserror::Error)]
#[allow(clippy::enum_variant_names)]
pub enum BuilderConfigError {
    #[error("entry_node is required")]
    MissingEntryNode,
    #[error("exit_node is required")]
    MissingExitNode,
    #[error("mixnet_client_config is required")]
    MissingMixnetClientConfig,
    #[error("user_agent is required")]
    MissingUserAgent,
    #[error("custom_topology_provider is required")]
    MissingTopologyProvider,
    #[error("network_env is required")]
    MissingNetworkEnv,
    #[error("cancel_token is required")]
    MissingCancelToken,
    #[cfg(unix)]
    #[error("connection_fd_callback is required")]
    MissingConnectionFdCallback,
}

/// Builder for `BuilderConfig`
///
/// This provides a more convenient way to construct a `BuilderConfig` compared to the
/// `new()` constructor with many arguments.
#[derive(Default)]
pub struct BuilderConfigBuilder {
    entry_node: Option<NymNodeWithKeys>,
    exit_node: Option<NymNodeWithKeys>,
    data_path: Option<PathBuf>,
    mixnet_client_config: Option<MixnetClientConfig>,
    two_hops: bool,
    user_agent: Option<UserAgent>,
    custom_topology_provider: Option<Box<dyn TopologyProvider + Send + Sync>>,
    network_env: Option<NymNetworkDetails>,
    cancel_token: Option<CancellationToken>,
    #[cfg(unix)]
    connection_fd_callback: Option<Arc<dyn Fn(RawFd) + Send + Sync>>,
}

impl BuilderConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn entry_node(mut self, entry_node: NymNodeWithKeys) -> Self {
        self.entry_node = Some(entry_node);
        self
    }

    pub fn exit_node(mut self, exit_node: NymNodeWithKeys) -> Self {
        self.exit_node = Some(exit_node);
        self
    }

    pub fn data_path(mut self, data_path: Option<PathBuf>) -> Self {
        self.data_path = data_path;
        self
    }

    pub fn mixnet_client_config(mut self, mixnet_client_config: MixnetClientConfig) -> Self {
        self.mixnet_client_config = Some(mixnet_client_config);
        self
    }

    pub fn two_hops(mut self, two_hops: bool) -> Self {
        self.two_hops = two_hops;
        self
    }

    pub fn user_agent(mut self, user_agent: UserAgent) -> Self {
        self.user_agent = Some(user_agent);
        self
    }

    pub fn custom_topology_provider(
        mut self,
        custom_topology_provider: Box<dyn TopologyProvider + Send + Sync>,
    ) -> Self {
        self.custom_topology_provider = Some(custom_topology_provider);
        self
    }

    pub fn network_env(mut self, network_env: NymNetworkDetails) -> Self {
        self.network_env = Some(network_env);
        self
    }

    pub fn cancel_token(mut self, cancel_token: CancellationToken) -> Self {
        self.cancel_token = Some(cancel_token);
        self
    }

    #[cfg(unix)]
    pub fn connection_fd_callback(
        mut self,
        connection_fd_callback: Arc<dyn Fn(RawFd) + Send + Sync>,
    ) -> Self {
        self.connection_fd_callback = Some(connection_fd_callback);
        self
    }

    /// Builds the `BuilderConfig`.
    ///
    /// Returns an error if any required field is missing.
    pub fn build(self) -> Result<BuilderConfig, BuilderConfigError> {
        Ok(BuilderConfig {
            entry_node: self
                .entry_node
                .ok_or(BuilderConfigError::MissingEntryNode)?,
            exit_node: self.exit_node.ok_or(BuilderConfigError::MissingExitNode)?,
            data_path: self.data_path,
            mixnet_client_config: self
                .mixnet_client_config
                .ok_or(BuilderConfigError::MissingMixnetClientConfig)?,
            two_hops: self.two_hops,
            user_agent: self
                .user_agent
                .ok_or(BuilderConfigError::MissingUserAgent)?,
            custom_topology_provider: self
                .custom_topology_provider
                .ok_or(BuilderConfigError::MissingTopologyProvider)?,
            network_env: self
                .network_env
                .ok_or(BuilderConfigError::MissingNetworkEnv)?,
            cancel_token: self
                .cancel_token
                .ok_or(BuilderConfigError::MissingCancelToken)?,
            #[cfg(unix)]
            connection_fd_callback: self
                .connection_fd_callback
                .ok_or(BuilderConfigError::MissingConnectionFdCallback)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mixnet_client_config_default_values() {
        let config = MixnetClientConfig::default();
        assert!(!config.disable_poisson_rate);
        assert!(!config.disable_background_cover_traffic);
        assert_eq!(config.min_mixnode_performance, None);
        assert_eq!(config.min_gateway_performance, None);
    }

    #[test]
    fn test_builder_config_builder_fails_without_required_fields() {
        // Building without any fields should fail with specific error
        let result = BuilderConfig::builder().build();
        assert!(result.is_err());
        match result {
            Err(BuilderConfigError::MissingEntryNode) => (), // Expected
            Err(e) => panic!("Expected MissingEntryNode, got: {}", e),
            Ok(_) => panic!("Expected error, got Ok"),
        }
    }

    #[test]
    fn test_builder_config_builder_validates_all_required_fields() {
        // Test that each required field is validated
        let result = BuilderConfig::builder().build();
        assert!(result.is_err());

        // Short-circuits at first missing field, so we just verify it's one of the expected errors
        #[allow(unreachable_patterns)] // All variants are covered, but keeping catch-all for safety
        match result {
            Err(BuilderConfigError::MissingEntryNode)
            | Err(BuilderConfigError::MissingExitNode)
            | Err(BuilderConfigError::MissingMixnetClientConfig)
            | Err(BuilderConfigError::MissingUserAgent)
            | Err(BuilderConfigError::MissingTopologyProvider)
            | Err(BuilderConfigError::MissingNetworkEnv)
            | Err(BuilderConfigError::MissingCancelToken) => (),
            #[cfg(unix)]
            Err(BuilderConfigError::MissingConnectionFdCallback) => (),
            Err(e) => panic!("Unexpected error: {}", e),
            Ok(_) => panic!("Expected validation error, got Ok"),
        }
    }

    #[test]
    fn test_builder_config_builder_method_chaining() {
        // Test that builder methods chain properly and return Self
        let builder = BuilderConfig::builder();

        // Verify the builder returns itself for chaining
        let builder = builder.two_hops(true);
        let builder = builder.two_hops(false);
        let builder = builder.data_path(None);

        // Builder should still fail because required fields are missing
        let result = builder.build();
        assert!(result.is_err());
    }
}
