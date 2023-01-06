// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate rocket;

use crate::config::Config;
use crate::contract_cache::ValidatorCacheRefresher;
use crate::epoch_operations::RewardedSetUpdater;
use crate::network_monitor::NetworkMonitorBuilder;
use crate::node_status_api::uptime_updater::HistoricalUptimeUpdater;
use crate::nyxd_client::Client;
use crate::storage::NymApiStorage;
use ::config::defaults::mainnet::read_var_if_not_default;
use ::config::defaults::setup_env;
use ::config::defaults::var_names::{CONFIGURED, MIXNET_CONTRACT_ADDRESS, MIX_DENOM};
use ::config::NymConfig;
use anyhow::Result;
use build_information::BinaryBuildInformation;
use clap::Parser;
use contract_cache::ValidatorCache;
use lazy_static::lazy_static;
use log::{info, warn};
use logging::setup_logging;
use node_status_api::NodeStatusCache;
use okapi::openapi3::OpenApi;
use rocket::fairing::AdHoc;
use rocket::http::Method;
use rocket::{Ignite, Rocket};
use rocket_cors::{AllowedHeaders, AllowedOrigins, Cors};
use rocket_okapi::mount_endpoints_and_merged_docs;
use rocket_okapi::swagger_ui::make_swagger_ui;
use std::sync::Arc;
use std::time::Duration;
use std::{fs, process};
use task::{wait_for_signal, TaskManager};
use tokio::sync::Notify;
use validator_client::nyxd::{self, SigningNyxdClient};

#[cfg(feature = "coconut")]
use coconut::{
    comm::QueryCommunicationChannel,
    dkg::controller::{init_keypair, DkgController},
    InternalSignRequest,
};
#[cfg(feature = "coconut")]
use rand::rngs::OsRng;

pub(crate) mod config;
pub(crate) mod contract_cache;
mod epoch_operations;
mod network_monitor;
mod node_status_api;
pub(crate) mod nyxd_client;
pub(crate) mod storage;
mod swagger;

#[cfg(feature = "coconut")]
mod coconut;

lazy_static! {
    pub static ref PRETTY_BUILD_INFORMATION: String =
        BinaryBuildInformation::new(env!("CARGO_PKG_VERSION")).pretty_print();
}

// Helper for passing LONG_VERSION to clap
fn pretty_build_info_static() -> &'static str {
    &PRETTY_BUILD_INFORMATION
}

// explicitly defined custom parser (as opposed to just using
// #[arg(value_parser = clap::value_parser!(u8).range(0..100))]
// for better error message
fn threshold_in_range(s: &str) -> Result<u8, String> {
    let threshold: usize = s
        .parse()
        .map_err(|_| format!("`{s}` isn't a valid threshold number"))?;
    if threshold > 100 {
        Err(format!("{threshold} is not within the range 0-100"))
    } else {
        Ok(threshold as u8)
    }
}

#[derive(Parser)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
struct ApiArgs {
    /// Path pointing to an env file that configures the Nym API.
    #[clap(short, long)]
    config_env_file: Option<std::path::PathBuf>,

    /// Id of the nym-api we want to run
    #[clap(long)]
    id: Option<String>,

    /// Specifies whether network monitoring is enabled on this API
    #[clap(short = 'm', long)]
    enable_monitor: bool,

    /// Specifies whether network rewarding is enabled on this API
    #[clap(short = 'r', long, requires = "enable_monitor", requires = "mnemonic")]
    enable_rewarding: bool,

    /// Endpoint to nyxd instance from which the monitor will grab nodes to test
    #[clap(long)]
    nyxd_validator: Option<url::Url>,

    /// Address of the mixnet contract managing the network
    #[clap(long)]
    mixnet_contract: Option<nyxd::AccountId>,

    /// Mnemonic of the network monitor used for rewarding operators
    // even though we're currently converting the mnemonic to string (and then back to the concrete type)
    // at least we're getting immediate validation when passing the arguments
    #[clap(long)]
    mnemonic: Option<bip39::Mnemonic>,

    /// Specifies whether a config file based on provided arguments should be saved to a file
    #[clap(short = 'w', long)]
    save_config: bool,

    /// Specifies the minimum percentage of monitor test run data present in order to distribute rewards for given interval.
    #[clap(long, value_parser = threshold_in_range)]
    monitor_threshold: Option<u8>,

    /// Mixnodes with reliability lower the this get blacklisted by network monitor, get no traffic and cannot be selected into a rewarded set.
    #[clap(long, value_parser = threshold_in_range)]
    min_mixnode_reliability: Option<u8>,

    /// Gateways with reliability lower the this get blacklisted by network monitor, get no traffic and cannot be selected into a rewarded set.
    #[clap(long, value_parser = threshold_in_range)]
    min_gateway_reliability: Option<u8>,

