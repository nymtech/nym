// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::NodeId;
use cosmwasm_std::{Addr, Coin};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PendingEpochEvent {
    // can't just pass the `Delegation` struct here as it's impossible to determine
    // `cumulative_reward_ratio` ahead of time
    Delegate {
        owner: Addr,
        mix_id: NodeId,
        amount: Coin,
        proxy: Option<Addr>,
    },
    Undelegate {
        owner: Addr,
        mix_id: NodeId,
        proxy: Option<Addr>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PendingIntervalEvent {
    ChangeMixCostParams {
        //
    },
}
