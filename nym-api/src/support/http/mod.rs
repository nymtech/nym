// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::circulating_supply_api::cache::CirculatingSupplyCache;
use crate::ecash::client::Client;
use crate::ecash::state::EcashState;
use crate::ecash::{self, comm::QueryCommunicationChannel};
use crate::network::models::NetworkDetails;
use crate::network::network_routes;
use crate::node_describe_cache::DescribedNodes;
use crate::node_status_api::routes_deprecated::unstable;
use crate::node_status_api::{self, NodeStatusCache};
use crate::nym_contract_cache::cache::NymContractCache;
use crate::nym_nodes::{nym_node_routes_deprecated, nym_node_routes_next};
use crate::status::{api_status_routes, ApiStatusState, SignerState};
use crate::support::caching::cache::SharedCache;
use crate::support::config::Config;
use crate::support::{nyxd, storage};
use crate::{circulating_supply_api, nym_contract_cache};
use anyhow::{bail, Context, Result};
use nym_crypto::asymmetric::identity;
use nym_validator_client::nyxd::Coin;
use rocket::{Ignite, Rocket};
use rocket_cors::{AllowedHeaders, AllowedOrigins, Cors};
use rocket_okapi::mount_endpoints_and_merged_docs;
use rocket_okapi::swagger_ui::make_swagger_ui;

pub(crate) mod helpers;
pub(crate) mod openapi;

pub(crate) async fn setup_rocket(
    config: &Config,
    network_details: NetworkDetails,
    nyxd_client: nyxd::Client,
    identity_keypair: identity::KeyPair,
    coconut_keypair: ecash::keys::KeyPair,
) -> anyhow::Result<Rocket<Ignite>> {
    let openapi_settings = rocket_okapi::settings::OpenApiSettings::default();
    let mut rocket = rocket::build();

    let mix_denom = network_details.network.chain_details.mix_denom.base.clone();

    mount_endpoints_and_merged_docs! {
        rocket,
        "/v1".to_owned(),
        openapi_settings,
        "/" => (vec![], openapi::custom_openapi_spec()),
        "" => circulating_supply_api::circulating_supply_routes(&openapi_settings),
        "" => nym_contract_cache::nym_contract_cache_routes(&openapi_settings),
        "/status" => node_status_api::node_status_routes(&openapi_settings, config.network_monitor.enabled),
        "/network" => network_routes(&openapi_settings),
        "/api-status" => api_status_routes(&openapi_settings),
        "/ecash" => ecash::routes_open_api(&openapi_settings, config.coconut_signer.enabled),
        "" => nym_node_routes_deprecated(&openapi_settings),

        // => when we move those routes, we'll need to add a redirection for backwards compatibility
        "/unstable/nym-nodes" => nym_node_routes_next(&openapi_settings)
    }

    let storage =
        storage::NymApiStorage::init(&config.node_status_api.storage_paths.database_path).await?;

    let rocket = rocket
        .manage(network_details)
        .manage(SharedCache::<DescribedNodes>::new())
        .mount("/swagger", make_swagger_ui(&openapi::get_docs()))
        .attach(setup_rocket_cors()?)
        .attach(NymContractCache::stage())
        .attach(NodeStatusCache::stage())
        .attach(CirculatingSupplyCache::stage(mix_denom.clone()))
        .attach(storage::NymApiStorage::stage(storage.clone()))
        .manage(unstable::NodeInfoCache::default());

    let mut status_state = ApiStatusState::new();

    let rocket = if config.coconut_signer.enabled {
        // make sure we have some tokens to cover multisig fees
        let balance = nyxd_client.balance(&mix_denom).await?;
        if balance.amount < ecash::MINIMUM_BALANCE {
            let address = nyxd_client.address().await;
            let min = Coin::new(ecash::MINIMUM_BALANCE, mix_denom);
            bail!("the account ({address}) doesn't have enough funds to cover verification fees. it has {balance} while it needs at least {min}")
        }

        let cosmos_address = nyxd_client.address().await.to_string();
        let announce_address = config
            .coconut_signer
            .announce_address
            .clone()
            .map(|u| u.to_string())
            .unwrap_or_default();
        status_state.add_zk_nym_signer(SignerState {
            cosmos_address,
            identity: identity_keypair.public_key().to_base58_string(),
            announce_address,
            coconut_keypair: coconut_keypair.clone(),
        });

        let ecash_contract = nyxd_client
            .get_ecash_contract_address()
            .await
            .context("e-cash contract address is required to setup the zk-nym signer")?;

        let comm_channel = QueryCommunicationChannel::new(nyxd_client.clone());

        let ecash_state = EcashState::new(
            ecash_contract,
            nyxd_client.clone(),
            identity_keypair,
            coconut_keypair,
            comm_channel,
            storage.clone(),
        )
        .await?;

        rocket.manage(ecash_state)
    } else {
        rocket
    };

    Ok(rocket.manage(status_state).ignite().await?)
}

fn setup_rocket_cors() -> Result<Cors> {
    let allowed_origins = AllowedOrigins::all();

    // You can also deserialize this
    let cors = rocket_cors::CorsOptions {
        allowed_origins,
        allowed_methods: vec![rocket::http::Method::Post, rocket::http::Method::Get]
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
