// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Uint128;

// to be determined whether those should be constants or exist as contract state
pub(crate) const MINIMUM_DEPOSIT: Uint128 = Uint128::new(1_000_000_000);
// Wait time for the verification to take place
pub(crate) const BLOCK_TIME_FOR_VERIFICATION_SECS: u64 = 60;