    /// Set this nym api to work in a enabled credentials that would attempt to use gateway with the bandwidth credential requirement
    #[clap(long)]
    enabled_credentials_mode: bool,

    /// Announced address where coconut clients will connect.
    #[cfg(feature = "coconut")]
    #[clap(long)]
    announce_address: Option<url::Url>,

    /// Flag to indicate whether coconut signer authority is enabled on this API
    #[cfg(feature = "coconut")]
    #[clap(long, requires = "mnemonic", requires = "announce-address")]
    enable_coconut: bool,
}

async fn wait_for_interrupt(mut shutdown: TaskManager) {
    wait_for_signal().await;

    log::info!("Sending shutdown");
    shutdown.signal_shutdown().ok();

    log::info!("Waiting for tasks to finish... (Press ctrl-c to force)");
    shutdown.wait_for_shutdown().await;

    log::info!("Stopping nym API");
}

fn override_config(mut config: Config, args: ApiArgs) -> Config {
    if let Some(id) = args.id {
        fs::create_dir_all(Config::default_config_directory(Some(&id)))
            .expect("Could not create config directory");
        fs::create_dir_all(Config::default_data_directory(Some(&id)))
            .expect("Could not create data directory");
        config = config.with_id(&id);
    }

    config = config
        .with_network_monitor_enabled(args.enable_monitor)
        .with_rewarding_enabled(args.enable_rewarding)
        .with_disabled_credentials_mode(!args.enabled_credentials_mode);

    if let Some(nyxd_validator) = args.nyxd_validator {
        config = config.with_custom_nyxd_validator(nyxd_validator);
    }

    if let Some(mixnet_contract) = args.mixnet_contract {
        config = config.with_custom_mixnet_contract(mixnet_contract.to_string())
    } else if std::env::var(CONFIGURED).is_ok() {
        if let Some(mixnet_contract) = read_var_if_not_default(MIXNET_CONTRACT_ADDRESS) {
            config = config.with_custom_mixnet_contract(mixnet_contract)
        }
    }
    if let Some(mnemonic) = args.mnemonic {
        config = config.with_mnemonic(mnemonic.to_string())
    }

    if let Some(monitor_threshold) = args.monitor_threshold {
        config = config.with_minimum_interval_monitor_threshold(monitor_threshold)
    }

    if let Some(reliability) = args.min_mixnode_reliability {
        config = config.with_min_mixnode_reliability(reliability)
    }

    if let Some(reliability) = args.min_gateway_reliability {
        config = config.with_min_gateway_reliability(reliability)
    }

    #[cfg(feature = "coconut")]
    if let Some(announce_address) = args.announce_address {
        config = config
            .with_announce_address(announce_address)
            .with_coconut_signer_enabled(args.enable_coconut);
    }

    if args.save_config {
        info!("Saving the configuration to a file");
        if let Err(err) = config.save_to_file(None) {
            error!("Failed to write config to a file - {err}");
            process::exit(1)
        }
    }

    config
}

fn setup_cors() -> Result<Cors> {
    let allowed_origins = AllowedOrigins::all();

    // You can also deserialize this
    let cors = rocket_cors::CorsOptions {
        allowed_origins,
        allowed_methods: vec![Method::Post, Method::Get]
            .into_iter()
            .map(From::from)
            .collect(),
        allowed_headers: AllowedHeaders::all(),
        allow_credentials: true,
        ..Default::default()
    }
    .to_cors()?;

    Ok(cors)
}

fn setup_liftoff_notify(notify: Arc<Notify>) -> AdHoc {
    AdHoc::on_liftoff("Liftoff notifier", |_| {
        Box::pin(async move { notify.notify_one() })
    })
}

fn setup_network_monitor<'a>(
    config: &'a Config,
    _nyxd_client: Client<SigningNyxdClient>,
    system_version: &str,
    rocket: &Rocket<Ignite>,
) -> Option<NetworkMonitorBuilder<'a>> {
    if !config.get_network_monitor_enabled() {
        return None;
    }

    // get instances of managed states
    let node_status_storage = rocket.state::<NymApiStorage>().unwrap().clone();
    let validator_cache = rocket.state::<ValidatorCache>().unwrap().clone();

    Some(NetworkMonitorBuilder::new(
        config,
        _nyxd_client,
        system_version,
        node_status_storage,
        validator_cache,
    ))
}

// TODO: Remove if still unused
#[allow(dead_code)]
fn expected_monitor_test_runs(config: &Config, interval_length: Duration) -> usize {
    let test_delay = config.get_network_monitor_run_interval();

    // this is just a rough estimate. In real world there will be slightly fewer test runs
    // as they are not instantaneous and hence do not happen exactly every test_delay
    (interval_length.as_secs() / test_delay.as_secs()) as usize
}

