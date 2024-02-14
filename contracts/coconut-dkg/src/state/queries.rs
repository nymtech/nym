// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::state::storage::STATE;
use cosmwasm_std::{StdResult, Storage};
use nym_coconut_dkg_common::types::State;

pub(crate) fn query_state(storage: &dyn Storage) -> StdResult<State> {
    STATE.load(storage)
}
