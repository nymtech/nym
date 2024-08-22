// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::support::nyxd;
use crate::circulating_supply_api::cache::CirculatingSupplyCache;
use crate::ecash::api_routes::handlers::ecash_routes;
use crate::ecash::client::Client;
use crate::ecash::comm::QueryCommunicationChannel;
use crate::ecash::dkg::controller::keys::{
    can_validate_coconut_keys, load_bte_keypair, load_ecash_keypair_if_exists,
};
use crate::ecash::dkg::controller::DkgController;
use crate::ecash::state::EcashState;
use crate::epoch_operations::{self, RewardedSetUpdater};
use crate::network::models::NetworkDetails;
use crate::node_describe_cache::{self, DescribedNodes};
use crate::node_status_api::handlers::unstable;
use crate::node_status_api::uptime_updater::HistoricalUptimeUpdater;
use crate::node_status_api::{self, NodeStatusCache};
use crate::nym_contract_cache::cache::NymContractCache;
use crate::status::{ApiStatusState, SignerState};
use crate::support::caching::cache::SharedCache;
use crate::support::config::Config;
use crate::support::storage;
use crate::{circulating_supply_api, ecash, network_monitor, nym_contract_cache};
use anyhow::{bail, Context};
use nym_config::defaults::NymNetworkDetails;
use nym_sphinx::receiver::SphinxMessageReceiver;
use nym_task::TaskManager;
use nym_validator_client::nyxd::Coin;
use router::RouterBuilder;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

pub(crate) mod api_docs;
pub(crate) mod router;

/// Shutdown goes 2 directions:
/// 1. signal background tasks to gracefully finish
/// 2. signal server itself
///
/// These are done through separate shutdown handles. Ofcourse, shut down server
/// AFTER you have shut down BG tasks (or past their grace period).
pub(crate) struct ShutdownHandles {
    task_manager: TaskManager,
    axum_shutdown_button: ShutdownAxum,
    /// Tokio JoinHandle for axum server's task
    axum_join_handle: AxumJoinHandle,
}

impl ShutdownHandles {
    /// Cancellation token is given to Axum server constructor. When the token
    /// receives a shutdown signal, Axum server will shut down gracefully.
    pub(crate) fn new(
        task_manager: TaskManager,
        axum_server_handle: AxumJoinHandle,
        shutdown_button: CancellationToken,
    ) -> Self {
        Self {
            task_manager,
            axum_shutdown_button: ShutdownAxum(shutdown_button.clone()),
            axum_join_handle: axum_server_handle,
        }
    }

    pub(crate) fn task_manager_mut(&mut self) -> &mut TaskManager {
        &mut self.task_manager
    }

    /// Signal server to shut down, then return join handle to its
    /// `tokio` task
    ///
    /// https://tikv.github.io/doc/tokio/task/struct.JoinHandle.html
    #[must_use]
    pub(crate) fn shutdown_axum(self) -> AxumJoinHandle {
        self.axum_shutdown_button.0.cancel();
        self.axum_join_handle
    }
}

struct ShutdownAxum(CancellationToken);

type AxumJoinHandle = JoinHandle<Result<(), std::io::Error>>;

#[derive(Clone)]
// TODO rocket remove smurf name after eliminating rocket
pub(crate) struct AxumAppState {
    nym_contract_cache: NymContractCache,
    node_status_cache: NodeStatusCache,
    circulating_supply_cache: CirculatingSupplyCache,
    storage: storage::NymApiStorage,
    described_nodes_state: SharedCache<DescribedNodes>,
    network_details: NetworkDetails,
    node_info_cache: unstable::NodeInfoCache,
}

impl AxumAppState {
    pub(crate) fn nym_contract_cache(&self) -> &NymContractCache {
        &self.nym_contract_cache
    }

    pub(crate) fn node_status_cache(&self) -> &NodeStatusCache {
        &self.node_status_cache
    }

    pub(crate) fn circulating_supply_cache(&self) -> &CirculatingSupplyCache {
        &self.circulating_supply_cache
    }

    pub(crate) fn network_details(&self) -> &NetworkDetails {
        &self.network_details
    }

    pub(crate) fn described_nodes_state(&self) -> &SharedCache<DescribedNodes> {
        &self.described_nodes_state
    }

    pub(crate) fn storage(&self) -> &storage::NymApiStorage {
        &self.storage
    }

    pub(crate) fn node_info_cache(&self) -> &unstable::NodeInfoCache {
        &self.node_info_cache
    }
}

