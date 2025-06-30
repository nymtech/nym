// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::WasmCoreError;
use crate::storage::wasm_client_traits::WasmClientStorage;
use crate::storage::ClientStorage;
use js_sys::Promise;
use nym_client_core::client::base_client::storage::helpers::set_active_gateway;
use nym_client_core::client::base_client::storage::GatewaysDetailsStore;
use nym_client_core::client::replies::reply_storage::browser_backend;
use nym_client_core::config;
use nym_client_core::error::ClientCoreError;
use nym_client_core::init::helpers::gateways_for_init;
use nym_client_core::init::types::GatewaySelectionSpecification;
use nym_client_core::init::{
    self, setup_gateway,
    types::{GatewaySetup, InitialisationResult},
};
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use nym_topology::wasm_helpers::WasmFriendlyNymTopology;
use nym_topology::{NymTopology, RoutingNode};
use nym_validator_client::client::IdentityKey;
use nym_validator_client::{NymApiClient, UserAgent};
use rand::thread_rng;
use url::Url;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen_futures::future_to_promise;
use wasm_utils::error::PromisableResult;

pub use nym_credential_storage::ephemeral_storage::EphemeralStorage as EphemeralCredentialStorage;
use nym_topology::provider_trait::ToTopologyMetadata;
use wasm_utils::{console_log, console_warn};

// don't get too excited about the name, under the hood it's just a big fat placeholder
// with no disk_persistence
pub fn setup_reply_surb_storage_backend(config: config::ReplySurbs) -> browser_backend::Backend {
    browser_backend::Backend::new(
        config.minimum_reply_surb_storage_threshold,
        config.maximum_reply_surb_storage_threshold,
    )
}

pub fn parse_recipient(recipient: &str) -> Result<Recipient, WasmCoreError> {
    Recipient::try_from_base58_string(recipient).map_err(|source| {
        WasmCoreError::MalformedRecipient {
            raw: recipient.to_string(),
            source,
        }
    })
}

pub fn parse_sender_tag(tag: &str) -> Result<AnonymousSenderTag, WasmCoreError> {
    AnonymousSenderTag::try_from_base58_string(tag).map_err(|source| {
        WasmCoreError::MalformedSenderTag {
            raw: tag.to_string(),
            source,
        }
    })
}

pub async fn current_network_topology_async(
    nym_api_url: String,
) -> Result<WasmFriendlyNymTopology, WasmCoreError> {
    let url: Url = match nym_api_url.parse() {
        Ok(url) => url,
        Err(source) => {
            return Err(WasmCoreError::MalformedUrl {
                raw: nym_api_url,
                source,
            })
        }
    };

    let api_client = NymApiClient::new(url);
    let rewarded_set = api_client.get_current_rewarded_set().await?;
    let mixnodes_res = api_client
        .get_all_basic_active_mixing_assigned_nodes_with_metadata()
        .await?;
    let metadata = mixnodes_res.metadata;
    let mixnodes = mixnodes_res.nodes;

    let gateways_res = api_client
        .get_all_basic_entry_assigned_nodes_with_metadata()
        .await?;
    if gateways_res.metadata != metadata {
        console_warn!("inconsistent nodes metadata between mixnodes and gateways calls! {metadata:?} and {:?}", gateways_res.metadata);
        return Err(WasmCoreError::UnavailableNetworkTopology);
    }

    let gateways = gateways_res.nodes;

    let topology = NymTopology::new(metadata.to_topology_metadata(), rewarded_set, Vec::new())
        .with_skimmed_nodes(&mixnodes)
        .with_skimmed_nodes(&gateways);

    Ok(topology.into())
}

#[wasm_bindgen(js_name = "currentNetworkTopology")]
pub fn current_network_topology(nym_api_url: String) -> Promise {
    // blame js for that serde conversion
    future_to_promise(async move {
        current_network_topology_async(nym_api_url)
            .await
            .map(|topology| serde_wasm_bindgen::to_value(&topology).unwrap())
            .into_promise_result()
    })
}

