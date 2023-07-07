// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::WasmCoreError;
use crate::storage::wasm_client_traits::WasmClientStorage;
use crate::storage::ClientStorage;
use crate::topology::WasmNymTopology;
use js_sys::Promise;
use nym_bandwidth_controller::wasm_mockups::{Client as FakeClient, DirectSigningNyxdClient};
use nym_bandwidth_controller::BandwidthController;
use nym_client_core::client::replies::reply_storage::browser_backend;
use nym_client_core::config;
use nym_client_core::init::helpers::current_gateways;
use nym_client_core::init::{setup_gateway_from, GatewaySetup, InitialisationDetails};
pub use nym_credential_storage::ephemeral_storage::EphemeralStorage as EphemeralCredentialStorage;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use nym_topology::{gateway, NymTopology};
use nym_validator_client::client::IdentityKey;
use nym_validator_client::NymApiClient;
use rand::thread_rng;
use url::Url;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen_futures::future_to_promise;
use wasm_utils::error::PromisableResult;

pub type FakeWasmDkgClient = FakeClient<DirectSigningNyxdClient>;
pub type WasmBandwidthController =
    BandwidthController<FakeWasmDkgClient, EphemeralCredentialStorage>;

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
) -> Result<WasmNymTopology, WasmCoreError> {
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

#[wasm_bindgen]
pub fn current_network_topology(nym_api_url: String) -> Promise {
    future_to_promise(async move {
        current_network_topology_async(nym_api_url)
            .await
            .into_promise_result()
    })
}

async fn setup_gateway(
    client_store: &ClientStorage,
    chosen_gateway: Option<IdentityKey>,
    gateways: &[gateway::Node],
) -> Result<InitialisationDetails, WasmCoreError> {
    let setup = if client_store.has_full_gateway_info().await? {
        GatewaySetup::MustLoad
    } else {
        GatewaySetup::new_fresh(chosen_gateway.clone(), None)
    };

    setup_gateway_from(&setup, client_store, client_store, false, Some(gateways))
        .await
        .map_err(Into::into)
}

pub async fn setup_gateway_from_api(
    client_store: &ClientStorage,
    chosen_gateway: Option<IdentityKey>,
    nym_apis: &[Url],
) -> Result<InitialisationDetails, WasmCoreError> {
    let mut rng = thread_rng();
    let gateways = current_gateways(&mut rng, nym_apis).await?;
    setup_gateway(client_store, chosen_gateway, &gateways).await
}

pub async fn setup_from_topology(
    explicit_gateway: Option<IdentityKey>,
    topology: &NymTopology,
    client_store: &ClientStorage,
) -> Result<InitialisationDetails, WasmCoreError> {
    let gateways = topology.gateways();
    setup_gateway(client_store, explicit_gateway, gateways).await
}
