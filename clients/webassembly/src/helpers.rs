// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::WasmClientError;
use crate::storage::ClientStorage;
use crate::topology::WasmNymTopology;
use js_sys::Promise;
use nym_client_core::client::replies::reply_storage::browser_backend;
use nym_client_core::config;
use nym_client_core::config::GatewayEndpointConfig;
use nym_client_core::init::GatewaySetup;
use nym_crypto::asymmetric::identity;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use nym_topology::NymTopology;
use nym_validator_client::client::{IdentityKey, IdentityKeyRef};
use nym_validator_client::NymApiClient;
use rand::{CryptoRng, Rng};
use url::Url;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen_futures::future_to_promise;
use wasm_utils::{console_log, PromisableResult};

// don't get too excited about the name, under the hood it's just a big fat placeholder
// with no disk_persistence
pub(crate) fn setup_reply_surb_storage_backend(
    config: config::ReplySurbs,
) -> browser_backend::Backend {
    browser_backend::Backend::new(
        config.minimum_reply_surb_storage_threshold,
        config.maximum_reply_surb_storage_threshold,
    )
}

pub(crate) fn parse_recipient(recipient: &str) -> Result<Recipient, WasmClientError> {
    Recipient::try_from_base58_string(recipient).map_err(|source| {
        WasmClientError::MalformedRecipient {
            raw: recipient.to_string(),
            source,
        }
    })
}

pub(crate) fn parse_sender_tag(tag: &str) -> Result<AnonymousSenderTag, WasmClientError> {
    AnonymousSenderTag::try_from_base58_string(tag).map_err(|source| {
        WasmClientError::MalformedSenderTag {
            raw: tag.to_string(),
            source,
        }
    })
}

pub(crate) async fn current_network_topology_async(
    nym_api_url: String,
) -> Result<WasmNymTopology, WasmClientError> {
    let url: Url = match nym_api_url.parse() {
        Ok(url) => url,
        Err(source) => {
            return Err(WasmClientError::MalformedUrl {
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

pub(crate) async fn choose_gateway(
    client_store: &ClientStorage,
    chosen_gateway: Option<IdentityKey>,
    nym_apis: &[Url],
) -> Result<GatewayEndpointConfig, WasmClientError> {
    let existing_gateway_config = client_store.read_gateway_config().await?;

    console_log!("loaded: {:?}", existing_gateway_config);

    if let Some(existing) = existing_gateway_config {
        if let Some(provided) = &chosen_gateway {
            if provided != &existing.gateway_id {
                return Err(WasmClientError::AlreadyRegistered {
                    gateway_config: existing,
                });
            }
        }
        return Ok(existing);
    };

    // if NOTHING is specified nor available, choose gateway randomly.
    let setup = GatewaySetup::new(None, chosen_gateway, None);
    let config = setup.try_get_new_gateway_details(nym_apis).await?;

    // perform registration + persist the new gateway info
    // TODO: this is actually quite bad. we shouldn't be persisting gateway info here since we did not have persisted
    // the shared key yet. this will only happen when we start the base client itself.
    // but unfortunately, we can't do much more until we do a bit more refactoring.
    client_store.store_gateway_config(&config).await?;

    console_log!("stored: {:?}", config);

    Ok(config)
}

pub(crate) async fn gateway_from_topology<R: Rng + CryptoRng>(
    rng: &mut R,
    explicit_gateway: Option<IdentityKeyRef<'_>>,
    topology: &NymTopology,
    client_store: &ClientStorage,
) -> Result<GatewayEndpointConfig, WasmClientError> {
    let existing_gateway_config = client_store.read_gateway_config().await?;
    console_log!("loaded: {:?}", existing_gateway_config);

    let new_gateway: GatewayEndpointConfig = if let Some(provided) = explicit_gateway {
        if let Some(existing) = existing_gateway_config {
            // we have stored gateway info and explicitly provided identity key
            //
            // check if they match, otherwise return an error
            return if provided != existing.gateway_id {
                Err(WasmClientError::AlreadyRegistered {
                    gateway_config: existing,
                })
            } else {
                Ok(existing)
            };
        } else {
            // we have explicitly provided identity key and didn't have any prior stored data
            //
            // try to grab details from the topology
            let gateway_identity = identity::PublicKey::from_base58_string(provided)
                .map_err(|source| WasmClientError::InvalidGatewayIdentity { source })?;
            if let Some(gateway) = topology.get_gateway(&gateway_identity) {
                gateway.clone().into()
            } else {
                return Err(WasmClientError::NonExistentGateway {
                    gateway_identity: gateway_identity.to_base58_string(),
                });
            }
        }
    } else if let Some(existing) = existing_gateway_config {
        // we have stored data and didn't provide anything separately - use what's stored!
        return Ok(existing);
    } else {
        // we don't have anything stored nor we have provided anything
        //
        // just grab random gateway from our topology
        topology.random_gateway(rng)?.clone().into()
    };

    console_log!("storing: {:?}", new_gateway);
    client_store.store_gateway_config(&new_gateway).await?;
    Ok(new_gateway)
}
