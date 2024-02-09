// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::circulating_supply_api::cache::CirculatingSupplyCache;
use crate::coconut::client::Client;
use crate::coconut::{self, comm::QueryCommunicationChannel};
use crate::network::models::NetworkDetails;
use crate::network::network_routes;
use crate::node_describe_cache::DescribedNodes;
use crate::node_status_api::{self, NodeStatusCache};
use crate::nym_contract_cache::cache::NymContractCache;
use crate::support::caching::cache::SharedCache;
use crate::support::config::Config;
use crate::support::{nyxd, storage};
use crate::{circulating_supply_api, nym_contract_cache, nym_nodes::nym_node_routes};
use anyhow::{bail, Result};
use nym_crypto::asymmetric::identity;
use nym_validator_client::nyxd::Coin;
use rocket::http::Method;
use rocket::{Ignite, Rocket};
use rocket_cors::{AllowedHeaders, AllowedOrigins, Cors};
use rocket_okapi::mount_endpoints_and_merged_docs;
use rocket_okapi::swagger_ui::make_swagger_ui;

pub(crate) mod openapi;

pub(crate) async fn setup_rocket(
    config: &Config,
    network_details: NetworkDetails,
    _nyxd_client: nyxd::Client,
    identity_keypair: identity::KeyPair,
    coconut_keypair: coconut::keys::KeyPair,
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
        "" => nym_node_routes(&openapi_settings),
    }

    let rocket = rocket
        .manage(network_details)
        .manage(SharedCache::<DescribedNodes>::new())
        .mount("/swagger", make_swagger_ui(&openapi::get_docs()))
        .attach(setup_cors()?)
        .attach(NymContractCache::stage())
        .attach(NodeStatusCache::stage())
        .attach(CirculatingSupplyCache::stage(mix_denom.clone()));

    // This is not a very nice approach. A lazy value would be more suitable, but that's still
    // a nightly feature: https://github.com/rust-lang/rust/issues/74465
    let storage = if config.coconut_signer.enabled || config.network_monitor.enabled {
        Some(
            storage::NymApiStorage::init(&config.node_status_api.storage_paths.database_path)
                .await?,
        )
    } else {
        None
    };

    let rocket = if config.coconut_signer.enabled {
        // make sure we have some tokens to cover multisig fees
        let balance = _nyxd_client.balance(&mix_denom).await?;
        if balance.amount < coconut::MINIMUM_BALANCE {
            let address = _nyxd_client.address().await;
            let min = Coin::new(coconut::MINIMUM_BALANCE, mix_denom);
            bail!("the account ({address}) doesn't have enough funds to cover verification fees. it has {balance} while it needs at least {min}")
        }

        let comm_channel = QueryCommunicationChannel::new(_nyxd_client.clone());
        rocket.attach(coconut::stage(
            _nyxd_client.clone(),
            mix_denom,
            identity_keypair,
            coconut_keypair,
            comm_channel,
            storage.clone().unwrap(),
        ))
    } else {
        rocket
    };

    // see if we should start up network monitor
    let rocket = if config.network_monitor.enabled {
        rocket.attach(storage::NymApiStorage::stage(storage.unwrap()))
    } else {
        rocket
    };

    Ok(rocket.ignite().await?)
}

fn setup_cors() -> Result<Cors> {
    let allowed_origins = AllowedOrigins::all();

    // You can also deserialize this
    let cors = rocket_cors::CorsOptions {
        allowed_origins,
        allowed_methods: vec![Method::Post, Method::Get]
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
