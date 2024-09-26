// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::currency::{DecCoin, RegisteredCoins};
use crate::error::TypesError;
use crate::mixnode::NodeCostParams;
use nym_mixnet_contract_common::{
    BlockHeight, EpochEventId, IntervalEventId, IntervalRewardingParamsUpdate, NodeId,
    PendingEpochEvent as MixnetContractPendingEpochEvent,
    PendingEpochEventKind as MixnetContractPendingEpochEventKind,
    PendingIntervalEvent as MixnetContractPendingIntervalEvent,
    PendingIntervalEventKind as MixnetContractPendingIntervalEventKind,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/PendingEpochEvent.ts"
    )
)]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, JsonSchema)]
pub struct PendingEpochEvent {
    pub id: EpochEventId,
    pub created_at: BlockHeight,
    pub event: PendingEpochEventData,
}

impl PendingEpochEvent {
    pub fn try_from_mixnet_contract(
        pending_event: MixnetContractPendingEpochEvent,
        reg: &RegisteredCoins,
    ) -> Result<Self, TypesError> {
        Ok(PendingEpochEvent {
            id: pending_event.id,
            created_at: pending_event.event.created_at,
            event: PendingEpochEventData::try_from_mixnet_contract(pending_event.event.kind, reg)?,
        })
    }
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/PendingEpochEventData.ts"
    )
)]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, JsonSchema)]
pub enum PendingEpochEventData {
    Delegate {
        owner: String,
        mix_id: NodeId,
        amount: DecCoin,
        proxy: Option<String>,
    },
    Undelegate {
        owner: String,
        mix_id: NodeId,
        proxy: Option<String>,
    },
    PledgeMore {
        mix_id: NodeId,
        amount: DecCoin,
    },
    DecreasePledge {
        mix_id: NodeId,
        decrease_by: DecCoin,
    },
    UnbondMixnode {
        mix_id: NodeId,
    },
    UpdateActiveSetSize {
        new_size: u32,
    },
}

impl PendingEpochEventData {
    pub fn try_from_mixnet_contract(
        pending_event: MixnetContractPendingEpochEventKind,
        reg: &RegisteredCoins,
    ) -> Result<Self, TypesError> {
        match pending_event {
            MixnetContractPendingEpochEventKind::Delegate {
                owner,
                node_id: mix_id,
                amount,
                proxy,
                ..
            } => Ok(PendingEpochEventData::Delegate {
                owner: owner.into_string(),
                mix_id,
                amount: reg.attempt_convert_to_display_dec_coin(amount.into())?,
                proxy: proxy.map(|p| p.into_string()),
            }),
            MixnetContractPendingEpochEventKind::Undelegate {
                owner,
                node_id: mix_id,
                proxy,
                ..
            } => Ok(PendingEpochEventData::Undelegate {
                owner: owner.into_string(),
                mix_id,
                proxy: proxy.map(|p| p.into_string()),
            }),
            MixnetContractPendingEpochEventKind::MixnodePledgeMore { mix_id, amount } => {
                Ok(PendingEpochEventData::PledgeMore {
                    mix_id,
                    amount: reg.attempt_convert_to_display_dec_coin(amount.into())?,
                })
            }
            MixnetContractPendingEpochEventKind::MixnodeDecreasePledge {
                mix_id,
                decrease_by,
            } => Ok(PendingEpochEventData::DecreasePledge {
                mix_id,
                decrease_by: reg.attempt_convert_to_display_dec_coin(decrease_by.into())?,
            }),
            MixnetContractPendingEpochEventKind::UnbondMixnode { mix_id } => {
                Ok(PendingEpochEventData::UnbondMixnode { mix_id })
            }
            _ => todo!(),
        }
    }
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/PendingIntervalEvent.ts"
    )
)]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, JsonSchema)]
pub struct PendingIntervalEvent {
    pub id: IntervalEventId,
    pub created_at: BlockHeight,
    pub event: PendingIntervalEventData,
}

impl PendingIntervalEvent {
    pub fn try_from_mixnet_contract(
        pending_event: MixnetContractPendingIntervalEvent,
        reg: &RegisteredCoins,
    ) -> Result<Self, TypesError> {
        Ok(PendingIntervalEvent {
            id: pending_event.id,
            created_at: pending_event.event.created_at,
            event: PendingIntervalEventData::try_from_mixnet_contract(
                pending_event.event.kind,
                reg,
            )?,
        })
    }
}

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/PendingIntervalEventData.ts"
    )
)]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, JsonSchema)]
pub enum PendingIntervalEventData {
    ChangeMixCostParams {
        mix_id: NodeId,
        new_costs: NodeCostParams,
    },

    UpdateRewardingParams {
        update: IntervalRewardingParamsUpdate,
    },
    UpdateIntervalConfig {
        epochs_in_interval: u32,
        epoch_duration_secs: u64,
    },
}

impl PendingIntervalEventData {
    pub fn try_from_mixnet_contract(
        pending_event: MixnetContractPendingIntervalEventKind,
        reg: &RegisteredCoins,
    ) -> Result<Self, TypesError> {
        match pending_event {
            MixnetContractPendingIntervalEventKind::ChangeMixCostParams { mix_id, new_costs } => {
                Ok(PendingIntervalEventData::ChangeMixCostParams {
                    mix_id,
                    new_costs: NodeCostParams::from_mixnet_contract_mixnode_cost_params(
                        new_costs, reg,
                    )?,
                })
            }
            MixnetContractPendingIntervalEventKind::UpdateRewardingParams { update } => {
                Ok(PendingIntervalEventData::UpdateRewardingParams { update })
            }
            MixnetContractPendingIntervalEventKind::UpdateIntervalConfig {
                epochs_in_interval,
                epoch_duration_secs,
            } => Ok(PendingIntervalEventData::UpdateIntervalConfig {
                epochs_in_interval,
                epoch_duration_secs,
            }),
            _ => todo!(),
        }
    }
}
