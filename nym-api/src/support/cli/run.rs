// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::client::Client;
use crate::ecash::comm::{CommunicationChannelConfig, QueryCommunicationChannel};
use crate::ecash::dkg::controller::keys::{
    can_validate_ecash_keys, load_archived_ecash_keypairs, load_bte_keypair,
    load_ecash_keypair_if_exists,
};
use crate::ecash::dkg::controller::DkgController;
use crate::ecash::state::EcashState;
use crate::epoch_operations::{EpochAdvancer, PerformanceSource};
use crate::key_rotation::KeyRotationController;
use crate::mixnet_contract_cache::cache::MixnetContractCache;
use crate::network::models::NetworkDetailsV2 as NetworkDetails;
use crate::node_describe_cache::cache::DescribedNodes;
use crate::node_families::cache::NodeFamiliesCacheData;
use crate::node_performance::contract_window::refresher::PerformanceContractWindowRefresher;
use crate::node_performance::contract_window::PerformanceContractWindow;
use crate::node_performance::provider::legacy_storage_provider::LegacyStoragePerformanceProvider;
use crate::node_status_api::handlers::unstable;
use crate::node_status_api::uptime_updater::HistoricalUptimeUpdater;
use crate::node_status_api::NodeStatusCache;
use crate::status::{ApiStatusState, SignerState};
use crate::support::attested_snapshots::state::AttestedSnapshotsState;
use crate::support::caching::cache::SharedCache;
use crate::support::caching::startup::StartupCaches;
use crate::support::chain_anchor::SharedChainAnchor;
use crate::support::config::helpers::try_load_current_config;
use crate::support::config::{Config, DEFAULT_CHAIN_STATUS_CACHE_TTL};
use crate::support::http::state::chain_status::ChainStatusCache;
use crate::support::http::state::contract_details::ContractDetailsCache;
use crate::support::http::state::contract_snapshots::ContractSnapshots;
use crate::support::http::state::force_refresh::ForcedRefresh;
use crate::support::http::state::mixnet_contract_cache::MixnetContractCacheState;
use crate::support::http::state::network_monitors::{LastNMSubmissions, NetworkMonitorsCache};
use crate::support::http::state::node_annotations_cache::NodeAnnotationsCache;
use crate::support::http::state::AppState;
use crate::support::http::{RouterBuilder, TASK_MANAGER_TIMEOUT_S};
use crate::support::nyxd;
use crate::support::storage::runtime_migrations::m001_directory_services_v2_1::migrate_to_directory_services_v2_1;
use crate::support::storage::NymApiStorage;
use crate::unstable_routes::v1::account::cache::AddressInfoCache;
use crate::{
    directory, ecash, epoch_operations, mixnet_contract_cache, network_monitor,
    node_describe_cache, node_families, node_performance, node_status_api, signers_cache,
};
use anyhow::{bail, Context};
use nym_config::defaults::v2::NymNetworkDetails;
use nym_crypto::asymmetric::ed25519;
use nym_sphinx::receiver::SphinxMessageReceiver;
use nym_task::ShutdownManager;
use nym_validator_client::nyxd::Coin;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

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

    /// Bearer token for exposing and accessing additional utility routes
    #[clap(long, env = "NYMAPI_UTILITY_ROUTES_BEARER_ARG")]
    pub(crate) utility_routes_bearer: Option<String>,
}

pub(crate) async fn initialise_storage(config: &Config) -> anyhow::Result<NymApiStorage> {
    let nym_network_details = NymNetworkDetails::new_from_env();
    let nyxd_client = nyxd::Client::new(config, &nym_network_details)?;
    let storage = NymApiStorage::init(&config.node_status_api.storage_paths.database_path).await?;

    // try to perform any needed migrations of the storage
    migrate_to_directory_services_v2_1(&storage, &nyxd_client).await?;
    Ok(storage)
}

