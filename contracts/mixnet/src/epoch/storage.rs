// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{StdResult, Storage};
use cw_storage_plus::{Item, Map};
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

pub(crate) fn advance_epoch(storage: &mut dyn Storage) -> StdResult<Epoch> {
    CURRENT_EPOCH.update(storage, |current_epoch| Ok(current_epoch.next_epoch()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::tests::test_helpers;

    #[test]
    fn saving_rewarded_set() {
        let mut deps = test_helpers::init_contract();

        let active_set_size = 100;
        let mut nodes = Vec::new();
        for i in 0..1000 {
            nodes.push(format!("identity{:04}", i))
        }

        save_rewarded_set(deps.as_mut().storage, 1234, active_set_size, nodes).unwrap();

        // first k nodes MUST BE active
        for i in 0..1000 {
            let identity = format!("identity{:04}", i);
            if i < active_set_size {
                assert_eq!(
                    RewardedSetNodeStatus::Active,
                    REWARDED_SET
                        .load(deps.as_ref().storage, (1234, identity))
                        .unwrap()
                )
            } else {
                assert_eq!(
                    RewardedSetNodeStatus::Standby,
                    REWARDED_SET
                        .load(deps.as_ref().storage, (1234, identity))
                        .unwrap()
                )
            }
        }
    }

    #[test]
    fn advancing_epoch() {
        let mut deps = test_helpers::init_contract();

        let initial = CURRENT_EPOCH.load(deps.as_ref().storage).unwrap();
        let new_epoch = advance_epoch(deps.as_mut().storage).unwrap();
        let new_epoch_read = CURRENT_EPOCH.load(deps.as_ref().storage).unwrap();

        assert_eq!(new_epoch, new_epoch_read);
        assert_eq!(initial.next_epoch(), new_epoch);
        assert_eq!(initial.end(), new_epoch.start());
        assert_eq!(initial.length(), new_epoch.length());

        // as a sanity check, advance it again
        let new_epoch2 = advance_epoch(deps.as_mut().storage).unwrap();
        let new_epoch_read2 = CURRENT_EPOCH.load(deps.as_ref().storage).unwrap();

        assert_eq!(new_epoch2, new_epoch_read2);
        assert_eq!(new_epoch.next_epoch(), new_epoch2);
        assert_eq!(new_epoch.end(), new_epoch2.start());
        assert_eq!(new_epoch.length(), new_epoch2.length());
    }
}
