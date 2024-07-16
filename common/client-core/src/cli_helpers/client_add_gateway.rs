// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli_helpers::types::GatewayInfo;
use crate::cli_helpers::{CliClient, CliClientConfig};
use crate::client::base_client::non_wasm_helpers::setup_fs_gateways_storage;
use crate::{
    client::{
        base_client::storage::helpers::{get_all_registered_identities, set_active_gateway},
        key_manager::persistence::OnDiskKeys,
    },
    error::ClientCoreError,
    init::types::{GatewaySelectionSpecification, GatewaySetup},
};
use log::info;
use nym_client_core_gateways_storage::GatewayDetails;
use nym_crypto::asymmetric::identity;
use nym_topology::NymTopology;
use std::path::PathBuf;

#[cfg_attr(feature = "cli", derive(clap::Args))]
#[derive(Debug, Clone)]
pub struct CommonClientAddGatewayArgs {
    /// Id of client we want to add gateway for.
    #[cfg_attr(feature = "cli", clap(long))]
    pub id: String,

    /// Explicitly specify id of the gateway to register with.
    /// If unspecified, a random gateway will be chosen instead.
    #[cfg_attr(feature = "cli", clap(long, alias = "gateway"))]
    pub gateway_id: Option<identity::PublicKey>,

    /// Specifies whether the client will attempt to enforce tls connection to the desired gateway.
    #[cfg_attr(feature = "cli", clap(long))]
    pub force_tls_gateway: bool,

    /// Specifies whether the new gateway should be determined based by latency as opposed to being chosen
    /// uniformly.
    #[cfg_attr(feature = "cli", clap(long, conflicts_with = "gateway_id"))]
    pub latency_based_selection: bool,

    /// Specify whether this new gateway should be set as the active one
    #[cfg_attr(feature = "cli", clap(long, default_value_t = true))]
    pub set_active: bool,

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
}

pub async fn add_gateway<C, A>(args: A) -> Result<GatewayInfo, C::Error>
where
    A: AsRef<CommonClientAddGatewayArgs>,
    C: CliClient,
{
    let common_args = args.as_ref();
    let id = &common_args.id;

    let config = C::try_load_current_config(id).await?;
    let core = config.core_config();
    let paths = config.common_paths();

    let key_store = OnDiskKeys::new(paths.keys.clone());
    let details_store = setup_fs_gateways_storage(&paths.gateway_registrations).await?;

    // Attempt to use a user-provided gateway, if possible
    let user_chosen_gateway_id = common_args.gateway_id;
    log::debug!("User chosen gateway id: {user_chosen_gateway_id:?}");

    let selection_spec = GatewaySelectionSpecification::new(
        user_chosen_gateway_id.map(|id| id.to_base58_string()),
        Some(common_args.latency_based_selection),
        common_args.force_tls_gateway,
    );
    log::debug!("Gateway selection specification: {selection_spec:?}");

    let registered_gateways = get_all_registered_identities(&details_store).await?;

    // if user provided gateway id (and we can't overwrite data), make sure we're not trying to register
    // with a known gateway
    if let Some(user_chosen) = user_chosen_gateway_id {
        if registered_gateways.contains(&user_chosen) {
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
        crate::init::helpers::current_gateways(&mut rng, &core.client.nym_api_urls, None).await?
    };

    // since we're registering with a brand new gateway,
    // make sure the list of available gateways doesn't overlap the list of known gateways
    let available_gateways = available_gateways
        .into_iter()
        .filter(|g| !registered_gateways.contains(g.identity()))
        .collect::<Vec<_>>();

    if available_gateways.is_empty() {
        return Err(ClientCoreError::NoNewGatewaysAvailable.into());
    }

    let gateway_setup = GatewaySetup::New {
        specification: selection_spec,
        available_gateways,
        wg_tun_address: None,
    };

    let init_details =
        crate::init::setup_gateway(gateway_setup, &key_store, &details_store).await?;

    let address = init_details.client_address();

    let gateway_registration = init_details.gateway_registration;
    let GatewayDetails::Remote(ref gateway_details) = gateway_registration.details else {
        return Err(ClientCoreError::UnexpectedPersistedCustomGatewayDetails)?;
    };

    if common_args.set_active {
        set_active_gateway(
            &details_store,
            &gateway_details.gateway_id.to_base58_string(),
        )
        .await?;
    } else {
        info!("registered with new gateway {} (under address {address}), but this will not be our default address", gateway_details.gateway_id);
    }

    Ok(GatewayInfo {
        registration: gateway_registration.registration_timestamp,
        identity: gateway_details.gateway_id,
        active: common_args.set_active,
        typ: gateway_registration.details.typ().to_string(),
        endpoint: Some(gateway_details.gateway_listener.clone()),
        wg_tun_address: gateway_details.wg_tun_address.clone(),
    })
}