pub async fn setup_gateway_wasm(
    client_store: &ClientStorage,
    force_tls: bool,
    chosen_gateway: Option<IdentityKey>,
    gateways: Vec<RoutingNode>,
) -> Result<InitialisationResult, WasmCoreError> {
    // TODO: so much optimization and extra features could be added here, but that's for the future

    let setup = if client_store
        .get_active_gateway_id()
        .await?
        .active_gateway_id_bs58
        .is_some()
    {
        GatewaySetup::MustLoad { gateway_id: None }
    } else {
        let selection_spec =
            GatewaySelectionSpecification::new(chosen_gateway.clone(), None, force_tls);

        GatewaySetup::New {
            specification: selection_spec,
            available_gateways: gateways,
        }
    };

    init::setup_gateway(setup, client_store, client_store)
        .await
        .map_err(Into::into)
}

pub async fn setup_gateway_from_api(
    client_store: &ClientStorage,
    force_tls: bool,
    chosen_gateway: Option<IdentityKey>,
    nym_apis: &[Url],
    minimum_performance: u8,
    ignore_epoch_roles: bool,
) -> Result<InitialisationResult, WasmCoreError> {
    let mut rng = thread_rng();
    let gateways = gateways_for_init(
        &mut rng,
        nym_apis,
        None,
        minimum_performance,
        ignore_epoch_roles,
    )
    .await?;
    setup_gateway_wasm(client_store, force_tls, chosen_gateway, gateways).await
}

pub async fn current_gateways_wasm(
    nym_apis: &[Url],
    user_agent: Option<UserAgent>,
    minimum_performance: u8,
    ignore_epoch_roles: bool,
) -> Result<Vec<RoutingNode>, ClientCoreError> {
    let mut rng = thread_rng();
    gateways_for_init(
        &mut rng,
        nym_apis,
        user_agent,
        minimum_performance,
        ignore_epoch_roles,
    )
    .await
}

pub async fn setup_from_topology(
    explicit_gateway: Option<IdentityKey>,
    force_tls: bool,
    topology: &NymTopology,
    client_store: &ClientStorage,
) -> Result<InitialisationResult, WasmCoreError> {
    let gateways = topology.entry_capable_nodes().cloned().collect::<Vec<_>>();
    setup_gateway_wasm(client_store, force_tls, explicit_gateway, gateways).await
}

pub async fn generate_new_client_keys(store: &ClientStorage) -> Result<(), WasmCoreError> {
    let mut rng = thread_rng();
    init::generate_new_client_keys(&mut rng, store).await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn add_gateway(
    preferred_gateway: Option<IdentityKey>,
    latency_based_selection: Option<bool>,
    force_tls: bool,
    nym_apis: &[Url],
    user_agent: UserAgent,
    min_performance: u8,
    ignore_epoch_roles: bool,
    storage: &ClientStorage,
) -> Result<(), WasmCoreError> {
    let selection_spec = GatewaySelectionSpecification::new(
        preferred_gateway.clone(),
        latency_based_selection,
        force_tls,
    );

    let preferred_gateway = preferred_gateway
        .as_ref()
        .map(|g| g.parse())
        .transpose()
        .map_err(|source| WasmCoreError::InvalidGatewayIdentity { source })?;

    let registered_gateways = storage.all_gateways_identities().await.map_err(|source| {
        ClientCoreError::GatewaysDetailsStoreError {
            source: Box::new(source),
        }
    })?;

    // if user provided gateway id (and we can't overwrite data), make sure we're not trying to register
    // with a known gateway
    if let Some(user_chosen) = preferred_gateway {
        if registered_gateways.contains(&user_chosen) {
            return Err(ClientCoreError::AlreadyRegistered {
                gateway_id: user_chosen.to_base58_string(),
            }
            .into());
        }
    }

    // Setup gateway by either registering a new one, or creating a new config from the selected
    // one but with keys kept, or reusing the gateway configuration.
    let available_gateways = current_gateways_wasm(
        nym_apis,
        Some(user_agent),
        min_performance,
        ignore_epoch_roles,
    )
    .await?;

    // since we're registering with a brand new gateway,
    // make sure the list of available gateways doesn't overlap the list of known gateways
    let available_gateways = available_gateways
        .into_iter()
        .filter(|g| !registered_gateways.contains(&g.identity()))
        .collect::<Vec<_>>();

    if available_gateways.is_empty() {
        return Err(ClientCoreError::NoNewGatewaysAvailable.into());
    }

    let gateway_setup = GatewaySetup::New {
        specification: selection_spec,
        available_gateways,
    };

    let init_details = setup_gateway(gateway_setup, storage, storage).await?;
    let gateway = init_details.gateway_id().to_base58_string();
    set_active_gateway(storage, &gateway).await?;

    console_log!("finished registration with gateway {gateway}");
    Ok(())
}
