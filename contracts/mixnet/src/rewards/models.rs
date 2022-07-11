// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct RewardPoolChange {
    /// Indicates amount that shall get moved from the reward pool to the staking supply
    /// upon the current interval finishing.
    pub removed: Decimal,

    // this will be used once coconut credentials are in use;
    /// Indicates amount that shall get added to the both reward pool and not touch the staking supply
    /// upon the current interval finishing.
    #[allow(unused)]
    pub added: Decimal,
}
