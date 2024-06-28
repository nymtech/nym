// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

// use crate::ecash::error::Result;
// use crate::ecash::state::State;
// use log::trace;
// use rocket::serde::json::Json;
// use rocket::State as RocketState;
//
// // routes with globally aggregated keys, signatures, etc.
//
// pub async fn aggregated_verification_key(state: &RocketState<State>) -> Result<Json<()>> {
//     trace!("aggregated_verification_key request");
//
//     // see if we're not in the middle of new dkg
//     state.ensure_dkg_not_in_progress().await?;
//     todo!()
// }
//
// pub async fn aggregated_expiration_date_signatures(state: &RocketState<State>) -> Result<Json<()>> {
//     trace!("aggregated_expiration_date_signatures request");
//
//     // see if we're not in the middle of new dkg
//     state.ensure_dkg_not_in_progress().await?;
//
//     todo!()
// }
//
// pub async fn aggregated_coin_indices_signatures(state: &RocketState<State>) -> Result<Json<()>> {
//     trace!("aggregated_coin_indices_signatures request");
//
//     // see if we're not in the middle of new dkg
//     state.ensure_dkg_not_in_progress().await?;
//
//     todo!()
// }
