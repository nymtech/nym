// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::client::Client;
use crate::ecash::comm::QueryCommunicationChannel;
use crate::ecash::dkg::controller::keys::{
    can_validate_coconut_keys, load_bte_keypair, load_ecash_keypair_if_exists,
};
use crate::ecash::dkg::controller::DkgController;
use crate::ecash::state::EcashState;
use crate::epoch_operations::EpochAdvancer;
use crate::key_rotation::KeyRotationController;
use crate::mixnet_contract_cache::cache::MixnetContractCache;
use crate::network::models::NetworkDetails;
use crate::node_describe_cache::cache::DescribedNodes;
use crate::node_performance::provider::contract_provider::ContractPerformanceProvider;
use crate::node_performance::provider::legacy_storage_provider::LegacyStoragePerformanceProvider;
use crate::node_performance::provider::NodePerformanceProvider;
use crate::node_status_api::handlers::unstable;
use crate::node_status_api::uptime_updater::HistoricalUptimeUpdater;
use crate::node_status_api::NodeStatusCache;
use crate::status::{ApiStatusState, SignerState};
use crate::support::caching::cache::SharedCache;
use crate::support::config::helpers::try_load_current_config;
use crate::support::config::{Config, DEFAULT_CHAIN_STATUS_CACHE_TTL};
use crate::support::http::state::chain_status::ChainStatusCache;
use crate::support::http::state::contract_details::ContractDetailsCache;
use crate::support::http::state::force_refresh::ForcedRefresh;
use crate::support::http::state::AppState;
use crate::support::http::{RouterBuilder, ShutdownHandles, TASK_MANAGER_TIMEOUT_S};
use crate::support::nyxd;
use crate::support::storage::runtime_migrations::m001_directory_services_v2_1::migrate_to_directory_services_v2_1;
use crate::support::storage::NymApiStorage;
use crate::unstable_routes::v1::account::cache::AddressInfoCache;
use crate::{
    ecash, epoch_operations, mixnet_contract_cache, network_monitor, node_describe_cache,
    node_performance, node_status_api,
};
use anyhow::{bail, Context};
use nym_config::defaults::NymNetworkDetails;
use nym_sphinx::receiver::SphinxMessageReceiver;
use nym_task::TaskManager;
use nym_validator_client::nyxd::Coin;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    /// Id of the nym-api we want to run.if unspecified, a default value will be used.
    /// default: "default"
    #[clap(long, default_value = "default", env = "NYMAPI_ID_ARG")]
    pub(crate) id: String,

    /// Specifies whether network monitoring is enabled on this API
    /// default: None - config value will be used instead
    #[clap(short = 'm', long, env = "NYMAPI_ENABLE_MONITOR_ARG")]
    pub(crate) enable_monitor: Option<bool>,

    /// Specifies whether network rewarding is enabled on this API
    /// default: None - config value will be used instead
    #[clap(
        short = 'r',
        long,
        requires = "enable_monitor",
        requires = "mnemonic",
        env = "NYMAPI_ENABLE_REWARDING_ARG"
    )]
    pub(crate) enable_rewarding: Option<bool>,

    /// Endpoint to nyxd instance used for contract information.
    /// default: None - config value will be used instead
    #[clap(long, env = "NYMAPI_NYXD_VALIDATOR_ARG")]
    pub(crate) nyxd_validator: Option<url::Url>,

    /// Mnemonic of the network monitor used for sending rewarding and zk-nyms transactions
    /// default: None - config value will be used instead
    #[clap(long, env = "NYMAPI_MNEMONIC_ARG")]
    pub(crate) mnemonic: Option<bip39::Mnemonic>,

    /// Flag to indicate whether coconut signer authority is enabled on this API
    /// default: None - config value will be used instead
    #[clap(
        long,
        requires = "mnemonic",
        requires = "announce_address",
        alias = "enable_coconut",
        env = "NYMAPI_ENABLE_ZK_NYM_ARG"
    )]
    pub(crate) enable_zk_nym: Option<bool>,

    /// Announced address that is going to be put in the DKG contract where zk-nym clients will connect
    /// to obtain their credentials
    /// default: None - config value will be used instead
    #[clap(long, env = "NYMAPI_ANNOUNCE_ADDRESS_ARG")]
    pub(crate) announce_address: Option<url::Url>,

    /// Set this nym api to work in a enabled credentials that would attempt to use gateway with the bandwidth credential requirement
    /// default: None - config value will be used instead
    #[clap(long, env = "NYMAPI_MONITOR_CREDENTIALS_MODE_ARG")]
    pub(crate) monitor_credentials_mode: Option<bool>,

    /// Socket address this api will use for binding its http API.
    /// default: `127.0.0.1:8080` in `debug` builds and `0.0.0.0:8080` in `release`
    #[clap(long)]
    pub(crate) bind_address: Option<SocketAddr>,

    /// account/address cache TTL: should be lower than epoch length (1 hour)
    /// because, at worst, data will be stale for <epoch_length> + <cache_ttl> seconds
    #[clap(long, env = "ADDRESS_CACHE_REFRESH_INTERVAL_S")]
    pub(crate) address_cache_ttl_seconds: Option<u64>,

    /// number of addresses that are cached on account/address endpoint
    #[clap(long, env = "ADDRESS_CACHE_CAPACITY")]
    pub(crate) address_cache_capacity: Option<u64>,

    #[clap(hide = true, long, default_value_t = false)]
    pub(crate) allow_illegal_ips: bool,
}

