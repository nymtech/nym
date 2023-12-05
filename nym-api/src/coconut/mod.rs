// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use self::comm::APICommunicationChannel;
use crate::coconut::client::Client as LocalClient;
use crate::coconut::state::State;
use crate::support::storage::NymApiStorage;
use keypair::KeyPair;
use nym_config::defaults::NYM_API_VERSION;
use nym_crypto::asymmetric::identity;
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
pub(crate) mod storage;
#[cfg(test)]
pub(crate) mod tests;

pub fn stage<C, D>(
    client: C,
    mix_denom: String,
    identity_keypair: identity::KeyPair,
    key_pair: KeyPair,
    comm_channel: D,
    storage: NymApiStorage,
) -> AdHoc
where
    C: LocalClient + Send + Sync + 'static,
    D: APICommunicationChannel + Send + Sync + 'static,
{
    let state = State::new(
        client,
        mix_denom,
        identity_keypair,
        key_pair,
        comm_channel,
        storage,
    );
    AdHoc::on_ignite("Internal Sign Request Stage", |rocket| async {
        rocket.manage(state).mount(
            // this format! is so ugly...
            format!("/{NYM_API_VERSION}/{COCONUT_ROUTES}/{BANDWIDTH}"),
            routes![
                api_routes::post_blind_sign,
                api_routes::verify_bandwidth_credential,
                api_routes::epoch_credentials,
                api_routes::issued_credential,
                api_routes::issued_credentials,
            ],
        )
    })
}
