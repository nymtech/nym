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
use crate::epoch_operations::EpochAdvancer;
use crate::key_rotation::KeyRotationController;
use crate::mixnet_contract_cache::cache::MixnetContractCache;
use crate::network::models::NetworkDetailsV2 as NetworkDetails;
use crate::node_describe_cache::cache::DescribedNodes;
use crate::node_families::cache::NodeFamiliesCacheData;
use crate::node_performance::contract_window::refresher::{
    PerformanceContractWindowRefresher, DEFAULT_WINDOW_POLLING_INTERVAL,
};
use crate::node_performance::contract_window::PerformanceContractWindow;
use crate::node_status_api::handlers::unstable;
use crate::status::{ApiStatusState, SignerState};
use crate::support::attested_snapshots;
use crate::support::caching::cache::SharedCache;
use crate::support::caching::startup::StartupCaches;
use crate::support::config::helpers::try_load_current_config;
use crate::support::config::{Config, DEFAULT_CHAIN_STATUS_CACHE_TTL};
use crate::support::http::state::chain_status::ChainStatusCache;
use crate::support::http::state::contract_details::ContractDetailsCache;
use crate::support::http::state::force_refresh::ForcedRefresh;
use crate::support::http::state::mixnet_contract_cache::MixnetContractCacheState;
use crate::support::http::state::AppState;
use crate::support::http::{RouterBuilder, TASK_MANAGER_TIMEOUT_S};
use crate::support::nyxd;
use crate::support::storage::runtime_migrations::m001_directory_services_v2_1::migrate_to_directory_services_v2_1;
use crate::support::storage::NymApiStorage;
use crate::unstable_routes::v1::account::cache::AddressInfoCache;
use crate::{
    ecash, epoch_operations, mixnet_contract_cache, node_describe_cache, node_families,
    signers_cache,
};
use anyhow::{bail, Context};
use nym_config::defaults::v2::NymNetworkDetails;
use nym_crypto::asymmetric::ed25519;
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

    /// Specifies whether network rewarding is enabled on this API
    /// default: None - config value will be used instead
    #[clap(
        short = 'r',
        long,
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
    // background, beside the attested snapshots' startup production
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
    describe_cache_refresher.start(shutdown_manager.clone_shutdown_token());

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

    // ATTESTED SNAPSHOTS
    // every attested contract's, produced together under one chain anchor; the routes read each
    // node's published performance from the latest settled one
    let contract_snapshots = attested_snapshots::start(
        &config.directory.debug,
        nyxd_client.query_client(),
        identity_keypair.clone(),
        &storage_cfg.cache_file("nyx_checkpoint.json"),
        storage_cfg.cache_file("attested_snapshots"),
        &shutdown_manager,
    )
    .await?;

    // ===== WAIT: nothing else is spawned and nothing is served until the startup caches hold
    // values and a performance snapshot has settled, so that no route, reward or selection reads
    // one that was never built
    let startup_caches = StartupCaches {
        mixnet_contract: mixnet_contract_cache_state.clone(),
        described_nodes: described_nodes_cache.clone(),
        node_families: node_families_cache.clone(),
        contract_snapshots: contract_snapshots.clone(),
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
    // held only where rewarding runs, rewarding and selection being its readers; loaded before
    // the epoch operations start
    let performance_contract_window = if config.rewarding.enabled {
        Some(
            start_performance_contract_window(
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

    // rewarding, where enabled, which is exactly where the window is held
    if let Some(window) = &performance_contract_window {
        epoch_operations::ensure_rewarding_permission(&nyxd_client).await?;
        EpochAdvancer::start(
            nyxd_client.clone(),
            &mixnet_contract_cache_state,
            mixnet_contract_cache_refresh_requester.clone(),
            window.clone(),
            described_nodes_cache.clone(),
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

    let router = RouterBuilder::with_default_routes(config.base.utility_routes_bearer.take())
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
            mixnet_contract_cache,
            node_families_cache,
            storage,
            described_nodes_cache,
            network_details,
            // filled on demand from the database, for the unstable test-run routes
            node_info_cache: unstable::NodeInfoCache::default(),
            contract_info_cache: ContractDetailsCache::new(
                config.contracts_info_cache.time_to_live,
            ),
            api_status: ApiStatusState::new(signer_information),
            ecash_state: Arc::new(ecash_state),
            contract_snapshots,
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

/// Starts the window over the performance contract, loaded up front.
async fn start_performance_contract_window(
    nyxd_client: &nyxd::Client,
    mixnet_contract_cache: &MixnetContractCache,
    shutdown_manager: &ShutdownManager,
) -> anyhow::Result<PerformanceContractWindow> {
    let window = PerformanceContractWindow::default();
    let refresher = PerformanceContractWindowRefresher::new(
        window.clone(),
        nyxd_client.query_client(),
        mixnet_contract_cache.clone(),
        DEFAULT_WINDOW_POLLING_INTERVAL,
    );
    refresher
        .load_initial_values()
        .await
        .context("failed to load the performance contract window")?;
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