async fn setup_rocket(
    config: &Config,
    _mix_denom: String,
    liftoff_notify: Arc<Notify>,
    _nyxd_client: Client<SigningNyxdClient>,
    #[cfg(feature = "coconut")] coconut_keypair: coconut::keypair::KeyPair,
) -> Result<Rocket<Ignite>> {
    let openapi_settings = rocket_okapi::settings::OpenApiSettings::default();
    let mut rocket = rocket::build();

    let custom_route_spec = (vec![], custom_openapi_spec());

    mount_endpoints_and_merged_docs! {
        rocket,
        "/v1".to_owned(),
        openapi_settings,
        "/" => custom_route_spec,
        "" => contract_cache::validator_cache_routes(&openapi_settings),
        "/status" => node_status_api::node_status_routes(&openapi_settings, config.get_network_monitor_enabled()),
    }

    let rocket = rocket
        .mount("/swagger", make_swagger_ui(&swagger::get_docs()))
        .attach(setup_cors()?)
        .attach(setup_liftoff_notify(liftoff_notify))
        .attach(ValidatorCache::stage())
        .attach(NodeStatusCache::stage());

    // This is not a very nice approach. A lazy value would be more suitable, but that's still
    // a nightly feature: https://github.com/rust-lang/rust/issues/74465
    let storage = if cfg!(feature = "coconut") || config.get_network_monitor_enabled() {
        Some(NymApiStorage::init(config.get_node_status_api_database_path()).await?)
    } else {
        None
    };

    #[cfg(feature = "coconut")]
    let rocket = if config.get_coconut_signer_enabled() {
        rocket.attach(InternalSignRequest::stage(
            _nyxd_client.clone(),
            _mix_denom,
            coconut_keypair,
            QueryCommunicationChannel::new(_nyxd_client),
            storage.clone().unwrap(),
        ))
    } else {
        rocket
    };

    // see if we should start up network monitor
    let rocket = if config.get_network_monitor_enabled() {
        rocket.attach(storage::NymApiStorage::stage(storage.unwrap()))
    } else {
        rocket
    };

    Ok(rocket.ignite().await?)
}

fn custom_openapi_spec() -> OpenApi {
    use rocket_okapi::okapi::openapi3::*;
    OpenApi {
        openapi: OpenApi::default_version(),
        info: Info {
            title: "Validator API".to_owned(),
            description: None,
            terms_of_service: None,
            contact: None,
            license: None,
            version: env!("CARGO_PKG_VERSION").to_owned(),
            ..Default::default()
        },
        servers: get_servers(),
        ..Default::default()
    }
}

fn get_servers() -> Vec<rocket_okapi::okapi::openapi3::Server> {
    if std::env::var_os("CARGO").is_some() {
        return vec![];
    }
    vec![rocket_okapi::okapi::openapi3::Server {
        url: std::env::var("OPEN_API_BASE").unwrap_or_else(|_| "/api/v1/".to_owned()),
        description: Some("API".to_owned()),
        ..Default::default()
    }]
}

