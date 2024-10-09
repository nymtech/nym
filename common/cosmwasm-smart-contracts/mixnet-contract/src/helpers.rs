// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::MixnetContractError;
use crate::mixnode::PendingMixNodeChanges;
use crate::{
    EpochEventId, IntervalEventId, MixNodeBond, MixNodeDetails, NodeId, NodeRewarding, NymNodeBond,
    NymNodeDetails, PendingNodeChanges,
};
use contracts_common::IdentityKeyRef;
use cosmwasm_std::{Coin, Decimal, StdError, StdResult, Uint128};

#[track_caller]
pub fn compare_decimals(a: Decimal, b: Decimal, epsilon: Option<Decimal>) {
    let epsilon = epsilon.unwrap_or_else(|| Decimal::from_ratio(1u128, 100_000_000u128));
    if a > b {
        assert!(a - b < epsilon, "{a} != {b}")
    } else {
        assert!(b - a < epsilon, "{a} != {b}")
    }
}

pub fn into_base_decimal(val: impl Into<Uint128>) -> StdResult<Decimal> {
    val.into_base_decimal()
}

pub trait IntoBaseDecimal {
    fn into_base_decimal(self) -> StdResult<Decimal>;
}

impl<T> IntoBaseDecimal for T
where
    T: Into<Uint128>,
{
    fn into_base_decimal(self) -> StdResult<Decimal> {
        let atomics = self.into();
        Decimal::from_atomics(atomics, 0).map_err(|_| StdError::GenericErr {
            msg: format!("Decimal range exceeded for {atomics} with 0 decimal places."),
        })
    }
}

pub trait NodeDetails {
    type Bond: NodeBond;
    type PendingChanges: PendingChanges;

    fn split(self) -> (Self::Bond, NodeRewarding, Self::PendingChanges);
    fn rewarding_info(&self) -> &NodeRewarding;
    fn bond_info(&self) -> &Self::Bond;
    fn pending_changes(&self) -> &Self::PendingChanges;
}

pub trait NodeBond {
    fn node_id(&self) -> NodeId;

    fn is_unbonding(&self) -> bool;

    fn identity(&self) -> IdentityKeyRef;

    fn original_pledge(&self) -> &Coin;

    fn ensure_bonded(&self) -> Result<(), MixnetContractError> {
        if self.is_unbonding() {
            return Err(MixnetContractError::NodeIsUnbonding {
                node_id: self.node_id(),
            });
        }
        Ok(())
    }
}

pub trait PendingChanges {
    fn pending_pledge_changes(&self) -> Option<EpochEventId>;

    fn pending_cost_params_changes(&self) -> Option<IntervalEventId>;

    fn ensure_no_pending_pledge_changes(&self) -> Result<(), MixnetContractError> {
        if let Some(pending_event_id) = self.pending_pledge_changes() {
            return Err(MixnetContractError::PendingPledgeChange { pending_event_id });
        }
        Ok(())
    }

    fn ensure_no_pending_params_changes(&self) -> Result<(), MixnetContractError> {
        if let Some(pending_event_id) = self.pending_cost_params_changes() {
            return Err(MixnetContractError::PendingParamsChange { pending_event_id });
        }
        Ok(())
    }
}

impl NodeDetails for MixNodeDetails {
    type Bond = MixNodeBond;
    type PendingChanges = PendingMixNodeChanges;

    fn split(self) -> (Self::Bond, NodeRewarding, Self::PendingChanges) {
        (
            self.bond_information,
            self.rewarding_details,
            self.pending_changes,
        )
    }

    fn rewarding_info(&self) -> &NodeRewarding {
        &self.rewarding_details
    }

    fn bond_info(&self) -> &Self::Bond {
        &self.bond_information
    }

    fn pending_changes(&self) -> &Self::PendingChanges {
        &self.pending_changes
    }
}

impl NodeBond for MixNodeBond {
    fn node_id(&self) -> NodeId {
        self.mix_id
    }

    fn is_unbonding(&self) -> bool {
        self.is_unbonding
    }

    fn identity(&self) -> IdentityKeyRef {
        self.identity()
    }

    fn original_pledge(&self) -> &Coin {
        self.original_pledge()
    }
}

impl PendingChanges for PendingMixNodeChanges {
    fn pending_pledge_changes(&self) -> Option<EpochEventId> {
        self.pledge_change
    }

    fn pending_cost_params_changes(&self) -> Option<IntervalEventId> {
        self.cost_params_change
    }
}

impl NodeDetails for NymNodeDetails {
    type Bond = NymNodeBond;
    type PendingChanges = PendingNodeChanges;

    fn split(self) -> (Self::Bond, NodeRewarding, Self::PendingChanges) {
        (
            self.bond_information,
            self.rewarding_details,
            self.pending_changes,
        )
    }

    fn rewarding_info(&self) -> &NodeRewarding {
        &self.rewarding_details
    }

    fn bond_info(&self) -> &Self::Bond {
        &self.bond_information
    }

    fn pending_changes(&self) -> &Self::PendingChanges {
        &self.pending_changes
    }
}

impl NodeBond for NymNodeBond {
    fn node_id(&self) -> NodeId {
        self.node_id
    }

    fn is_unbonding(&self) -> bool {
        self.is_unbonding
    }

    fn identity(&self) -> IdentityKeyRef {
        self.identity()
    }

    fn original_pledge(&self) -> &Coin {
        &self.original_pledge
    }
}

impl PendingChanges for PendingNodeChanges {
    fn pending_pledge_changes(&self) -> Option<EpochEventId> {
        self.pledge_change
    }

    fn pending_cost_params_changes(&self) -> Option<IntervalEventId> {
        self.cost_params_change
    }
}
