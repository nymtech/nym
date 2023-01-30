// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use coconut_dkg_common::types::{Epoch, InitialReplacementData};
use cw_storage_plus::Item;

pub(crate) const CURRENT_EPOCH: Item<'_, Epoch> = Item::new("current_epoch");
pub const THRESHOLD: Item<u64> = Item::new("threshold");
pub const INITIAL_REPLACEMENT_DATA: Item<InitialReplacementData> =
    Item::new("initial_replacement_data");