async fn run_nym_api(args: ApiArgs) -> Result<()> {
    let system_version = env!("CARGO_PKG_VERSION");

    // try to load config from the file, if it doesn't exist, use default values
    let id = args.id.as_deref();
    let (config, _already_inited) = match Config::load_from_file(id) {
        Ok(cfg) => (cfg, true),
        Err(_) => {
            let config_path = Config::default_config_file_path(id)
                .into_os_string()
                .into_string()
                .unwrap();
            warn!(
                "Could not load the configuration file from {config_path}. Either the file did not exist or was malformed. Using the default values instead",                
            );
            (Config::new(), false)
        }
    };

    let save_to_file = args.save_config;
    let config = override_config(config, args);

    #[cfg(feature = "coconut")]
    if !_already_inited {
        init_keypair(&config)?;
    }

    // if we just wanted to write data to the config, exit
    if save_to_file {
        info!("Saving the configuration to a file");
        if let Err(err) = config.save_to_file(None) {
            error!("Failed to write config to a file - {err}");
            process::exit(1)
        } else {
            return Ok(());
        }
    }

    let mix_denom = std::env::var(MIX_DENOM).expect("mix denom not set");

    let signing_nyxd_client = Client::new_signing(&config);

    let liftoff_notify = Arc::new(Notify::new());
    // We need a bigger timeout
    let shutdown = TaskManager::new(10);

    #[cfg(feature = "coconut")]
    let coconut_keypair = coconut::keypair::KeyPair::new();

    // let's build our rocket!
    let rocket = setup_rocket(
        &config,
        mix_denom,
        Arc::clone(&liftoff_notify),
        signing_nyxd_client.clone(),
        #[cfg(feature = "coconut")]
        coconut_keypair.clone(),
    )
    .await?;
    let monitor_builder = setup_network_monitor(
        &config,
        signing_nyxd_client.clone(),
        system_version,
        &rocket,
    );

    let validator_cache = rocket.state::<ValidatorCache>().unwrap().clone();
    let node_status_cache = rocket.state::<NodeStatusCache>().unwrap().clone();

    #[cfg(feature = "coconut")]
    {
        let dkg_controller =
            DkgController::new(&config, signing_nyxd_client.clone(), coconut_keypair, OsRng)
                .await?;
        let shutdown_listener = shutdown.subscribe();
        tokio::spawn(async move { dkg_controller.run(shutdown_listener).await });
    }

    // if network monitor is disabled, we're not going to be sending any rewarding hence
    // we're not starting signing client
    let validator_cache_listener = if config.get_network_monitor_enabled() {
        // Main storage
        let storage = rocket.state::<NymApiStorage>().unwrap().clone();

        // setup our daily uptime updater. Note that if network monitor is disabled, then we have
        // no data for the updates and hence we don't need to start it up
        let uptime_updater = HistoricalUptimeUpdater::new(storage.clone());
        let shutdown_listener = shutdown.subscribe();
        tokio::spawn(async move { uptime_updater.run(shutdown_listener).await });

        // spawn the validator cache refresher
        let validator_cache_refresher = ValidatorCacheRefresher::new(
            signing_nyxd_client.clone(),
            config.get_caching_interval(),
            validator_cache.clone(),
        );
        let validator_cache_listener = validator_cache_refresher.subscribe();
        let shutdown_listener = shutdown.subscribe();
        tokio::spawn(async move { validator_cache_refresher.run(shutdown_listener).await });

        // spawn rewarded set updater
        if config.get_rewarding_enabled() {
            let mut rewarded_set_updater =
                RewardedSetUpdater::new(signing_nyxd_client, validator_cache.clone(), storage)
                    .await?;
            let shutdown_listener = shutdown.subscribe();
            tokio::spawn(async move { rewarded_set_updater.run(shutdown_listener).await.unwrap() });
        }
        validator_cache_listener
    } else {
        // Spawn the validator cache refresher.
        // When the network monitor is not enabled, we spawn the validator cache refresher task
        // with just a nyxd client, in contrast to a signing client.
        let nyxd_client = Client::new_query(&config);
        let validator_cache_refresher = ValidatorCacheRefresher::new(
            nyxd_client,
            config.get_caching_interval(),
            validator_cache.clone(),
        );
        let validator_cache_listener = validator_cache_refresher.subscribe();
        let shutdown_listener = shutdown.subscribe();
        tokio::spawn(async move { validator_cache_refresher.run(shutdown_listener).await });

        validator_cache_listener
    };

    // Spawn the node status cache refresher.
    // It is primarily refreshed in-sync with the validator cache, however provide a fallback
    // caching interval that is twice the validator cache
    let storage = rocket.state::<NymApiStorage>().cloned();
    let mut nym_api_cache_refresher = node_status_api::NodeStatusCacheRefresher::new(
        node_status_cache,
        config.get_caching_interval().saturating_mul(2),
        validator_cache,
        validator_cache_listener,
        storage,
    );
    let shutdown_listener = shutdown.subscribe();
    tokio::spawn(async move { nym_api_cache_refresher.run(shutdown_listener).await });

    // launch the rocket!
    // Rocket handles shutdown on it's own, but its shutdown handling should be incorporated
    // with that of the rest of the tasks.
    // Currently it's runtime is forcefully terminated once the nym-api exits.
    let shutdown_handle = rocket.shutdown();
    tokio::spawn(rocket.launch());

    // to finish building our monitor, we need to have rocket up and running so that we could
    // obtain our bandwidth credential
    if let Some(monitor_builder) = monitor_builder {
        info!("Starting network monitor...");
        // wait for rocket's liftoff stage
        liftoff_notify.notified().await;

        // we're ready to go! spawn the network monitor!
        let runnables = monitor_builder.build().await;
        runnables.spawn_tasks(&shutdown);
    } else {
        info!("Network monitoring is disabled.");
    }

    wait_for_interrupt(shutdown).await;
    shutdown_handle.notify();

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting validator api...");

    cfg_if::cfg_if! {if #[cfg(feature = "console-subscriber")] {
        // instriment tokio console subscriber needs RUSTFLAGS="--cfg tokio_unstable" at build time
        console_subscriber::init();
    }}

    setup_logging();
    let args = ApiArgs::parse();
    setup_env(args.config_env_file.as_ref());
    run_nym_api(args).await
}
