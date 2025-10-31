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