async fn start_nym_api_tasks(config: &Config) -> anyhow::Result<ShutdownHandles> {
    let task_manager = TaskManager::new(TASK_MANAGER_TIMEOUT_S);

    let nyxd_client = nyxd::Client::new(config)?;
    let connected_nyxd = config.get_nyxd_url();
    let nym_network_details = NymNetworkDetails::new_from_env();
    let network_details = NetworkDetails::new(connected_nyxd.to_string(), nym_network_details);

    let ecash_keypair_wrapper = ecash::keys::KeyPair::new();

    // if the keypair doesnt exist (because say this API is running in the caching mode), nothing will happen
    if let Some(loaded_keys) = load_ecash_keypair_if_exists(&config.ecash_signer)? {
        let issued_for = loaded_keys.issued_for_epoch;
        ecash_keypair_wrapper.set(loaded_keys).await;

        if can_validate_coconut_keys(&nyxd_client, issued_for).await? {
            ecash_keypair_wrapper.validate()
        }
    }

    let storage = NymApiStorage::init(&config.node_status_api.storage_paths.database_path).await?;

    // try to perform any needed migrations of the storage
    migrate_to_directory_services_v2_1(&storage, &nyxd_client).await?;

    let identity_keypair = config.base.storage_paths.load_identity()?;
    let identity_public_key = *identity_keypair.public_key();

    let router = RouterBuilder::with_default_routes(config.network_monitor.enabled);

    let mixnet_contract_cache_state = MixnetContractCache::new();
    let node_status_cache_state = NodeStatusCache::new();
    let mix_denom = network_details.network.chain_details.mix_denom.base.clone();
    let described_nodes_cache = SharedCache::<DescribedNodes>::new();
    let node_info_cache = unstable::NodeInfoCache::default();

    let ecash_contract = nyxd_client
        .get_ecash_contract_address()
        .await
        .context("e-cash contract address is required to setup the nym-api routes")?;

    let comm_channel = QueryCommunicationChannel::new(nyxd_client.clone());

    let encoded_identity = identity_keypair.public_key().to_base58_string();
    let mut ecash_state = EcashState::new(
        config,
        ecash_contract,
        nyxd_client.clone(),
        identity_keypair,
        ecash_keypair_wrapper.clone(),
        comm_channel,
        storage.clone(),
        task_manager.subscribe_named("ecash-state-data-cleaner"),
    );

    // if ecash signer is enabled, there are additional constraints on the nym-api,
    // such as having sufficient token balance
    let signer_information = if config.ecash_signer.enabled {
        let cosmos_address = nyxd_client.address().await?;

        // make sure we have some tokens to cover multisig fees
        let balance = nyxd_client.balance(&mix_denom).await?;
        if balance.amount < ecash::MINIMUM_BALANCE {
            let min = Coin::new(ecash::MINIMUM_BALANCE, mix_denom);
            bail!("the account ({cosmos_address}) doesn't have enough funds to cover verification fees. it has {balance} while it needs at least {min}")
        }

        let announce_address = config
            .ecash_signer
            .announce_address
            .clone()
            .map(|u| u.to_string())
            .unwrap_or_default();
        Some(SignerState {
            cosmos_address: cosmos_address.to_string(),
            identity: encoded_identity,
            announce_address,
            ecash_keypair: ecash_keypair_wrapper.clone(),
        })
    } else {
        None
    };

    ecash_state.spawn_background_cleaner();
    let router = router.with_state(AppState {
        nyxd_client: nyxd_client.clone(),
        chain_status_cache: ChainStatusCache::new(DEFAULT_CHAIN_STATUS_CACHE_TTL),
        address_info_cache: AddressInfoCache::new(
            config.address_cache.time_to_live,
            config.address_cache.capacity,
        ),
        forced_refresh: ForcedRefresh::new(config.describe_cache.debug.allow_illegal_ips),
        mixnet_contract_cache: mixnet_contract_cache_state.clone(),
        node_status_cache: node_status_cache_state.clone(),
        storage: storage.clone(),
        described_nodes_cache: described_nodes_cache.clone(),
        network_details: network_details.clone(),
        node_info_cache,
        contract_info_cache: ContractDetailsCache::new(config.contracts_info_cache.time_to_live),
        api_status: ApiStatusState::new(signer_information),
        ecash_state: Arc::new(ecash_state),
    });

    // start note describe cache refresher
    // we should be doing the below, but can't due to our current startup structure
    // let refresher = node_describe_cache::new_refresher(&config.topology_cacher);
    // let cache = refresher.get_shared_cache();
    let describe_cache_refresher = node_describe_cache::provider::new_provider_with_initial_value(
        &config.describe_cache,
        mixnet_contract_cache_state.clone(),
        described_nodes_cache.clone(),
    )
    .named("node-self-described-data-refresher");

    let describe_cache_refresh_requester = describe_cache_refresher.refresh_requester();

    let describe_cache_watcher = describe_cache_refresher
        .start_with_watcher(task_manager.subscribe_named("node-self-described-data-refresher"));

    let performance_provider = if config.performance_provider.use_performance_contract_data {
        if network_details
            .network
            .contracts
            .performance_contract_address
            .is_none()
        {
            bail!("can't use performance contract data without setting the address of the contract")
        }

        let performance_contract_cache = node_performance::contract_cache::start_cache_refresher(
            &config.performance_provider,
            nyxd_client.clone(),
            mixnet_contract_cache_state.clone(),
            &task_manager,
        )
        .await?;
        let provider = ContractPerformanceProvider::new(
            &config.performance_provider,
            performance_contract_cache,
        );
        Box::new(provider) as Box<dyn NodePerformanceProvider + Send + Sync>
    } else {
        Box::new(LegacyStoragePerformanceProvider::new(
            storage.clone(),
            mixnet_contract_cache_state.clone(),
        ))
    };

    // start all the caches first
    let mixnet_contract_cache_refresher = mixnet_contract_cache::build_refresher(
        &config.mixnet_contract_cache,
        &mixnet_contract_cache_state.clone(),
        nyxd_client.clone(),
    );
    let contract_cache_watcher =
        mixnet_contract_cache_refresher.start_with_watcher(task_manager.subscribe());

    node_status_api::start_cache_refresh(
        &config.node_status_api,
        &mixnet_contract_cache_state,
        &described_nodes_cache,
        &node_status_cache_state,
        performance_provider,
        contract_cache_watcher.clone(),
        describe_cache_watcher,
        &task_manager,
    );

    // start dkg task
    if config.ecash_signer.enabled {
        let dkg_bte_keypair = load_bte_keypair(&config.ecash_signer)?;

        DkgController::start(
            &config.ecash_signer,
            nyxd_client.clone(),
            ecash_keypair_wrapper,
            dkg_bte_keypair,
            identity_public_key,
            rand::rngs::OsRng,
            &task_manager,
        )?;
    }

    let has_performance_data =
        config.network_monitor.enabled || config.performance_provider.use_performance_contract_data;

    // and then only start the uptime updater (and the monitor itself, duh)
    // if the monitoring is enabled
    if config.network_monitor.enabled {
        network_monitor::start::<SphinxMessageReceiver>(
            config,
            &mixnet_contract_cache_state,
            described_nodes_cache.clone(),
            node_status_cache_state.clone(),
            &storage,
            nyxd_client.clone(),
            &task_manager,
        )
        .await;

        HistoricalUptimeUpdater::start(storage.to_owned(), &task_manager);
    }

    // start 'rewarding' if its enabled and there exists source for performance data
    if config.rewarding.enabled && has_performance_data {
        epoch_operations::ensure_rewarding_permission(&nyxd_client).await?;
        EpochAdvancer::start(
            nyxd_client,
            &mixnet_contract_cache_state,
            &node_status_cache_state,
            described_nodes_cache.clone(),
            &storage,
            &task_manager,
        );
    }

    // finally start a background task watching the contract changes and requesting
    // self-described cache refresh upon being close to key rotation rollover
    KeyRotationController::new(
        describe_cache_refresh_requester,
        contract_cache_watcher,
        mixnet_contract_cache_state,
    )
    .start(task_manager.subscribe_named("KeyRotationController"));

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

pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    // args take precedence over env
    let config = try_load_current_config(&args.id)?
        .override_with_env()
        .override_with_args(args);

    config.validate()?;

    let mut axum_shutdown = start_nym_api_tasks(&config).await?;

    // it doesn't matter which server catches the interrupt: it needs only be caught once
    if let Err(err) = axum_shutdown.task_manager_mut().catch_interrupt().await {
        error!("Error stopping axum tasks: {err}");
    }

    info!("Stopping nym API");

    axum_shutdown.task_manager_mut().signal_shutdown().ok();
    axum_shutdown.task_manager_mut().wait_for_shutdown().await;

    let running_server = axum_shutdown.shutdown_axum();

    match running_server.await {
        Ok(Ok(_)) => {
            info!("Axum HTTP server shut down without errors");
        }
        Ok(Err(err)) => {
            error!("Axum HTTP server terminated with: {err}");
            anyhow::bail!(err)
        }
        Err(err) => {
            error!("Server task panicked: {err}");
        }
    };

    Ok(())
}
