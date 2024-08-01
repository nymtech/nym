// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{
    CUMULATIVE_EPOCH_WORK_KEY, MIXNODES_REWARDING_PK_NAMESPACE, PENDING_REWARD_POOL_KEY,
    REWARDING_PARAMS_KEY,
};
use crate::rewards::models::RewardPoolChange;
use cosmwasm_std::{Decimal, StdResult, Storage};
use cw_storage_plus::{Item, Map};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::mixnode::NodeRewarding;
use mixnet_contract_common::reward_params::{RewardingParams, WorkFactor};
use mixnet_contract_common::NodeId;

// LEGACY CONSTANTS:

// current parameters used for rewarding purposes
pub(crate) const REWARDING_PARAMS: Item<'_, RewardingParams> = Item::new(REWARDING_PARAMS_KEY);
pub(crate) const PENDING_REWARD_POOL_CHANGE: Item<'_, RewardPoolChange> =
    Item::new(PENDING_REWARD_POOL_KEY);

pub const MIXNODE_REWARDING: Map<NodeId, NodeRewarding> = Map::new(MIXNODES_REWARDING_PK_NAMESPACE);

// we're using the same underlying key to allow seamless delegation migration
pub const NYMNODE_REWARDING: Map<NodeId, NodeRewarding> = MIXNODE_REWARDING;

pub struct RewardingStorage<'a> {
    /// Global parameters used for reward calculation, such as the current reward pool, the active set size, etc.
    pub global_rewarding_params: Item<'a, RewardingParams>,

    /// All the changes to the rewarding pool that should get applied upon the **interval** finishing.
    pub pending_reward_pool_change: Item<'a, RewardPoolChange>,

    /// Information associated with all nym-nodes (and legacy-mixnodes) required for reward calculation
    // important note: this is using **EXACTLY** the same underlying key (and structure) as legacy mixnode rewarding
    pub nym_node_rewarding_data: Map<'a, NodeId, NodeRewarding>,

    /// keeps track of total cumulative work submitted for this rewarding epoch to make sure it never goes above 1
    pub cumulative_epoch_work: Item<'a, WorkFactor>,
}

impl<'a> RewardingStorage<'a> {
    pub const fn new() -> RewardingStorage<'a> {
        RewardingStorage {
            global_rewarding_params: REWARDING_PARAMS,
            pending_reward_pool_change: PENDING_REWARD_POOL_CHANGE,
            nym_node_rewarding_data: NYMNODE_REWARDING,
            cumulative_epoch_work: Item::new(CUMULATIVE_EPOCH_WORK_KEY),
        }
    }

    // an 'alias' because a `new` method might be a bit misleading since it'd suggest a brand new storage is created
    // as opposed to using the same underlying data as before
    pub const fn load() -> RewardingStorage<'a> {
        Self::new()
    }

    pub fn initialise(
        &self,
        storage: &mut dyn Storage,
        reward_params: RewardingParams,
    ) -> StdResult<()> {
        self.global_rewarding_params.save(storage, &reward_params)?;
        self.pending_reward_pool_change
            .save(storage, &RewardPoolChange::default())?;
        self.cumulative_epoch_work
            .save(storage, &WorkFactor::zero())?;

        Ok(())
    }

    pub fn reset_cumulative_epoch_work(
        &self,
        storage: &mut dyn Storage,
    ) -> Result<(), MixnetContractError> {
        self.cumulative_epoch_work
            .save(storage, &WorkFactor::zero())?;
        Ok(())
    }

    pub fn update_cumulative_epoch_work(
        &self,
        storage: &mut dyn Storage,
        work: Decimal,
    ) -> Result<(), MixnetContractError> {
        // we use a default in case this is the first run in the new contract since that value hasn't existed before
        let current = self
            .cumulative_epoch_work
            .may_load(storage)?
            .unwrap_or(WorkFactor::zero());
        let updated = current + work;
        if updated > WorkFactor::one() {
            return Err(MixnetContractError::TotalWorkAboveOne);
        }
        self.cumulative_epoch_work.save(storage, &updated)?;
        Ok(())
    }

    pub fn add_pending_pool_changes(
        &self,
        storage: &mut dyn Storage,
        amount: Decimal,
    ) -> Result<(), MixnetContractError> {
        let mut pending_changes = self.pending_reward_pool_change.load(storage)?;
        pending_changes.removed += amount;
        self.pending_reward_pool_change
            .save(storage, &pending_changes)?;
        Ok(())
    }

    pub fn try_persist_node_reward(
        &self,
        storage: &mut dyn Storage,
        node: NodeId,
        updated_data: NodeRewarding,
        reward: Decimal,
        work: WorkFactor,
    ) -> Result<(), MixnetContractError> {
        self.nym_node_rewarding_data
            .save(storage, node, &updated_data)?;
        self.add_pending_pool_changes(storage, reward)?;
        self.update_cumulative_epoch_work(storage, work)?;

        Ok(())
    }
}
