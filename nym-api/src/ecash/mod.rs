// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

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

// equivalent of 100nym
pub(crate) const MINIMUM_BALANCE: u128 = 100_000000;

// pub(crate) fn routes_open_api(settings: &OpenApiSettings, enabled: bool) -> (Vec<Route>, OpenApi) {
//     if enabled {
//         openapi_get_routes_spec![
//             settings:
//             api_routes::partial_signing::post_blind_sign,
//             api_routes::partial_signing::partial_expiration_date_signatures,
//             api_routes::partial_signing::partial_coin_indices_signatures,
//             api_routes::spending::verify_ticket,
//             api_routes::spending::batch_redeem_tickets,
//             api_routes::spending::double_spending_filter_v1,
//             api_routes::issued::epoch_credentials,
//             api_routes::issued::issued_credential,
//             api_routes::issued::issued_credentials,
//             api_routes::aggregation::master_verification_key,
//             api_routes::aggregation::coin_indices_signatures,
//             api_routes::aggregation::expiration_date_signatures
//         ]
//     } else {
//         openapi_get_routes_spec![
//             settings:
//         ]
//     }
// }
