// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mixnode::MixNodeCostParams;
use crate::reward_params::IntervalRewardingParamsUpdate;
use crate::{EpochEventId, IntervalEventId, NodeId};
use cosmwasm_std::{Addr, Coin};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PendingEpochEvent {
    pub id: EpochEventId,
    pub event: PendingEpochEventData,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum PendingEpochEventData {
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
    UnbondMixnode {
        mix_id: NodeId,
    },
    UpdateActiveSetSize {
        new_size: u32,
    },
}

impl From<(EpochEventId, PendingEpochEventData)> for PendingEpochEvent {
    fn from(data: (EpochEventId, PendingEpochEventData)) -> Self {
        PendingEpochEvent {
            id: data.0,
            event: data.1,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PendingIntervalEvent {
    pub id: EpochEventId,
    pub event: PendingIntervalEventData,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum PendingIntervalEventData {
    ChangeMixCostParams {
        mix: NodeId,
        new_costs: MixNodeCostParams,
    },

    UpdateRewardingParams {
        update: IntervalRewardingParamsUpdate,
    },
    UpdateIntervalConfig {
        epochs_in_interval: u32,
        epoch_duration_secs: u64,
    },
}

impl From<(IntervalEventId, PendingIntervalEventData)> for PendingIntervalEvent {
    fn from(data: (IntervalEventId, PendingIntervalEventData)) -> Self {
        PendingIntervalEvent {
            id: data.0,
            event: data.1,
        }
    }
}