pub(crate) async fn start_nym_api_tasks_v2(config: &Config) -> anyhow::Result<ShutdownHandles> {
    let nyxd_client = nyxd::Client::new(config);
    let connected_nyxd = config.get_nyxd_url();
    let nym_network_details = NymNetworkDetails::new_from_env();
    let network_details = NetworkDetails::new(connected_nyxd.to_string(), nym_network_details);

    let coconut_keypair = ecash::keys::KeyPair::new();

    // if the keypair doesnt exist (because say this API is running in the caching mode), nothing will happen
    if let Some(loaded_keys) = load_ecash_keypair_if_exists(&config.coconut_signer)? {
        let issued_for = loaded_keys.issued_for_epoch;
        coconut_keypair.set(loaded_keys).await;

        if can_validate_coconut_keys(&nyxd_client, issued_for).await? {
            coconut_keypair.validate()
        }
    }

    let identity_keypair = config.base.storage_paths.load_identity()?;
    let identity_public_key = *identity_keypair.public_key();

    let router = RouterBuilder::with_default_routes(config.network_monitor.enabled);

    let nym_contract_cache_state = NymContractCache::new();
    let node_status_cache_state = NodeStatusCache::new();
    let mix_denom = network_details.network.chain_details.mix_denom.base.clone();
    let circulating_supply_cache = CirculatingSupplyCache::new(mix_denom.to_owned());
    let described_nodes_state = SharedCache::<DescribedNodes>::new();
    let storage =
        storage::NymApiStorage::init(&config.node_status_api.storage_paths.database_path).await?;
    let node_info_cache = unstable::NodeInfoCache::default();

    let mut status_state = ApiStatusState::new();

    // if coconut signer is enabled, add /coconut to server
    let router = if config.coconut_signer.enabled {
        // make sure we have some tokens to cover multisig fees
        let balance = nyxd_client.balance(&mix_denom).await?;
        if balance.amount < ecash::MINIMUM_BALANCE {
            let address = nyxd_client.address().await;
            let min = Coin::new(ecash::MINIMUM_BALANCE, mix_denom);
            bail!("the account ({address}) doesn't have enough funds to cover verification fees. it has {balance} while it needs at least {min}")
        }

        let cosmos_address = nyxd_client.address().await.to_string();
        let announce_address = config
            .coconut_signer
            .announce_address
            .clone()
            .map(|u| u.to_string())
            .unwrap_or_default();
        status_state.add_zk_nym_signer(SignerState {
            cosmos_address,
            identity: identity_keypair.public_key().to_base58_string(),
            announce_address,
            coconut_keypair: coconut_keypair.clone(),
        });

        let ecash_contract = nyxd_client
            .get_ecash_contract_address()
            .await
            .context("e-cash contract address is required to setup the zk-nym signer")?;

        let comm_channel = QueryCommunicationChannel::new(nyxd_client.clone());

        let ecash_state = EcashState::new(
            ecash_contract,
            nyxd_client.clone(),
            identity_keypair,
            coconut_keypair.clone(),
            comm_channel,
            storage.clone(),
        )
        .await?;

        router.nest("/v1/ecash", ecash_routes(Arc::new(ecash_state)))
    } else {
        router
    };

    let router = router.with_state(AxumAppState {
        nym_contract_cache: nym_contract_cache_state.clone(),
        node_status_cache: node_status_cache_state.clone(),
        circulating_supply_cache: circulating_supply_cache.clone(),
        storage: storage.clone(),
        described_nodes_state: described_nodes_state.clone(),
        network_details,
        node_info_cache,
    });

    let task_manager = TaskManager::new(10);

    // start note describe cache refresher
    // we should be doing the below, but can't due to our current startup structure
    // let refresher = node_describe_cache::new_refresher(&config.topology_cacher);
    // let cache = refresher.get_shared_cache();
    node_describe_cache::new_refresher_with_initial_value(
        &config.topology_cacher,
        nym_contract_cache_state.clone(),
        described_nodes_state,
    )
    .named("node-self-described-data-refresher")
    .start(task_manager.subscribe_named("node-self-described-data-refresher"));

    // start all the caches first
    let nym_contract_cache_listener = nym_contract_cache::start_refresher(
        &config.node_status_api,
        &nym_contract_cache_state,
        nyxd_client.clone(),
        &task_manager,
    );
    node_status_api::start_cache_refresh(
        &config.node_status_api,
        &nym_contract_cache_state,
        &node_status_cache_state,
        storage.clone(),
        nym_contract_cache_listener,
        &task_manager,
    );
    circulating_supply_api::start_cache_refresh(
        &config.circulating_supply_cacher,
        nyxd_client.clone(),
        &circulating_supply_cache,
        &task_manager,
    );

    // start dkg task
    if config.coconut_signer.enabled {
        let dkg_bte_keypair = load_bte_keypair(&config.coconut_signer)?;

        DkgController::start(
            &config.coconut_signer,
            nyxd_client.clone(),
            coconut_keypair,
            dkg_bte_keypair,
            identity_public_key,
            rand::rngs::OsRng,
            &task_manager,
        )?;
    }

    // and then only start the uptime updater (and the monitor itself, duh)
    // if the monitoring is enabled
    if config.network_monitor.enabled {
        network_monitor::start::<SphinxMessageReceiver>(
            &config.network_monitor,
            &nym_contract_cache_state,
            &storage,
            nyxd_client.clone(),
            &task_manager,
        )
        .await;

        HistoricalUptimeUpdater::start(storage.to_owned(), &task_manager);

        // start 'rewarding' if its enabled
        if config.rewarding.enabled {
            epoch_operations::ensure_rewarding_permission(&nyxd_client).await?;
            RewardedSetUpdater::start(
                nyxd_client,
                &nym_contract_cache_state,
                storage,
                &task_manager,
            );
        }
    }

    let bind_address = config.base.bind_address.to_owned();
    let server = router.build_server(&bind_address).await?;

    let cancellation_token = CancellationToken::new();
    let shutdown_button = cancellation_token.clone();
    let axum_shutdown_receiver = cancellation_token.cancelled_owned();
    let server_handle = tokio::spawn(async move {
        {
            info!("Started Axum HTTP V2 server on {bind_address}");
            server.run(axum_shutdown_receiver).await
        }
    });

    let shutdown = ShutdownHandles::new(task_manager, server_handle, shutdown_button);

    Ok(shutdown)
}
