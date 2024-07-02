// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash;
use nym_bin_common::bin_info;
use nym_bin_common::build_information::BinaryBuildInformation;
use okapi::openapi3::OpenApi;
use rocket::Route;
use rocket_okapi::openapi_get_routes_spec;
use rocket_okapi::settings::OpenApiSettings;
use tokio::time::Instant;

pub(crate) mod routes;

pub(crate) struct ApiStatusState {
    startup_time: Instant,
    build_information: BinaryBuildInformation,
    signer_information: Option<SignerState>,
}

pub(crate) struct SignerState {
    // static information
    pub cosmos_address: String,

    pub identity: String,

    pub announce_address: String,

    pub(crate) coconut_keypair: ecash::keys::KeyPair,
}

impl ApiStatusState {
    pub fn new() -> Self {
        ApiStatusState {
            startup_time: Instant::now(),
            build_information: bin_info!(),
            signer_information: None,
        }
    }

    pub fn add_zk_nym_signer(&mut self, signer_information: SignerState) {
        self.signer_information = Some(signer_information)
    }
}

pub(crate) fn api_status_routes(settings: &OpenApiSettings) -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![
        settings:
        routes::health,
        routes::build_information,
        routes::signer_information
    ]
}
