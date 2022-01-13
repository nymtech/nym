// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{StdResult, Storage};
use cw_storage_plus::{Item, Map, PrimaryKey};
use mixnet_contract_common::{Epoch, IdentityKey, RewardedSetNodeStatus};

// type aliases for better reasoning for storage keys
// (I found it helpful)
type BlockHeight = u64;
type EpochId = u32;

// TODO: those values need to be verified
pub(crate) const REWARDED_NODE_DEFAULT_LIMIT: u32 = 1000;
pub(crate) const REWARDED_NODE_MAX_LIMIT: u32 = 1500;

pub(crate) const CURRENT_EPOCH: Item<Epoch> = Item::new("cep");
pub(crate) const CURRENT_REWARDED_SET_HEIGHT: Item<BlockHeight> = Item::new("crh");

// pub(crate) const _EPOCH_MAP: Map<u32, Epoch> = Map::new("ep");

// I've changed the `()` data to an `u8` as after serializing `()` is represented as "null",
// taking more space than a single digit u8. If we don't care about what's there, why not go with more efficient approach? : )
pub(crate) const REWARDED_SET_HEIGHTS_FOR_EPOCH: Map<(EpochId, BlockHeight), u8> = Map::new("rsh");

// pub(crate) const REWARDED_SET: Map<(u64, IdentityKey), NodeStatus> = Map::new("rs");
pub(crate) const REWARDED_SET: Map<(BlockHeight, IdentityKey), RewardedSetNodeStatus> =
    Map::new("rs");

pub(crate) fn save_rewarded_set(
    storage: &mut dyn Storage,
    height: BlockHeight,
    active_set_size: u32,
    entries: Vec<IdentityKey>,
) -> StdResult<()> {
    for (i, identity) in entries.into_iter().enumerate() {
        // first k nodes are active
        let set_status = if i < active_set_size as usize {
            RewardedSetNodeStatus::Active
        } else {
            RewardedSetNodeStatus::Standby
        };

        REWARDED_SET.save(storage, (height, identity), &set_status)?;
    }

    Ok(())
}

pub(crate) fn advance_epoch(storage: &mut dyn Storage) -> StdResult<()> {
    CURRENT_EPOCH
        .update(storage, |current_epoch| Ok(current_epoch.next_epoch()))
        .map(|_| ())
}
