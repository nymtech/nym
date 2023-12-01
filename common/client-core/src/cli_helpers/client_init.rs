// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::disk_persistence::CommonClientPaths;
use crate::error::ClientCoreError;
use crate::{
    client::{
        base_client::storage::gateway_details::OnDiskGatewayDetails,
        key_manager::persistence::OnDiskKeys,
    },
    init::types::{GatewayDetails, GatewaySelectionSpecification, GatewaySetup, InitResults},
};
use log::info;
use nym_crypto::asymmetric::identity;
use nym_topology::NymTopology;
use std::path::{Path, PathBuf};

pub trait InitialisableClient {
    const NAME: &'static str;
    type Error: From<ClientCoreError>;
    type InitArgs: AsRef<CommonClientInitArgs>;
    type Config: ClientConfig;

    fn try_upgrade_outdated_config(id: &str) -> Result<(), Self::Error>;

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

    /// Specifies whether the new gateway should be determined based by latency as opposed to being chosen
    /// uniformly.
    #[cfg_attr(feature = "cli", clap(long, conflicts_with = "gateway"))]
    pub latency_based_selection: bool,

    /// Force register gateway. WARNING: this will overwrite any existing keys for the given id,
    /// potentially causing loss of access.
    #[cfg_attr(feature = "cli", clap(long))]
    pub force_register_gateway: bool,

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
{
    info!("initialising {} client", C::NAME);

    let common_args = init_args.as_ref();
    let id = &common_args.id;

    let already_init = if C::default_config_path(id).exists() {
        // in case we're using old config, try to upgrade it
        // (if we're using the current version, it's a no-op)
        C::try_upgrade_outdated_config(id)?;
        eprintln!("{} client \"{id}\" was already initialised before", C::NAME);
        true
    } else {
        C::initialise_storage_paths(id)?;
        false
    };

    // Usually you only register with the gateway on the first init, however you can force
    // re-registering if wanted.
    let user_wants_force_register = common_args.force_register_gateway;
    if user_wants_force_register {
        eprintln!("Instructed to force registering gateway. This might overwrite keys!");
    }

    // If the client was already initialized, don't generate new keys and don't re-register with
    // the gateway (because this would create a new shared key).
    // Unless the user really wants to.
    let register_gateway = !already_init || user_wants_force_register;

    // Attempt to use a user-provided gateway, if possible
    let user_chosen_gateway_id = common_args.gateway;
    let selection_spec = GatewaySelectionSpecification::new(
        user_chosen_gateway_id.map(|id| id.to_base58_string()),
        Some(common_args.latency_based_selection),
        false,
    );

    // Load and potentially override config
    let config = C::construct_config(&init_args);
    let paths = config.common_paths();
    let core = config.core_config();

    // Setup gateway by either registering a new one, or creating a new config from the selected
    // one but with keys kept, or reusing the gateway configuration.
    let key_store = OnDiskKeys::new(paths.keys.clone());
    let details_store = OnDiskGatewayDetails::new(&paths.gateway_details);

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

    let gateway_setup = GatewaySetup::New {
        specification: selection_spec,
        available_gateways,
        overwrite_data: register_gateway,
    };

    let init_details =
        crate::init::setup_gateway(gateway_setup, &key_store, &details_store).await?;

    // TODO: ask the service provider we specified for its interface version and set it in the config

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

    let address = init_details.client_address()?;

    let GatewayDetails::Configured(gateway_details) = init_details.gateway_details else {
        return Err(ClientCoreError::UnexpectedPersistedCustomGatewayDetails)?;
    };
    let init_results = InitResults::new(config.core_config(), address, &gateway_details);

    Ok(InitResultsWithConfig {
        config,
        init_results,
    })
}
