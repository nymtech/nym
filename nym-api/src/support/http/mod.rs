// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::circulating_supply_api::cache::CirculatingSupplyCache;
use crate::node_status_api::{self, NodeStatusCache};
use crate::nym_contract_cache::cache::NymContractCache;
use crate::support::config::Config;
use crate::support::{nyxd, storage};
use crate::{circulating_supply_api, nym_contract_cache};
use anyhow::Result;
use rocket::fairing::AdHoc;
use rocket::http::Method;
use rocket::{Ignite, Rocket};
use rocket_cors::{AllowedHeaders, AllowedOrigins, Cors};
use rocket_okapi::mount_endpoints_and_merged_docs;
use rocket_okapi::swagger_ui::make_swagger_ui;
use std::sync::Arc;
use tokio::sync::Notify;

#[cfg(feature = "coconut")]
use crate::coconut::{self, comm::QueryCommunicationChannel, InternalSignRequest};

pub(crate) mod openapi;

pub(crate) async fn setup_rocket(
    config: &Config,
    mix_denom: String,
    liftoff_notify: Arc<Notify>,
    _nyxd_client: nyxd::Client,
    #[cfg(feature = "coconut")] coconut_keypair: coconut::keypair::KeyPair,
) -> anyhow::Result<Rocket<Ignite>> {
    let openapi_settings = rocket_okapi::settings::OpenApiSettings::default();
    let mut rocket = rocket::build();

    mount_endpoints_and_merged_docs! {
        rocket,
        "/v1".to_owned(),
        openapi_settings,
        "/" => (vec![], openapi::custom_openapi_spec()),
        "" => circulating_supply_api::circulating_supply_routes(&openapi_settings),
        "" => nym_contract_cache::nym_contract_cache_routes(&openapi_settings),
        "/status" => node_status_api::node_status_routes(&openapi_settings, config.get_network_monitor_enabled()),
    }

    let rocket = rocket
        .mount("/swagger", make_swagger_ui(&openapi::get_docs()))
        .attach(setup_cors()?)
        .attach(setup_liftoff_notify(liftoff_notify))
        .attach(NymContractCache::stage())
        .attach(NodeStatusCache::stage())
        .attach(CirculatingSupplyCache::stage(mix_denom.clone()));

    // This is not a very nice approach. A lazy value would be more suitable, but that's still
    // a nightly feature: https://github.com/rust-lang/rust/issues/74465
    let storage = if cfg!(feature = "coconut") || config.get_network_monitor_enabled() {
        Some(storage::NymApiStorage::init(config.get_node_status_api_database_path()).await?)
    } else {
        None
    };

    #[cfg(feature = "coconut")]
    let rocket = if config.get_coconut_signer_enabled() {
        let comm_channel = QueryCommunicationChannel::new(_nyxd_client.clone());
        rocket.attach(InternalSignRequest::stage(
            _nyxd_client.clone(),
            mix_denom,
            coconut_keypair,
            comm_channel,
            storage.clone().unwrap(),
        ))
    } else {
        rocket
    };

    // see if we should start up network monitor
    let rocket = if config.get_network_monitor_enabled() {
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

fn setup_liftoff_notify(notify: Arc<Notify>) -> AdHoc {
    AdHoc::on_liftoff("Liftoff notifier", |_| {
        Box::pin(async move { notify.notify_one() })
    })
}