async fn start_nym_api_tasks(mut config: Config) -> anyhow::Result<ShutdownManager> {
    let shutdown_manager = ShutdownManager::build_new_default()?
        .with_shutdown_duration(Duration::from_secs(TASK_MANAGER_TIMEOUT_S));

    // ===== SETUP: the chain client, storage, identity and ecash
    let nym_network_details = NymNetworkDetails::new_from_env();
    let nyxd_client = nyxd::Client::new(&config, &nym_network_details)?;
    let connected_nyxd = config.get_nyxd_url();
    let network_details = NetworkDetails::new(connected_nyxd.to_string(), nym_network_details);

    let ecash_keypair = load_ecash_keys(&config, &nyxd_client).await?;

    let storage = initialise_storage(&config).await?;
    let identity_keypair = Arc::new(config.base.storage_paths.load_identity()?);

    let mut ecash_state = build_ecash_state(
        &config,
        &nyxd_client,
        &identity_keypair,
        &ecash_keypair,
        &storage,
        &shutdown_manager,
    )
    .await?;
    let signer_information = load_signer_information(
        &config,
        &nyxd_client,
        &network_details,
        &identity_keypair,
        &ecash_keypair,
    )
    .await?;

    // ===== STARTUP CACHES: each loaded from its persisted file while fresh and refreshed in the
    // background, beside the attested producers' startup production
    let storage_cfg = &config.base.storage_paths;

    // MIXNET CONTRACT
    let mixnet_path = storage_cfg.cache_file("mixnet_contract");
    let ttl = config.mixnet_contract_cache.debug.caching_interval;
    let mixnet_contract_cache_state = MixnetContractCache::new(&mixnet_path, ttl);
    let mixnet_contract_cache_refresher = mixnet_contract_cache::build_refresher(
        &config.mixnet_contract_cache,
        &mixnet_contract_cache_state,
        nyxd_client.clone(),
        mixnet_path,
    );
    let mixnet_contract_cache_refresh_requester =
        mixnet_contract_cache_refresher.refresh_requester();
    let contract_cache_watcher =
        mixnet_contract_cache_refresher.start_with_watcher(shutdown_manager.clone_shutdown_token());

    // DESCRIBED NODES
    let described_path = storage_cfg.cache_file("described_nodes");
    let ttl = config.describe_cache.debug.caching_interval;
    let described_nodes_cache =
        SharedCache::<DescribedNodes>::new_with_persistent(&described_path, ttl);
    let describe_cache_refresher = node_describe_cache::provider::new_provider_with_initial_value(
        &config.describe_cache,
        mixnet_contract_cache_state.clone(),
        described_nodes_cache.clone(),
        described_path,
    );
    let describe_cache_refresh_requester = describe_cache_refresher.refresh_requester();
    let describe_cache_watcher =
        describe_cache_refresher.start_with_watcher(shutdown_manager.clone_shutdown_token());

    // NODE FAMILIES
    let node_families_path = storage_cfg.cache_file("node_families");
    let ttl = config.node_families_cache.debug.caching_interval;
    let node_families_cache =
        SharedCache::<NodeFamiliesCacheData>::new_with_persistent(&node_families_path, ttl);
    node_families::build_refresher(
        &config.node_families_cache,
        &mixnet_contract_cache_state,
        &node_families_cache,
        nyxd_client.clone(),
        node_families_path,
    )
    .start(shutdown_manager.clone_shutdown_token());

    // NODE ANNOTATIONS: its refresher starts below, after the performance snapshots. each mode
    // persists its own file, so that neither loads, or carries over, what the other built
    let annotations_file = if config.performance_provider.use_performance_contract_data {
        "contract_node_annotations"
    } else {
        "node_annotations"
    };
    let annotations_path = storage_cfg.cache_file(annotations_file);
    let ttl = config.node_status_api.debug.caching_interval;
    let node_status_cache_state = NodeStatusCache::new(&annotations_path, ttl);

    // CHAIN ANCHOR
    // the chain trust every attested contract's snapshots are verified under
    let checkpoint_path = storage_cfg.cache_file("nyx_checkpoint.json");
    let chain_anchor = SharedChainAnchor::new(
        &config.directory.debug,
        nyxd_client.query_client(),
        &checkpoint_path,
    )
    .await?;

    // PERFORMANCE SNAPSHOTS
    let performance_snapshots_path = storage_cfg.cache_file("performance_snapshots");
    let performance_snapshots_cache = node_performance::attestation::start_cache_refresher(
        config.directory.debug,
        identity_keypair.clone(),
        nyxd_client.clone(),
        &chain_anchor,
        performance_snapshots_path,
        &shutdown_manager,
    )
    .await?;
    let performance_snapshots = AttestedSnapshotsState::new(
        performance_snapshots_cache,
        config.directory.debug.settle_lag,
    );

    // NODE ANNOTATIONS' REFRESHER
    // after the performance snapshots, which contract mode builds them from, and before the
    // directory's startup production, so that their first refresh runs beside it
    let performance_provider = LegacyStoragePerformanceProvider::new(
        storage.clone(),
        mixnet_contract_cache_state.clone(),
        config.performance_provider.debug.stress_testing_data_period,
        config.performance_provider.debug.liveness_data_period,
    );
    let node_status_cache_refresh_requester = node_status_api::start_cache_refresh(
        &config,
        &nyxd_client,
        &mixnet_contract_cache_state,
        &described_nodes_cache,
        &performance_snapshots,
        &node_status_cache_state,
        performance_provider,
        contract_cache_watcher.clone(),
        describe_cache_watcher,
        annotations_path,
        &shutdown_manager,
    )
    .await;

    // DIRECTORY SNAPSHOTS
    let directory_path = storage_cfg.cache_file("directory");
    let directory_cache = directory::attestation::start_cache_refresher(
        config.directory,
        identity_keypair.clone(),
        nyxd_client.clone(),
        &chain_anchor,
        directory_path,
        &shutdown_manager,
    )
    .await?;
    let directory = AttestedSnapshotsState::new(directory_cache, config.directory.debug.settle_lag);

    // ===== WAIT: nothing else is spawned and nothing is served until the startup caches hold
    // values, so that no route, reward or selection reads one that was never built
    let startup_caches = StartupCaches {
        mixnet_contract: mixnet_contract_cache_state.clone(),
        described_nodes: described_nodes_cache.clone(),
        node_families: node_families_cache.clone(),
        node_annotations: node_status_cache_state.clone(),
    };
    if startup_caches
        .wait_until_built(&shutdown_manager.clone_shutdown_token())
        .await
        .is_err()
    {
        // shutdown was signalled, or a task panicked, so there is nothing more to start
        return Ok(shutdown_manager);
    }

    // ===== TASKS AND HTTP
    // held only where rewarding runs: contract-mode rewarding and selection and, with the switch
    // off, the contract-divergence route are its only readers; loaded, in contract mode, before
    // the epoch operations start
    let performance_contract_window = if config.rewarding.enabled {
        Some(
            start_performance_contract_window(
                &config,
                &nyxd_client,
                &mixnet_contract_cache_state,
                &shutdown_manager,
            )
            .await?,
        )
    } else {
        None
    };

    // start dkg task
    if config.ecash_signer.enabled {
        let dkg_bte_keypair = load_bte_keypair(&config.ecash_signer)?;

        DkgController::start(
            &config.ecash_signer,
            nyxd_client.clone(),
            ecash_keypair,
            dkg_bte_keypair,
            *identity_keypair.public_key(),
            nym_crypto::rng::os_rng(),
            &shutdown_manager,
        )?;
    }

    ecash_state.spawn_background_cleaner();

    // the network monitor and its uptime updater, where monitoring is enabled
    if config.network_monitor.enabled {
        network_monitor::start::<SphinxMessageReceiver>(
            &config,
            &mixnet_contract_cache_state,
            described_nodes_cache.clone(),
            node_status_cache_state.clone(),
            &storage,
            nyxd_client.clone(),
            &shutdown_manager,
        )
        .await;

        HistoricalUptimeUpdater::start(storage.to_owned(), &shutdown_manager);
    }

    // rewarding, where enabled and there is a source of performance data
    let has_performance_data =
        config.network_monitor.enabled || config.performance_provider.use_performance_contract_data;
    if config.rewarding.enabled && has_performance_data {
        epoch_operations::ensure_rewarding_permission(&nyxd_client).await?;
        let performance_source = match &performance_contract_window {
            Some(window) if config.performance_provider.use_performance_contract_data => {
                PerformanceSource::ContractWindow(window.clone())
            }
            _ => PerformanceSource::Annotations(node_status_cache_state.clone()),
        };
        EpochAdvancer::start(
            nyxd_client.clone(),
            &mixnet_contract_cache_state,
            mixnet_contract_cache_refresh_requester.clone(),
            performance_source,
            described_nodes_cache.clone(),
            &storage,
            &shutdown_manager,
        );
    }

    // a background task watching the contract changes and requesting self-described cache
    // refresh upon being close to key rotation rollover
    KeyRotationController::new(
        describe_cache_refresh_requester,
        contract_cache_watcher,
        mixnet_contract_cache_state.clone(),
    )
    .start(shutdown_manager.clone_shutdown_token());

    // it probes this nym-api's own HTTP after its start delay, which therefore runs from here
    let ecash_signers_cache = if config.signers_cache.enabled {
        signers_cache::start_refresher(
            &config.signers_cache,
            nyxd_client.clone(),
            &shutdown_manager,
        )
    } else {
        SharedCache::new()
    };

    let mixnet_contract_cache = MixnetContractCacheState::new(
        mixnet_contract_cache_state,
        mixnet_contract_cache_refresh_requester,
    );
    let node_annotations_cache =
        NodeAnnotationsCache::new(node_status_cache_state, node_status_cache_refresh_requester);

    let router = RouterBuilder::with_default_routes(
        config.network_monitor.enabled,
        config.base.utility_routes_bearer.take(),
    )
    .with_state(AppState {
        identity_keypair,
        nyxd_client,
        chain_status_cache: ChainStatusCache::new(DEFAULT_CHAIN_STATUS_CACHE_TTL),
        ecash_signers_cache,
        address_info_cache: AddressInfoCache::new(
            config.address_cache.time_to_live,
            config.address_cache.capacity,
        ),
        forced_refresh: ForcedRefresh::new(config.describe_cache.debug.allow_illegal_ips),
        network_monitor_submissions: LastNMSubmissions::new(),
        mixnet_contract_cache,
        network_monitors_cache: NetworkMonitorsCache::new(
            config.network_monitors_cache.time_to_live,
        ),
        node_families_cache,
        node_annotations_cache,
        storage,
        described_nodes_cache,
        network_details,
        // filled on demand from the database, for the unstable test-run routes
        node_info_cache: unstable::NodeInfoCache::default(),
        contract_info_cache: ContractDetailsCache::new(config.contracts_info_cache.time_to_live),
        api_status: ApiStatusState::new(signer_information),
        ecash_state: Arc::new(ecash_state),
        contract_snapshots: ContractSnapshots {
            directory,
            performance: performance_snapshots,
        },
        performance_contract_window,
        use_performance_contract_data: config.performance_provider.use_performance_contract_data,
    });

    let bind_address = config.base.bind_address.to_owned();
    let server = router.build_server(&bind_address).await?;

    let http_shutdown = shutdown_manager.clone_shutdown_token();
    tokio::spawn(async move {
        {
            info!("Started Axum HTTP V2 server on {bind_address}");
            server.run(http_shutdown).await
        }
    });

    shutdown_manager.close_tracker();

    Ok(shutdown_manager)
}

