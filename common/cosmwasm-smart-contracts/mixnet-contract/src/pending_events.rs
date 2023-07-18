// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mixnode::MixNodeCostParams;
use crate::reward_params::IntervalRewardingParamsUpdate;
use crate::{BlockHeight, EpochEventId, IntervalEventId, MixId};
use cosmwasm_std::{Addr, Coin};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PendingEpochEvent {
    pub id: EpochEventId,
    pub event: PendingEpochEventData,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PendingEpochEventData {
    pub created_at: BlockHeight,
    pub kind: PendingEpochEventKind,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PendingEpochEventKind {
    // can't just pass the `Delegation` struct here as it's impossible to determine
    // `cumulative_reward_ratio` ahead of time
    Delegate {
        owner: Addr,
        mix_id: MixId,
        amount: Coin,
        proxy: Option<Addr>,
    },
    Undelegate {
        owner: Addr,
        mix_id: MixId,
        proxy: Option<Addr>,
    },
    PledgeMore {
        mix_id: MixId,
        amount: Coin,
    },
    DecreasePledge {
        mix_id: MixId,
        decrease_by: Coin,
    },
    UnbondMixnode {
        mix_id: MixId,
    },
    UpdateActiveSetSize {
        new_size: u32,
    },
}

impl PendingEpochEventKind {
    pub fn attach_source_height(self, created_at: BlockHeight) -> PendingEpochEventData {
        PendingEpochEventData {
            created_at,
            kind: self,
        }
    }
}

impl From<(EpochEventId, PendingEpochEventData)> for PendingEpochEvent {
    fn from(data: (EpochEventId, PendingEpochEventData)) -> Self {
        PendingEpochEvent {
            id: data.0,
            event: data.1,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PendingIntervalEvent {
    pub id: IntervalEventId,
    pub event: PendingIntervalEventData,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PendingIntervalEventData {
    pub created_at: BlockHeight,
    pub kind: PendingIntervalEventKind,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PendingIntervalEventKind {
    ChangeMixCostParams {
        mix_id: MixId,
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

impl PendingIntervalEventKind {
    pub fn attach_source_height(self, created_at: BlockHeight) -> PendingIntervalEventData {
        PendingIntervalEventData {
            created_at,
            kind: self,
        }
    }
}

impl From<(IntervalEventId, PendingIntervalEventData)> for PendingIntervalEvent {
    fn from(data: (IntervalEventId, PendingIntervalEventData)) -> Self {
        PendingIntervalEvent {
            id: data.0,
            event: data.1,
        }
    }
}
