// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli_helpers::traits::{CliClient, CliClientConfig};
use crate::error::ClientCoreError;
use crate::{
    client::{
        base_client::{
            non_wasm_helpers::setup_fs_gateways_storage, storage::helpers::set_active_gateway,
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
use std::path::PathBuf;

// we can suppress this warning (as suggested by linter itself) since we're only using it in our own code
#[allow(async_fn_in_trait)]
pub trait InitialisableClient: CliClient {
    type InitArgs: AsRef<CommonClientInitArgs>;

    fn initialise_storage_paths(id: &str) -> Result<(), Self::Error>;

    fn default_config_path(id: &str) -> PathBuf;

    fn construct_config(init_args: &Self::InitArgs) -> Self::Config;
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

    ///Comma separated list of urls to use for domain fronting
    #[cfg_attr(
        feature = "cli",
        clap(long, value_delimiter = ',', requires = "nym_apis", hide = true)
    )]
    pub fronting_domains: Option<Vec<url::Url>>,

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
    <C as CliClient>::Config: std::fmt::Debug,
    <C as InitialisableClient>::InitArgs: std::fmt::Debug,
{
    info!("initialising {} client", C::NAME);

    let common_args = init_args.as_ref();
    let id = &common_args.id;

    if C::default_config_path(id).exists() {
        eprintln!("{} client \"{id}\" was already initialised before", C::NAME);
        return Err(ClientCoreError::AlreadyInitialised {
            client_id: id.to_string(),
        }
        .into());
    }

    C::initialise_storage_paths(id)?;

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
    if let Some(fronting_domains) = &core.client.fronting_domains {
        log::info!(
            "fronted by : {}",
            fronting_domains
                .iter()
                .map(|url| url.host_str().unwrap_or_default())
                .collect::<Vec<&str>>()
                .join(",")
        );
    }

    let key_store = OnDiskKeys::new(paths.keys.clone());
    let details_store = setup_fs_gateways_storage(&paths.gateway_registrations).await?;

    let mut rng = OsRng;
    crate::init::generate_new_client_keys(&mut rng, &key_store).await?;

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
        crate::init::helpers::current_gateways(
            &mut rng,
            &core.client.nym_api_urls,
            core.client.fronting_domains.as_ref(),
        )
        .await?
    };

    let gateway_setup = GatewaySetup::New {
        specification: selection_spec,
        available_gateways,
        wg_tun_address: None,
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

    set_active_gateway(&details_store, &init_results.gateway_id).await?;

    Ok(InitResultsWithConfig {
        config,
        init_results,
    })
}