/// The ecash keys this nym-api holds: its current one if any, and a signer's archived ones.
async fn load_ecash_keys(
    config: &Config,
    nyxd_client: &nyxd::Client,
) -> anyhow::Result<ecash::keys::KeyPair> {
    let ecash_keypair = ecash::keys::KeyPair::new();

    // if the keypair doesn't exist (because say this API is running in the caching mode), nothing will happen
    if let Some(loaded_keys) = load_ecash_keypair_if_exists(&config.ecash_signer)? {
        let issued_for = loaded_keys.issued_for_epoch;
        ecash_keypair.set(loaded_keys).await;

        if can_validate_ecash_keys(nyxd_client, issued_for).await? {
            ecash_keypair.validate()
        }
    }

    // credentials issued under earlier epochs remain spendable, and their auxiliary signatures
    // can only be produced with the keys of the epoch that issued them.
    //
    // this is deliberately not tied to holding a *current* key: an api that restarted between
    // archiving its old one and deriving its replacement holds nothing else.
    if config.ecash_signer.enabled {
        for archived in
            load_archived_ecash_keypairs(&config.ecash_signer.storage_paths.ecash_key_path)
        {
            ecash_keypair.archive(archived).await;
        }
    }

    Ok(ecash_keypair)
}

/// The ecash state, whose background cleaner is started once the startup caches are built.
async fn build_ecash_state(
    config: &Config,
    nyxd_client: &nyxd::Client,
    identity_keypair: &Arc<ed25519::KeyPair>,
    ecash_keypair: &ecash::keys::KeyPair,
    storage: &NymApiStorage,
    shutdown_manager: &ShutdownManager,
) -> anyhow::Result<EcashState> {
    let ecash_contract = nyxd_client
        .get_ecash_contract_address()
        .await
        .context("e-cash contract address is required to setup the nym-api routes")?;

    let comm_channel = QueryCommunicationChannel::new(
        nyxd_client.clone(),
        CommunicationChannelConfig::new(config),
    );

    Ok(EcashState::new(
        config,
        ecash_contract,
        nyxd_client.clone(),
        identity_keypair.clone(),
        ecash_keypair.clone(),
        comm_channel,
        storage.clone(),
        shutdown_manager,
    ))
}

