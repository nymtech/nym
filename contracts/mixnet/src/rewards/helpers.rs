// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// use super::storage;
// use crate::error::ContractError;
// use cosmwasm_std::Storage;
// use mixnet_contract_common::{IdentityKey, RewardingResult, RewardingStatus};
//
// pub(crate) fn update_rewarding_status(
//     storage: &mut dyn Storage,
//     interval_id: u32,
//     mix_identity: IdentityKey,
//     rewarding_result: RewardingResult,
// ) -> Result<(), ContractError> {
//     storage::REWARDING_STATUS.save(
//         storage,
//         (interval_id, mix_identity),
//         &RewardingStatus::Complete(rewarding_result),
//     )?;
//
//     Ok(())
// }
