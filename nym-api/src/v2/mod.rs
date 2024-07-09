// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::support::nyxd;
use crate::circulating_supply_api::cache::CirculatingSupplyCache;
use crate::ecash::dkg::controller::keys::{
    can_validate_coconut_keys, load_bte_keypair, load_ecash_keypair_if_exists,
};
use crate::ecash::dkg::controller::DkgController;
use crate::epoch_operations::{self, RewardedSetUpdater};
use crate::network::models::NetworkDetails;
use crate::node_describe_cache::{self, DescribedNodes};
use crate::node_status_api::handlers::unstable;
use crate::node_status_api::uptime_updater::HistoricalUptimeUpdater;
use crate::node_status_api::{self, NodeStatusCache};
use crate::nym_contract_cache::cache::NymContractCache;
use crate::status::ApiStatusState;
use crate::support::caching::cache::SharedCache;
use crate::support::config::Config;
use crate::support::http::setup_routes;
use crate::support::storage;
use crate::{circulating_supply_api, ecash, network_monitor, nym_contract_cache};
use anyhow::anyhow;
use axum::Router;
use core::net::SocketAddr;
use nym_config::defaults::NymNetworkDetails;
use nym_sphinx::receiver::SphinxMessageReceiver;
use nym_task::TaskManager;
use std::net::Ipv4Addr;
use tokio::net::TcpListener;
use tokio_util::sync::{CancellationToken, WaitForCancellationFutureOwned};

pub(crate) struct ApiHttpServer {
    // task_client: Option<TaskClient>,
    // inner: AxumServer,
    router: Router,
    // should not be a field, parameter instead to run_server or something
    listener: TcpListener,
}

impl ApiHttpServer {
    pub async fn build(bind_address: &SocketAddr, router: Router) -> anyhow::Result<Self> {
        let listener = tokio::net::TcpListener::bind(bind_address)
            .await
            .map_err(|err| anyhow!("Couldn't bind to address {} due to {}", bind_address, err))?;
        // let inner = axum::serve(listener, router.inner.into_make_service_with_connect_info());
        let server = ApiHttpServer { router, listener };

        Ok(server)
    }

    pub async fn run(self, receiver: WaitForCancellationFutureOwned) {
        let inner = axum::serve(
            self.listener,
            self.router
                .into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(receiver);
        if let Err(err) = inner.await {
            error!("the HTTP server has terminated with the error: {err}");
        } else {
            info!("the HTTP server has terminated without errors");
        }
    }
}

/// Shutdown goes 2 directions:
/// 1. signal background tasks to gracefully finish
/// 2. signal server itself
///
/// These are done through separate shutdown handles. Ofcourse, shut down server
/// AFTER you have shut down BG tasks (or past their grace period).
pub(crate) struct ShutdownHandles {
    task_manager: TaskManager,
    axum_handle: AxumHandle,
}

impl ShutdownHandles {
    /// Cancellation token is given to Axum server constructor. When it receives
    /// a shutdown signal, it will shut down Axum server gracefully.
    pub(crate) fn new(task_manager: TaskManager) -> (Self, WaitForCancellationFutureOwned) {
        //
        let token = CancellationToken::new();
        (
            Self {
                task_manager,
                axum_handle: AxumHandle(token.clone()),
            },
            token.cancelled_owned(),
        )
    }

    pub(crate) fn task_manager(&self) -> &TaskManager {
        &self.task_manager
    }

    pub(crate) fn task_manager_mut(&mut self) -> &mut TaskManager {
        &mut self.task_manager
    }

    /// After background tasks have finished, signal server to shut down.
    pub(crate) fn shutdown_axum(self) {
        self.axum_handle.0.cancel()
    }
}

struct AxumHandle(CancellationToken);

#[derive(Clone)]
// TODO dz remove smurf name after eliminating rocket
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

    let coconut_keypair_wrapper = ecash::keys::KeyPair::new();

