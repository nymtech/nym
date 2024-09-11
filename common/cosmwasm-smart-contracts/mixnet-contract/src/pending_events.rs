// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mixnode::MixNodeCostParams;
use crate::reward_params::IntervalRewardingParamsUpdate;
use crate::{BlockHeight, MixId};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin};

pub type EpochEventId = u32;
pub type IntervalEventId = u32;

/// A request made at some point in the current epoch that's going to get resolved once the epoch rolls over.
#[cw_serde]
pub struct PendingEpochEvent {
    /// The unique id associated with the event.
    pub id: EpochEventId,

    /// The underlying event details, containing its type and information on how it should get resolved.
    pub event: PendingEpochEventData,
}

/// Details of a particular pending epoch event.
#[cw_serde]
pub struct PendingEpochEventData {
    /// The block height at which the request has been made.
    pub created_at: BlockHeight,

    /// The underlying event data, containing its concrete type and information on how it should get resolved.
    pub kind: PendingEpochEventKind,
}

/// Enum encompassing all possible epoch events.
#[cw_serde]
pub enum PendingEpochEventKind {
    // can't just pass the `Delegation` struct here as it's impossible to determine
    // `cumulative_reward_ratio` ahead of time
    /// Request to create a delegation towards particular mixnode.
    /// Note that if such delegation already exists, it will get updated with the provided token amount.
    #[serde(alias = "Delegate")]
    #[non_exhaustive]
    Delegate {
        /// The address of the owner of the delegation.
        owner: Addr,

        /// The id of the mixnode used for the delegation.
        mix_id: MixId,

        /// The amount of tokens to use for the delegation.
        amount: Coin,

        /// Entity who made the delegation on behalf of the owner.
        /// If present, it's most likely the address of the vesting contract.
        proxy: Option<Addr>,
    },

    /// Request to remove delegation from particular mixnode.
    #[serde(alias = "Undelegate")]
    #[non_exhaustive]
    Undelegate {
        /// The address of the owner of the delegation.
        owner: Addr,

        /// The id of the mixnode used for the delegation.
        mix_id: MixId,

        /// Entity who made the delegation on behalf of the owner.
        /// If present, it's most likely the address of the vesting contract.
        proxy: Option<Addr>,
    },

    /// Request to pledge more tokens (by the node operator) towards its node.
    #[serde(alias = "PledgeMore")]
    PledgeMore {
        /// The id of the mixnode that will have its pledge updated.
        mix_id: MixId,

        /// The amount of additional tokens to use by the pledge.
        amount: Coin,
    },

    /// Request to decrease amount of pledged tokens (by the node operator) from its node.
    #[serde(alias = "DecreasePledge")]
    DecreasePledge {
        /// The id of the mixnode that will have its pledge updated.
        mix_id: MixId,

        /// The amount of tokens that should be removed from the pledge.
        decrease_by: Coin,
    },

    /// Request to unbond a mixnode and completely remove it from the network.
    #[serde(alias = "UnbondMixnode")]
    UnbondMixnode {
        /// The id of the mixnode that will get unbonded.
        mix_id: MixId,
    },

    /// Request to update the current size of the active set.
    #[serde(alias = "UpdateActiveSetSize")]
    UpdateActiveSetSize {
        /// The new desired size of the active set.
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

    pub fn new_delegate(owner: Addr, mix_id: MixId, amount: Coin) -> Self {
        PendingEpochEventKind::Delegate {
            owner,
            mix_id,
            amount,
            proxy: None,
        }
    }

    pub fn new_undelegate(owner: Addr, mix_id: MixId) -> Self {
        PendingEpochEventKind::Undelegate {
            owner,
            mix_id,
            proxy: None,
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

/// A request made at some point in the current interval that's going to get resolved once the interval rolls over.
#[cw_serde]
pub struct PendingIntervalEvent {
    /// The unique id associated with the event.
    pub id: IntervalEventId,

    /// The underlying event details, containing its type and information on how it should get resolved.
    pub event: PendingIntervalEventData,
}

/// Details of a particular pending interval event.
#[cw_serde]
pub struct PendingIntervalEventData {
    /// The block height at which the request has been made.
    pub created_at: BlockHeight,

    /// The underlying event data, containing its concrete type and information on how it should get resolved.
    pub kind: PendingIntervalEventKind,
}

/// Enum encompassing all possible interval events.
#[cw_serde]
pub enum PendingIntervalEventKind {
    /// Request to update cost parameters of given mixnode.
    #[serde(alias = "ChangeMixCostParams")]
    ChangeMixCostParams {
        /// The id of the mixnode that will have its cost parameters updated.
        mix_id: MixId,

        /// The new updated cost function of this mixnode.
        new_costs: MixNodeCostParams,
    },

    /// Request to update the underlying rewarding parameters used by the system
    #[serde(alias = "UpdateRewardingParams")]
    UpdateRewardingParams {
        /// The detailed specification of the update.
        update: IntervalRewardingParamsUpdate,
    },

    /// Request to change the next interval configuration.
    #[serde(alias = "UpdateIntervalConfig")]
    UpdateIntervalConfig {
        /// The new number of epochs in intervals.
        epochs_in_interval: u32,

        /// The new epoch duration.
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

/// Response containing all currently pending epoch events that will be resolved once the current epoch finishes.
#[cw_serde]
pub struct PendingEpochEventsResponse {
    /// Amount of seconds until the events would be eligible to be resolved.
    /// It's equivalent to the time until the current epoch finishes.
    pub seconds_until_executable: i64,

    /// The currently pending events.
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

/// Response containing all currently pending interval events that will be resolved once the current interval finishes.
#[cw_serde]
pub struct PendingIntervalEventsResponse {
    /// Amount of seconds until the events would be eligible to be resolved.
    /// It's equivalent to the time until the current interval finishes.
    pub seconds_until_executable: i64,

    /// The currently pending events.
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

/// Response containing number of currently pending epoch and interval events.
#[cw_serde]
pub struct NumberOfPendingEventsResponse {
    /// The number of the currently pending epoch events.
    pub epoch_events: u32,

    /// The number of the currently pending epoch events.
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
