// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use coconut_dkg_common::types::EpochState;
use cw_storage_plus::Item;

pub(crate) const CURRENT_EPOCH_STATE: Item<'_, EpochState> = Item::new("current_epoch_state");
