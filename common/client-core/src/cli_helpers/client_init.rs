// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::disk_persistence::CommonClientPaths;
use crate::error::ClientCoreError;
use crate::{
    client::{
        base_client::{
            non_wasm_helpers::setup_fs_gateways_storage,
            storage::helpers::{get_all_registered_identities, set_active_gateway},
        },
        key_manager::persistence::OnDiskKeys,
    },
    init::types::{GatewaySelectionSpecification, GatewaySetup, InitResults},
};
use log::info;
use nym_client_core_gateways_storage::GatewayDetails;
use nym_crypto::asymmetric::identity;
use nym_topology::NymTopology;
use rand::rngs::OsRng;
use std::path::{Path, PathBuf};

// we can suppress this warning (as suggested by linter itself) since we're only using it in our own code
#[allow(async_fn_in_trait)]
pub trait InitialisableClient {
    const NAME: &'static str;
    type Error: From<ClientCoreError>;
    type InitArgs: AsRef<CommonClientInitArgs>;
    type Config: ClientConfig;

    async fn try_upgrade_outdated_config(id: &str) -> Result<(), Self::Error>;

    fn initialise_storage_paths(id: &str) -> Result<(), Self::Error>;

    fn default_config_path(id: &str) -> PathBuf;

    fn construct_config(init_args: &Self::InitArgs) -> Self::Config;
}

pub trait ClientConfig {
    fn common_paths(&self) -> &CommonClientPaths;

    fn core_config(&self) -> &crate::config::Config;

    fn default_store_location(&self) -> PathBuf;

    fn save_to<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()>;
}

#[cfg_attr(feature = "cli", derive(clap::Args))]
#[derive(Debug, Clone)]
pub struct CommonClientInitArgs {
    /// Id of client we want to create config for.
    #[cfg_attr(feature = "cli", clap(long))]
    pub id: String,

    /// Id of the gateway we are going to connect to.
    #[cfg_attr(feature = "cli", clap(long))]
    pub gateway: Option<identity::PublicKey>,

    /// Specifies whether the client will attempt to enforce tls connection to the desired gateway.
    #[cfg_attr(feature = "cli", clap(long))]
    pub force_tls_gateway: bool,

    /// Specifies whether the new gateway should be determined based by latency as opposed to being chosen
    /// uniformly.
    #[cfg_attr(feature = "cli", clap(long, conflicts_with = "gateway"))]
    pub latency_based_selection: bool,

    /// Force register gateway. WARNING: this will overwrite any existing keys for the given id,
    /// potentially causing loss of access.
    #[cfg_attr(feature = "cli", clap(long))]
    pub force_register_gateway: bool,

    /// If the registration is happening against new gateway,
    /// specify whether it should be set as the currently active gateway
    #[cfg_attr(feature = "cli", clap(long, default_value_t = true))]
    pub set_active: bool,

    /// Comma separated list of rest endpoints of the nyxd validators
    #[cfg_attr(
        feature = "cli",
        clap(long, alias = "nyxd_validators", value_delimiter = ',', hide = true)
    )]
    pub nyxd_urls: Option<Vec<url::Url>>,

    /// Comma separated list of rest endpoints of the API validators
    #[cfg_attr(
        feature = "cli",
        clap(
            long,
            alias = "api_validators",
            value_delimiter = ',',
            group = "network"
        )
    )]
    pub nym_apis: Option<Vec<url::Url>>,

    /// Path to .json file containing custom network specification.
    #[cfg_attr(feature = "cli", clap(long, group = "network", hide = true))]
    pub custom_mixnet: Option<PathBuf>,

    /// Set this client to work in a enabled credentials mode that would attempt to use gateway
    /// with bandwidth credential requirement.
    #[cfg_attr(feature = "cli", clap(long, hide = true))]
    pub enabled_credentials_mode: Option<bool>,

    /// Mostly debug-related option to increase default traffic rate so that you would not need to
    /// modify config post init
    #[cfg_attr(feature = "cli", clap(long, hide = true))]
    pub fastmode: bool,

    /// Disable loop cover traffic and the Poisson rate limiter (for debugging only)
    #[cfg_attr(feature = "cli", clap(long, hide = true))]
    pub no_cover: bool,
}

pub struct InitResultsWithConfig<T> {
    pub config: T,
    pub init_results: InitResults,
}