    // if the keypair doesnt exist (because say this API is running in the caching mode), nothing will happen
    if let Some(loaded_keys) = load_ecash_keypair_if_exists(&config.coconut_signer)? {
        let issued_for = loaded_keys.issued_for_epoch;
        coconut_keypair_wrapper.set(loaded_keys).await;

        if can_validate_coconut_keys(&nyxd_client, issued_for).await? {
            coconut_keypair_wrapper.validate()
        }
    }

    let identity_keypair = config.base.storage_paths.load_identity()?;
    let identity_public_key = *identity_keypair.public_key();

    let router = setup_routes(config.network_monitor.enabled).await?;

    let nym_contract_cache_state = NymContractCache::new();
    let node_status_cache_state = NodeStatusCache::new();
    let mix_denom = network_details.network.chain_details.mix_denom.base.clone();
    let circulating_supply_cache = CirculatingSupplyCache::new(mix_denom.to_owned());
    let described_nodes_state = SharedCache::<DescribedNodes>::new();
    // TODO dz: below mentioned issue is closed, is there a nicer approach?
    // This is not a very nice approach. A lazy value would be more suitable, but that's still
    // a nightly feature: https://github.com/rust-lang/rust/issues/74465
    let storage =
        storage::NymApiStorage::init(&config.node_status_api.storage_paths.database_path).await?;
    let node_info_cache = unstable::NodeInfoCache::default();

    let mut status_state = ApiStatusState::new();

    // if coconut signer is enabled, add /coconut to server
    let router = if config.coconut_signer.enabled {
        // TODO dz it's ecash now, refactor this
        todo!()
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

    // setup shutdowns
    let (shutdown, axum_receiver) = ShutdownHandles::new(TaskManager::new(10));

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
    .start(
        shutdown
            .task_manager()
            .subscribe_named("node-self-described-data-refresher"),
    );

    // start all the caches first
    let nym_contract_cache_listener = nym_contract_cache::start_refresher(
        &config.node_status_api,
        &nym_contract_cache_state,
        nyxd_client.clone(),
        shutdown.task_manager(),
    );
    node_status_api::start_cache_refresh(
        &config.node_status_api,
        &nym_contract_cache_state,
        &node_status_cache_state,
        storage.clone(),
        nym_contract_cache_listener,
        shutdown.task_manager(),
    );
    circulating_supply_api::start_cache_refresh(
        &config.circulating_supply_cacher,
        nyxd_client.clone(),
        &circulating_supply_cache,
        shutdown.task_manager(),
    );

    // start dkg task
    if config.coconut_signer.enabled {
        let dkg_bte_keypair = load_bte_keypair(&config.coconut_signer)?;

        DkgController::start(
            &config.coconut_signer,
            nyxd_client.clone(),
            coconut_keypair_wrapper,
            dkg_bte_keypair,
            identity_public_key,
            rand::rngs::OsRng,
            shutdown.task_manager(),
        )?;
    }

    // and then only start the uptime updater (and the monitor itself, duh)
    // if the monitoring is enabled
    if config.network_monitor.enabled {
        // if network monitor is enabled, the storage MUST BE available
        let storage = storage;

        network_monitor::start::<SphinxMessageReceiver>(
            &config.network_monitor,
            &nym_contract_cache_state,
            &storage,
            nyxd_client.clone(),
            shutdown.task_manager(),
        )
        .await;

        HistoricalUptimeUpdater::start(storage.to_owned(), shutdown.task_manager());

        // start 'rewarding' if its enabled
        if config.rewarding.enabled {
            epoch_operations::ensure_rewarding_permission(&nyxd_client).await?;
            RewardedSetUpdater::start(
                nyxd_client,
                &nym_contract_cache_state,
                storage,
                shutdown.task_manager(),
            );
        }
    }

    // TODO dz currently read from rocket.toml
    let ip = std::net::IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
    let port = 8081u16;
    let bind_address = SocketAddr::new(ip, port);
    let server = ApiHttpServer::build(&bind_address, router).await?;

    tokio::spawn(async move {
        {
            info!("Started Axum HTTP V2 server on {bind_address}");
            server.run(axum_receiver).await
        }
    });

    Ok(shutdown)
}
