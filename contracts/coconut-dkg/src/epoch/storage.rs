// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use coconut_dkg_common::types::Epoch;
use cw_storage_plus::Item;

pub(crate) const CURRENT_EPOCH: Item<Epoch> = Item::new("current_epoch");
