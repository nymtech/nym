// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::try_upgrade_config;
use crate::config::{default_config_directory, default_config_filepath, default_data_directory};
use crate::{
    cli::{override_config, OverrideConfig},
    config::Config,
    error::NetworkRequesterError,
};
use clap::Args;
use nym_bin_common::output_format::OutputFormat;
use nym_client_core::client::base_client::storage::gateway_details::OnDiskGatewayDetails;
use nym_client_core::client::key_manager::persistence::OnDiskKeys;
use nym_client_core::config::GatewayEndpointConfig;
use nym_client_core::error::ClientCoreError;
use nym_client_core::init::helpers::current_gateways;
use nym_client_core::init::types::GatewaySetup;
use nym_client_core::init::types::{GatewayDetails, GatewaySelectionSpecification};
use nym_crypto::asymmetric::identity;
use nym_sphinx::addressing::clients::Recipient;
use serde::Serialize;
use std::fmt::Display;
use std::{fs, io};
use tap::TapFallible;

#[derive(Args, Clone)]
pub(crate) struct Init {
    /// Id of the nym-mixnet-client we want to create config for.
    #[arg(long)]
    id: String,

    /// Specifies whether this network requester should run in 'open-proxy' mode
    #[arg(long)]
    open_proxy: Option<bool>,

    /// Enable service anonymized statistics that get sent to a statistics aggregator server
    #[arg(long)]
    enable_statistics: Option<bool>,

    /// Mixnet client address where a statistics aggregator is running. The default value is a Nym
    /// aggregator client
    #[arg(long)]
    statistics_recipient: Option<String>,

    /// Id of the gateway we are going to connect to.
    #[arg(long)]
    gateway: Option<identity::PublicKey>,

    /// Specifies whether the new gateway should be determined based by latency as opposed to being chosen
    /// uniformly.
    #[arg(long, conflicts_with = "gateway")]
    latency_based_selection: bool,

    /// Force register gateway. WARNING: this will overwrite any existing keys for the given id,
    /// potentially causing loss of access.
    #[arg(long)]
    force_register_gateway: bool,

    /// Comma separated list of rest endpoints of the nyxd validators
    #[arg(long, alias = "nymd_validators", value_delimiter = ',')]
    nyxd_urls: Option<Vec<url::Url>>,

    /// Comma separated list of rest endpoints of the API validators
    #[arg(long, alias = "api_validators", value_delimiter = ',')]
    // the alias here is included for backwards compatibility (1.1.4 and before)
    nym_apis: Option<Vec<url::Url>>,

    /// Set this client to work in a enabled credentials mode that would attempt to use gateway
    /// with bandwidth credential requirement.
    #[arg(long)]
    enabled_credentials_mode: Option<bool>,

    #[arg(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

impl From<Init> for OverrideConfig {
    fn from(init_config: Init) -> Self {
        OverrideConfig {
            nym_apis: init_config.nym_apis,
            fastmode: false,
            no_cover: false,
            medium_toggle: false,
            nyxd_urls: init_config.nyxd_urls,
            enabled_credentials_mode: init_config.enabled_credentials_mode,
            open_proxy: init_config.open_proxy,
            enable_statistics: init_config.enabled_credentials_mode,
            statistics_recipient: init_config.statistics_recipient,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct InitResults {
    #[serde(flatten)]
    client_core: nym_client_core::init::types::InitResults,
    client_address: String,
}

impl InitResults {
    fn new(config: &Config, address: &Recipient, gateway: &GatewayEndpointConfig) -> Self {
        Self {
            client_core: nym_client_core::init::types::InitResults::new(
                &config.base,
                address,
                gateway,
            ),
            client_address: address.to_string(),
        }
    }
}

impl Display for InitResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.client_core)?;
        write!(
            f,
            "Address of this network-requester: {}",
            self.client_address
        )
    }
}

fn init_paths(id: &str) -> io::Result<()> {
    fs::create_dir_all(default_data_directory(id))?;
    fs::create_dir_all(default_config_directory(id))
}

pub(crate) async fn execute(args: &Init) -> Result<(), NetworkRequesterError> {
    log::info!("Initialising client...");

    let id = &args.id;

    let already_init = if default_config_filepath(id).exists() {
        // in case we're using old config, try to upgrade it
        // (if we're using the current version, it's a no-op)
        try_upgrade_config(id)?;
        log::info!("Client \"{id}\" was already initialised before");
        true
    } else {
        init_paths(&args.id)?;
        false
    };

    // Usually you only register with the gateway on the first init, however you can force
    // re-registering if wanted.
    let user_wants_force_register = args.force_register_gateway;
    if user_wants_force_register {
        log::warn!("Instructed to force registering gateway. This might overwrite keys!");
    }

    // If the client was already initialized, don't generate new keys and don't re-register with
    // the gateway (because this would create a new shared key).
    // Unless the user really wants to.
    let register_gateway = !already_init || user_wants_force_register;

    // Attempt to use a user-provided gateway, if possible
    let user_chosen_gateway_id = args.gateway;
    let selection_spec = GatewaySelectionSpecification::new(
        user_chosen_gateway_id.map(|id| id.to_base58_string()),
        Some(args.latency_based_selection),
    );

    // Load and potentially override config
    let config = override_config(Config::new(id), OverrideConfig::from(args.clone()));
    log::debug!("Using config: {:#?}", config);

    // Setup gateway by either registering a new one, or creating a new config from the selected
    // one but with keys kept, or reusing the gateway configuration.
    let key_store = OnDiskKeys::new(config.storage_paths.common_paths.keys.clone());
    let details_store =
        OnDiskGatewayDetails::new(&config.storage_paths.common_paths.gateway_details);

    let available_gateways = {
        let mut rng = rand::thread_rng();
        current_gateways(&mut rng, &config.base.client.nym_api_urls).await?
    };

    let gateway_setup = GatewaySetup::New {
        specification: selection_spec,
        available_gateways,
        overwrite_data: register_gateway,
    };

    let init_details =
        nym_client_core::init::setup_gateway(gateway_setup, &key_store, &details_store)
            .await
            .tap_err(|err| log::error!("Failed to setup gateway\nError: {err}"))?;

    let config_save_location = config.default_location();
    config.save_to_default_location().tap_err(|_| {
        log::error!("Failed to save the config file");
    })?;
    log::info!(
        "Saved configuration file to {}",
        config_save_location.display()
    );

    let address = init_details.client_address()?;

    log::info!("Client configuration completed.\n");

    let GatewayDetails::Configured(gateway_details) = init_details.gateway_details else {
        return Err(ClientCoreError::UnexpectedPersistedCustomGatewayDetails)?;
    };
    let init_results = InitResults::new(&config, &address, &gateway_details);
    println!("{}", args.output.format(&init_results));

    Ok(())
}
