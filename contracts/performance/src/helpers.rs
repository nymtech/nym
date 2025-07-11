// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{StdError, StdResult};
use cw_storage_plus::{Key, Namespace, Path, PrimaryKey};
use nym_contracts_common::contract_querier::ContractQuerier;
use nym_mixnet_contract_common::{Interval, MixNodeBond, NymNodeBond};
use nym_performance_contract_common::{EpochId, NodeId};
use std::ops::Deref;

pub(crate) trait MixnetContractQuerier: ContractQuerier {
    fn query_current_mixnet_interval(&self, address: impl Into<String>) -> StdResult<Interval> {
        self.query_contract_storage_value(address, b"ci")?
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

        // 1. check if it's a nym-node
        if let Some(nym_node) = self.query_nymnode_bond(mixnet_contract_address.clone(), node_id)? {
            return Ok(!nym_node.is_unbonding);
        }

        // 2. try a legacy mixnode
        if let Some(nym_node) = self.query_mixnode_bond(mixnet_contract_address, node_id)? {
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

        self.query_contract_storage_value(address, storage_key)
    }

    fn query_mixnode_bond(
        &self,
        address: impl Into<String>,
        node_id: NodeId,
    ) -> StdResult<Option<MixNodeBond>> {
        // construct proper map key
        let pk_namespace = "mnn";
        let path: Path<MixNodeBond> = Path::new(
            Namespace::from_static_str(pk_namespace).as_slice(),
            &node_id.key().iter().map(Key::as_ref).collect::<Vec<_>>(),
        );
        let storage_key = path.deref();

        self.query_contract_storage_value(address, storage_key)
    }
}

impl<T> MixnetContractQuerier for T where T: ContractQuerier {}
