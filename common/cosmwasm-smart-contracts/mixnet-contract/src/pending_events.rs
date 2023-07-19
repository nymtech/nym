// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mixnode::MixNodeCostParams;
use crate::reward_params::IntervalRewardingParamsUpdate;
use crate::{BlockHeight, MixId};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin};

pub type EpochEventId = u32;
pub type IntervalEventId = u32;

#[cw_serde]
pub struct PendingEpochEvent {
    pub id: EpochEventId,
    pub event: PendingEpochEventData,
}

#[cw_serde]
pub struct PendingEpochEventData {
    pub created_at: BlockHeight,
    pub kind: PendingEpochEventKind,
}

#[cw_serde]
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

#[cw_serde]
pub struct PendingIntervalEvent {
    pub id: IntervalEventId,
    pub event: PendingIntervalEventData,
}

#[cw_serde]
pub struct PendingIntervalEventData {
    pub created_at: BlockHeight,
    pub kind: PendingIntervalEventKind,
}

#[cw_serde]
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

#[cw_serde]
pub struct PendingEpochEventsResponse {
    pub seconds_until_executable: i64,
    pub events: Vec<PendingEpochEvent>,

    /// Field indicating paging information for the following queries if the caller wishes to get further entries.
    pub start_next_after: Option<u32>,
}

impl PendingEpochEventsResponse {
    pub fn new(
        seconds_until_executable: i64,
        events: Vec<PendingEpochEvent>,
        start_next_after: Option<u32>,
    ) -> Self {
        PendingEpochEventsResponse {
            seconds_until_executable,
            events,
            start_next_after,
        }
    }
}

#[cw_serde]
pub struct PendingIntervalEventsResponse {
    pub seconds_until_executable: i64,
    pub events: Vec<PendingIntervalEvent>,

    /// Field indicating paging information for the following queries if the caller wishes to get further entries.
    pub start_next_after: Option<u32>,
}

impl PendingIntervalEventsResponse {
    pub fn new(
        seconds_until_executable: i64,
        events: Vec<PendingIntervalEvent>,
        start_next_after: Option<u32>,
    ) -> Self {
        PendingIntervalEventsResponse {
            seconds_until_executable,
            events,
            start_next_after,
        }
    }
}

#[cw_serde]
pub struct PendingEpochEventResponse {
    pub event_id: EpochEventId,
    pub event: Option<PendingEpochEventData>,
}

impl PendingEpochEventResponse {
    pub fn new(event_id: EpochEventId, event: Option<PendingEpochEventData>) -> Self {
        PendingEpochEventResponse { event_id, event }
    }
}

#[cw_serde]
pub struct PendingIntervalEventResponse {
    pub event_id: IntervalEventId,
    pub event: Option<PendingIntervalEventData>,
}

impl PendingIntervalEventResponse {
    pub fn new(event_id: IntervalEventId, event: Option<PendingIntervalEventData>) -> Self {
        PendingIntervalEventResponse { event_id, event }
    }
}

#[cw_serde]
pub struct NumberOfPendingEventsResponse {
    pub epoch_events: u32,
    pub interval_events: u32,
}

impl NumberOfPendingEventsResponse {
    pub fn new(epoch_events: u32, interval_events: u32) -> Self {
        Self {
            epoch_events,
            interval_events,
        }
    }
}