pub async fn initialise_client<C>(
    init_args: C::InitArgs,
) -> Result<InitResultsWithConfig<C::Config>, C::Error>
where
    C: InitialisableClient,
    <C as InitialisableClient>::Config: std::fmt::Debug,
    <C as InitialisableClient>::InitArgs: std::fmt::Debug,
{
    info!("initialising {} client", C::NAME);

    let common_args = init_args.as_ref();
    let id = &common_args.id;

    let already_init = if C::default_config_path(id).exists() {
        // in case we're using old config, try to upgrade it
        // (if we're using the current version, it's a no-op)
        C::try_upgrade_outdated_config(id).await?;
        eprintln!("{} client \"{id}\" was already initialised before", C::NAME);
        true
    } else {
        info!(
            "{} client {id:?} hasn't been initialised before - new keys are going to be generated",
            C::NAME
        );
        C::initialise_storage_paths(id)?;
        false
    };

    // Usually you only register with the gateway on the first init, however you can force
    // re-registering if wanted.
    let user_wants_force_register = common_args.force_register_gateway;
    if user_wants_force_register {
        eprintln!("Instructed to force registering gateway. This might overwrite keys!");
    }

    // Attempt to use a user-provided gateway, if possible
    let user_chosen_gateway_id = common_args.gateway;
    log::debug!("User chosen gateway id: {user_chosen_gateway_id:?}");

    let selection_spec = GatewaySelectionSpecification::new(
        user_chosen_gateway_id.map(|id| id.to_base58_string()),
        Some(common_args.latency_based_selection),
        common_args.force_tls_gateway,
    );
    log::debug!("Gateway selection specification: {selection_spec:?}");

    // Load and potentially override config
    log::debug!("Init arguments: {init_args:#?}");
    let config = C::construct_config(&init_args);
    log::debug!("Constructed config: {config:#?}");
    let paths = config.common_paths();
    let core = config.core_config();

    log::info!(
        "Using nym-api: {}",
        core.client
            .nym_api_urls
            .iter()
            .map(|url| url.as_str())
            .collect::<Vec<&str>>()
            .join(",")
    );

    let key_store = OnDiskKeys::new(paths.keys.clone());
    let details_store = setup_fs_gateways_storage(&paths.gateway_registrations).await?;

    // if this is a first time client with this particular id is initialised, generated long-term keys
    if !already_init {
        let mut rng = OsRng;
        crate::init::generate_new_client_keys(&mut rng, &key_store).await?;
    }

    let registered_gateways = get_all_registered_identities(&details_store).await?;

    // if user provided gateway id (and we can't overwrite data), make sure we're not trying to register
    // with a known gateway
    if let Some(user_chosen) = user_chosen_gateway_id {
        if !common_args.force_register_gateway && registered_gateways.contains(&user_chosen) {
            return Err(ClientCoreError::AlreadyRegistered {
                gateway_id: user_chosen.to_base58_string(),
            }
            .into());
        }
    }

    // Setup gateway by either registering a new one, or creating a new config from the selected
    // one but with keys kept, or reusing the gateway configuration.
    let available_gateways = if let Some(custom_mixnet) = common_args.custom_mixnet.as_ref() {
        let hardcoded_topology = NymTopology::new_from_file(custom_mixnet).map_err(|source| {
            ClientCoreError::CustomTopologyLoadFailure {
                file_path: custom_mixnet.clone(),
                source,
            }
        })?;
        hardcoded_topology.get_gateways()
    } else {
        let mut rng = rand::thread_rng();
        crate::init::helpers::current_gateways(&mut rng, &core.client.nym_api_urls).await?
    };

    // since we're registering with a brand new gateway,
    // make sure the list of available gateways doesn't overlap the list of known gateways
    let available_gateways = if common_args.force_register_gateway {
        // if we're force registering, all bets are off
        available_gateways
    } else {
        available_gateways
            .into_iter()
            .filter(|g| !registered_gateways.contains(g.identity()))
            .collect()
    };

    let gateway_setup = GatewaySetup::New {
        specification: selection_spec,
        available_gateways,
        overwrite_data: common_args.force_register_gateway,
        wg_tun_address: None,
    };

    let init_details =
        crate::init::setup_gateway(gateway_setup, &key_store, &details_store).await?;

    // TODO: ask the service provider we specified for its interface version and set it in the config

    if !already_init {
        let config_save_location = config.default_store_location();
        if let Err(err) = config.save_to(&config_save_location) {
            return Err(ClientCoreError::ConfigSaveFailure {
                typ: C::NAME.to_string(),
                id: id.to_string(),
                path: config_save_location,
                source: err,
            }
            .into());
        }

        eprintln!(
            "Saved configuration file to {}",
            config_save_location.display()
        );
    }

    let address = init_details.client_address();

    let GatewayDetails::Remote(gateway_details) = init_details.gateway_registration.details else {
        return Err(ClientCoreError::UnexpectedPersistedCustomGatewayDetails)?;
    };

    let init_results = InitResults::new(
        config.core_config(),
        address,
        &gateway_details,
        init_details.gateway_registration.registration_timestamp,
    );

    if init_args.as_ref().set_active {
        set_active_gateway(&details_store, &init_results.gateway_id).await?;
    } else {
        info!("registered with new gateway {} (under address {address}), but this will not be our default address", init_results.gateway_id);
    }

    Ok(InitResultsWithConfig {
        config,
        init_results,
    })
}
