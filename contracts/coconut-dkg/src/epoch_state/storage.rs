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
pub fn save_epoch(storage: &mut dyn Storage, epoch: &Epoch) -> StdResult<()> {
    CURRENT_EPOCH.save(storage, epoch)
}

#[allow(deprecated)]
pub fn update_epoch<A>(storage: &mut dyn Storage, action: A) -> StdResult<()>
where
    A: FnOnce(Epoch) -> Result<Epoch, cosmwasm_std::StdError>,
{
    CURRENT_EPOCH.update(storage, action)?;
    Ok(())
}

#[allow(deprecated)]
pub fn load_current_epoch(storage: &dyn Storage) -> StdResult<Epoch> {
    CURRENT_EPOCH.load(storage)
}
