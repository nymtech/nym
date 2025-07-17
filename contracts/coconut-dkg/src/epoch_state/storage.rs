// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{StdResult, Storage};
use cw_storage_plus::{Item, Map, SnapshotItem, Strategy};
use nym_coconut_dkg_common::types::{Epoch, EpochId};

#[deprecated]
// leave old values in storage for backwards compatibility, but make sure everything in the contract
// uses the new reference
pub(crate) const CURRENT_EPOCH: Item<Epoch> = Item::new("current_epoch");
pub const HISTORICAL_EPOCH: SnapshotItem<Epoch> = SnapshotItem::new(
    "historical_epoch",
    "historical_epoch__checkpoints",
    "historical_epoch__changelog",
    Strategy::EveryBlock,
);

pub const THRESHOLD: Item<u64> = Item::new("threshold");

pub const EPOCH_THRESHOLDS: Map<EpochId, u64> = Map::new("epoch_thresholds");

#[allow(deprecated)]
pub fn save_epoch(storage: &mut dyn Storage, height: u64, epoch: &Epoch) -> StdResult<()> {
    CURRENT_EPOCH.save(storage, epoch)?;
    // NOTE: we save data for the PREVIOUS height.
    // currently cw-plus snapshot is treated as if it happened at the beginning of a block,
    // meaning if we create checkpoint at heights 10 and heights 20 and then query for value
    // at height 20, it will still return value that was saved at height 10.
    // the correct one will only be returned from heights >= 21.
    // this is not what we want. if dkg state was updated at height 20, we want that updated state immediately.
    HISTORICAL_EPOCH.save(storage, epoch, height - 1)
}

#[allow(deprecated)]
pub fn load_current_epoch(storage: &dyn Storage) -> StdResult<Epoch> {
    #[cfg(debug_assertions)]
    {
        let current = CURRENT_EPOCH.load(storage);
        let historical = HISTORICAL_EPOCH.load(storage);
        debug_assert_eq!(current, historical);
    }
    CURRENT_EPOCH.load(storage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::mock_dependencies;

    #[test]
    fn check_cw_plus_snapshot_behaviour_hasnt_changed() {
        // so currently cw-plus snapshot is treated as if it happened at the beginning of a block,
        // meaning if we create checkpoint at heights 10 and heights 20 and then query for value
        // at height 20, it will still return value that was saved at height 10.
        // the correct one will only be returned from heights >= 21.
        // this is not what we want. if dkg state was updated at height 20, we want that updated state immediately.
        //
        // this test ensures that behaviour hasn't changed so that we wouldn't accidentally introduce inconsistency
        const DUMMY_SNAPSHOT: SnapshotItem<u64> =
            SnapshotItem::new("a", "b", "c", Strategy::EveryBlock);

        let mut deps = mock_dependencies();
        DUMMY_SNAPSHOT.save(&mut deps.storage, &10, 10).unwrap();
        DUMMY_SNAPSHOT.save(&mut deps.storage, &20, 20).unwrap();
        DUMMY_SNAPSHOT.save(&mut deps.storage, &30, 30).unwrap();

        assert_eq!(DUMMY_SNAPSHOT.load(&deps.storage).unwrap(), 30);
        assert_eq!(
            DUMMY_SNAPSHOT
                .may_load_at_height(&deps.storage, 40)
                .unwrap(),
            Some(30)
        );
        assert_eq!(
            DUMMY_SNAPSHOT
                .may_load_at_height(&deps.storage, 31)
                .unwrap(),
            Some(30)
        );
        assert_eq!(
            DUMMY_SNAPSHOT
                .may_load_at_height(&deps.storage, 30)
                .unwrap(),
            Some(20)
        );
        assert_eq!(
            DUMMY_SNAPSHOT
                .may_load_at_height(&deps.storage, 29)
                .unwrap(),
            Some(20)
        );
        assert_eq!(
            DUMMY_SNAPSHOT
                .may_load_at_height(&deps.storage, 21)
                .unwrap(),
            Some(20)
        );
        assert_eq!(
            DUMMY_SNAPSHOT
                .may_load_at_height(&deps.storage, 20)
                .unwrap(),
            Some(10)
        );
        assert_eq!(
            DUMMY_SNAPSHOT
                .may_load_at_height(&deps.storage, 19)
                .unwrap(),
            Some(10)
        );
        assert_eq!(
            DUMMY_SNAPSHOT
                .may_load_at_height(&deps.storage, 11)
                .unwrap(),
            Some(10)
        );
        assert_eq!(
            DUMMY_SNAPSHOT
                .may_load_at_height(&deps.storage, 10)
                .unwrap(),
            None
        );
        assert_eq!(
            DUMMY_SNAPSHOT.may_load_at_height(&deps.storage, 9).unwrap(),
            None
        );
    }
}
