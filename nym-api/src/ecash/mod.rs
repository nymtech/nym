// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use self::comm::APICommunicationChannel;
use crate::ecash::client::Client as LocalClient;
use crate::ecash::state::State;
use crate::support::storage::NymApiStorage;
use keys::KeyPair;
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
pub(crate) mod keys;
pub(crate) mod state;
pub(crate) mod storage;
#[cfg(test)]
pub(crate) mod tests;

// equivalent of 10nym
pub(crate) const MINIMUM_BALANCE: u128 = 10_000000;

pub fn stage<C, D>(
    client: C,
    identity_keypair: identity::KeyPair,
    key_pair: KeyPair,
    comm_channel: D,
    storage: NymApiStorage,
) -> AdHoc
where
    C: LocalClient + Send + Sync + 'static,
    D: APICommunicationChannel + Send + Sync + 'static,
{
    let state = State::new(client, identity_keypair, key_pair, comm_channel, storage);
    AdHoc::on_ignite("Internal Sign Request Stage", |rocket| async {
        rocket.manage(state).mount(
            // this format! is so ugly...
            format!("/{NYM_API_VERSION}/{COCONUT_ROUTES}/{BANDWIDTH}"),
            routes![
                api_routes::get_current_free_pass_nonce,
                api_routes::post_free_pass,
                api_routes::post_blind_sign,
                api_routes::verify_online_credential,
                api_routes::verify_offline_credential,
                api_routes::expiration_date_signatures,
                api_routes::expiration_date_signatures_timestamp,
                api_routes::coin_indices_signatures,
                api_routes::spent_credentials_filter,
                api_routes::epoch_credentials,
                api_routes::issued_credential,
                api_routes::issued_credentials,
            ],
        )
    })
}
