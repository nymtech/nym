// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cw_storage_plus::{Item, Map};
use nym_coconut_dkg_common::types::{Epoch, EpochId};

pub(crate) const CURRENT_EPOCH: Item<'_, Epoch> = Item::new("current_epoch");

pub const THRESHOLD: Item<u64> = Item::new("threshold");

pub const EPOCH_THRESHOLDS: Map<EpochId, u64> = Map::new("epoch_thresholds");
