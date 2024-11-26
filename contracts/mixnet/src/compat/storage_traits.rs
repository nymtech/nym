// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// use crate::mixnodes::storage as mixnodes_storage;
// use crate::nodes::storage as nymnodes_storage;
// use crate::rewards::storage as rewards_storage;
// use cosmwasm_std::Storage;
// use cw_storage_plus::Map;
// use mixnet_contract_common::error::MixnetContractError;
// use mixnet_contract_common::mixnode::PendingMixNodeChanges;
// use mixnet_contract_common::{NodeId, PendingNodeChanges};
// use serde::de::DeserializeOwned;
// use serde::{Deserialize, Serialize};
//
// // I've created this trait to ensure everything is always stored in the right storage bucket,
// // because I fear I might have accidentally missed something during the transition period
// // of having BOTH mixnodes and nym-nodes
// pub(crate) trait NodeDetailsStorage {
//     //
// }
//
// pub(crate) trait NodeBondStorage {
//     //
// }
//
// pub(crate) trait PendingChangesStorage: Sized + Serialize + DeserializeOwned {
//     const STORAGE_MAP: Map<'static, NodeId, Self>;
//
//     fn save(&self, storage: &mut dyn Storage, node_id: NodeId) -> Result<(), MixnetContractError> {
//         Ok(Self::STORAGE_MAP.save(storage, node_id, self)?)
//     }
//
//     fn load(storage: &dyn Storage, node_id: NodeId) -> Result<Self, MixnetContractError> {
//         Ok(Self::STORAGE_MAP.load(storage, node_id)?)
//     }
// }
//
// pub(crate) trait RewardingStorage {
//     //
// }
//
// impl PendingChangesStorage for PendingNodeChanges {
//     const STORAGE_MAP: Map<'static, NodeId, Self> = nymnodes_storage::PENDING_NYMNODE_CHANGES;
// }
//
// impl PendingChangesStorage for PendingMixNodeChanges {
//     const STORAGE_MAP: Map<'static, NodeId, Self> = mixnodes_storage::PENDING_MIXNODE_CHANGES;
// }
