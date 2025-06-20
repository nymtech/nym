// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{from_json, Binary, CustomQuery, QuerierWrapper, StdError, StdResult};
use cw_storage_plus::{Key, Namespace, Path, PrimaryKey};
use nym_mixnet_contract_common::{Interval, MixNodeBond, NymNodeBond};
use nym_performance_contract_common::{EpochId, NodeId};
use serde::de::DeserializeOwned;
use std::ops::Deref;

pub(crate) trait MixnetContractQuerier {
    #[allow(dead_code)]
    fn query_mixnet_contract<T: DeserializeOwned>(
        &self,
        address: impl Into<String>,
        msg: &nym_mixnet_contract_common::QueryMsg,
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

        self.query_mixnet_contract_storage_value(address, storage_key)
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
        msg: &nym_mixnet_contract_common::QueryMsg,
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
