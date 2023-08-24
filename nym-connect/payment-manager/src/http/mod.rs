// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cli::CliArgs;
use crate::client::Client;
use crate::state::{Config, State};
use crate::storage::Storage;
use anyhow::Result;
use nym_network_defaults::NymNetworkDetails;
use rocket::http::Method;
use rocket::{Ignite, Rocket, Route};
use rocket_cors::{AllowedHeaders, AllowedOrigins, Cors};
use rocket_okapi::okapi::openapi3::OpenApi;
use rocket_okapi::settings::OpenApiSettings;
use rocket_okapi::swagger_ui::make_swagger_ui;
use rocket_okapi::{mount_endpoints_and_merged_docs, openapi_get_routes_spec};

pub(crate) mod openapi;
pub(crate) mod routes;

pub(crate) fn routes(settings: &OpenApiSettings) -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![
        settings: routes::claim_payment,
    ]
}

fn setup_cors() -> Result<Cors> {
    let allowed_origins = AllowedOrigins::all();

    // You can also deserialize this
    let cors = rocket_cors::CorsOptions {
        allowed_origins,
        allowed_methods: vec![Method::Post].into_iter().map(From::from).collect(),
        allowed_headers: AllowedHeaders::all(),
        allow_credentials: true,
        ..Default::default()
    }
    .to_cors()?;

    Ok(cors)
}

pub(crate) async fn setup_rocket(args: &CliArgs) -> Result<Rocket<Ignite>> {
    let openapi_settings = rocket_okapi::settings::OpenApiSettings::default();
    let mut rocket = rocket::build();
    let storage = Storage::init(&args.db_path).await?;

    mount_endpoints_and_merged_docs! {
        rocket,
        "/v1".to_owned(),
        openapi_settings,
        "/" => (vec![], openapi::custom_openapi_spec()),
        "" => routes(&openapi_settings),
    }

    let details = NymNetworkDetails::new_from_env();
    let client = Client::new(&details, &args.mnemonic)?;
    let config = Config::new(details.chain_details.mix_denom.base.clone());
    let rocket = rocket
        .manage(State::new(storage, client, config))
        .mount("/swagger", make_swagger_ui(&openapi::get_docs()))
        .attach(setup_cors()?);

    Ok(rocket.ignite().await?)
}
