// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::MixnetContractError;
use crate::mixnode::PendingMixNodeChanges;
use crate::{
    EpochEventId, EpochId, Interval, IntervalEventId, MixNodeBond, MixNodeDetails, NodeId,
    NodeRewarding, NymNodeBond, NymNodeDetails, PendingNodeChanges, QueryMsg,
};
use cosmwasm_std::{
    Binary, Coin, CustomQuery, Decimal, QuerierWrapper, StdError, StdResult, Uint128, from_json,
};
use cw_storage_plus::{Key, Namespace, Path, PrimaryKey};
use nym_contracts_common::IdentityKeyRef;
use serde::de::DeserializeOwned;
use std::ops::Deref;

pub trait MixnetContractQuerier {
    #[allow(dead_code)]
    fn query_mixnet_contract<T: DeserializeOwned>(
        &self,
        address: impl Into<String>,
        msg: &QueryMsg,
    ) -> StdResult<T>;

    fn query_mixnet_contract_storage(
        &self,
        address: impl Into<String>,
        key: impl Into<Binary>,
    ) -> StdResult<Option<Vec<u8>>>;

    fn query_mixnet_contract_storage_value<T: DeserializeOwned>(
        &self,
        address: impl Into<String>,
        key: impl Into<Binary>,
    ) -> StdResult<Option<T>> {
        match self.query_mixnet_contract_storage(address, key)? {
            None => Ok(None),
            Some(value) => Ok(Some(from_json(&value)?)),
        }
    }

    fn query_current_mixnet_interval(&self, address: impl Into<String>) -> StdResult<Interval> {
        self.query_mixnet_contract_storage_value(address, b"ci")?
            .ok_or(StdError::not_found(
                "unable to retrieve interval information from the mixnet contract storage",
            ))
    }

    fn query_current_absolute_mixnet_epoch_id(
        &self,
        address: impl Into<String>,
    ) -> StdResult<EpochId> {
        self.query_current_mixnet_interval(address)
            .map(|interval| interval.current_epoch_absolute_id())
    }

    fn check_node_existence(&self, address: impl Into<String>, node_id: NodeId) -> StdResult<bool> {
        let mixnet_contract_address = address.into();

        if let Some(nym_node) = self.query_nymnode_bond(mixnet_contract_address.clone(), node_id)? {
            return Ok(!nym_node.is_unbonding);
        }

        Ok(false)
    }

    fn query_nymnode_bond(
        &self,
        address: impl Into<String>,
        node_id: NodeId,
    ) -> StdResult<Option<NymNodeBond>> {
        // construct proper map key
        let pk_namespace = "nn";
        let path: Path<NymNodeBond> = Path::new(
            Namespace::from_static_str(pk_namespace).as_slice(),
            &node_id.key().iter().map(Key::as_ref).collect::<Vec<_>>(),
        );
        let storage_key = path.deref();

        self.query_mixnet_contract_storage_value(address, storage_key)
    }
}

impl<C> MixnetContractQuerier for QuerierWrapper<'_, C>
where
    C: CustomQuery,
{
    fn query_mixnet_contract<T: DeserializeOwned>(
        &self,
        address: impl Into<String>,
        msg: &QueryMsg,
    ) -> StdResult<T> {
        self.query_wasm_smart(address, msg)
    }

    fn query_mixnet_contract_storage(
        &self,
        address: impl Into<String>,
        key: impl Into<Binary>,
    ) -> StdResult<Option<Vec<u8>>> {
        self.query_wasm_raw(address, key)
    }
}

#[track_caller]
pub fn compare_decimals(a: Decimal, b: Decimal, epsilon: Option<Decimal>) {
    let epsilon = epsilon.unwrap_or_else(|| Decimal::from_ratio(1u128, 100_000_000u128));
    if a > b {
        assert!(a - b < epsilon, "{a} != {b}, delta: {}", a - b)
    } else {
        assert!(b - a < epsilon, "{a} != {b}, delta: {}", b - a)
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
        Decimal::from_atomics(atomics, 0).map_err(|_| {
            StdError::generic_err(format!(
                "Decimal range exceeded for {atomics} with 0 decimal places."
            ))
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

    fn identity(&self) -> IdentityKeyRef<'_>;

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

    fn identity(&self) -> IdentityKeyRef<'_> {
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

    fn identity(&self) -> IdentityKeyRef<'_> {
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
