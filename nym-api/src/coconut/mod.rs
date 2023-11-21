// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use self::comm::APICommunicationChannel;
use crate::coconut::client::Client as LocalClient;
use crate::coconut::state::State;
use crate::support::storage::NymApiStorage;
use getset::{CopyGetters, Getters};
use keypair::KeyPair;
use nym_coconut_interface::{Attribute, BlindSignRequest};
use nym_config::defaults::NYM_API_VERSION;
use nym_validator_client::nym_api::routes::{BANDWIDTH, COCONUT_ROUTES};
use rocket::fairing::AdHoc;

pub(crate) mod api_routes;
pub(crate) mod client;
pub(crate) mod comm;
mod deposit;
pub(crate) mod dkg;
pub(crate) mod error;
pub(crate) mod helpers;
pub(crate) mod keypair;
pub(crate) mod state;
#[cfg(test)]
pub(crate) mod tests;
pub(crate) mod types;

#[derive(Getters, CopyGetters, Debug)]
pub(crate) struct InternalSignRequest {
    // Total number of parameters to generate for
    #[getset(get_copy)]
    total_params: u32,

    #[getset(get)]
    public_attributes: Vec<Attribute>,

    #[getset(get)]
    blind_sign_request: BlindSignRequest,
}

impl InternalSignRequest {
    pub fn new(
        total_params: u32,
        public_attributes: Vec<Attribute>,
        blind_sign_request: BlindSignRequest,
    ) -> InternalSignRequest {
        InternalSignRequest {
            total_params,
            public_attributes,
            blind_sign_request,
        }
    }

    pub fn stage<C, D>(
        client: C,
        mix_denom: String,
        key_pair: KeyPair,
        comm_channel: D,
        storage: NymApiStorage,
    ) -> AdHoc
    where
        C: LocalClient + Send + Sync + 'static,
        D: APICommunicationChannel + Send + Sync + 'static,
    {
        let state = State::new(client, mix_denom, key_pair, comm_channel, storage);
        AdHoc::on_ignite("Internal Sign Request Stage", |rocket| async {
            rocket.manage(state).mount(
                // this format! is so ugly...
                format!("/{}/{}/{}", NYM_API_VERSION, COCONUT_ROUTES, BANDWIDTH),
                routes![
                    api_routes::post_blind_sign,
                    api_routes::verify_bandwidth_credential
                ],
            )
        })
    }
}