/// This nym-api's signer information when it is an ecash signer, which also requires its account
/// to hold enough to cover the multisig fees.
async fn load_signer_information(
    config: &Config,
    nyxd_client: &nyxd::Client,
    network_details: &NetworkDetails,
    identity_keypair: &ed25519::KeyPair,
    ecash_keypair: &ecash::keys::KeyPair,
) -> anyhow::Result<Option<SignerState>> {
    if !config.ecash_signer.enabled {
        return Ok(None);
    }

    let cosmos_address = nyxd_client.address().await?;

    // make sure we have some tokens to cover multisig fees
    let mix_denom = network_details.network.chain_details.mix_denom.base.clone();
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
    Ok(Some(SignerState {
        cosmos_address: cosmos_address.to_string(),
        identity: identity_keypair.public_key().to_base58_string(),
        announce_address,
        ecash_keypair: ecash_keypair.clone(),
    }))
}

/// Starts the window over the performance contract, loaded up front in contract mode.
async fn start_performance_contract_window(
    config: &Config,
    nyxd_client: &nyxd::Client,
    mixnet_contract_cache: &MixnetContractCache,
    shutdown_manager: &ShutdownManager,
) -> anyhow::Result<PerformanceContractWindow> {
    let window = PerformanceContractWindow::default();
    let refresher = PerformanceContractWindowRefresher::new(
        window.clone(),
        nyxd_client.query_client(),
        mixnet_contract_cache.clone(),
        config.performance_provider.debug.contract_polling_interval,
    );
    if config.performance_provider.use_performance_contract_data {
        refresher
            .load_initial_values()
            .await
            .context("failed to load the performance contract window")?;
    }
    refresher.start(shutdown_manager.clone_shutdown_token());

    Ok(window)
}

pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    // args take precedence over env
    let mut config = try_load_current_config(&args.id)?
        .override_with_env()
        .override_with_args(args);

    config.validate_and_fixup()?;

    let mut shutdown_manager = start_nym_api_tasks(config).await?;
    shutdown_manager.run_until_shutdown().await;

    Ok(())
}
