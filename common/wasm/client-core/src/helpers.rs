// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::WasmCoreError;
use crate::storage::wasm_client_traits::WasmClientStorage;
use crate::storage::ClientStorage;
use js_sys::Promise;
use nym_client_core::client::replies::reply_storage::browser_backend;
use nym_client_core::config;
use nym_client_core::init::helpers::current_gateways;
use nym_client_core::init::types::GatewaySelectionSpecification;
use nym_client_core::init::{
    self,
    types::{GatewaySetup, InitialisationResult},
};
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use nym_topology::{gateway, NymTopology, SerializableNymTopology};
use nym_validator_client::client::IdentityKey;
use nym_validator_client::NymApiClient;
use rand::thread_rng;
use url::Url;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen_futures::future_to_promise;
use wasm_utils::error::PromisableResult;

pub use nym_credential_storage::ephemeral_storage::EphemeralStorage as EphemeralCredentialStorage;

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
) -> Result<SerializableNymTopology, WasmCoreError> {
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
    let mixnodes = api_client.get_cached_active_mixnodes().await?;
    let gateways = api_client.get_cached_gateways().await?;

    Ok(NymTopology::from_detailed(mixnodes, gateways).into())
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
    gateways: &[gateway::Node],
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
            available_gateways: gateways.to_vec(),
            wg_tun_address: None,
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
) -> Result<InitialisationResult, WasmCoreError> {
    let mut rng = thread_rng();
    let gateways = current_gateways(&mut rng, nym_apis).await?;
    setup_gateway_wasm(client_store, force_tls, chosen_gateway, &gateways).await
}

pub async fn setup_from_topology(
    explicit_gateway: Option<IdentityKey>,
    force_tls: bool,
    topology: &NymTopology,
    client_store: &ClientStorage,
) -> Result<InitialisationResult, WasmCoreError> {
    let gateways = topology.gateways();
    setup_gateway_wasm(client_store, force_tls, explicit_gateway, gateways).await
}
