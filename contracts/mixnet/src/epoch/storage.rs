// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cw_storage_plus::{Item, Map};
use mixnet_contract_common::{Epoch, IdentityKey, NodeStatus};

pub(crate) const CURRENT_EPOCH: Item<Epoch> = Item::new("cep");
pub(crate) const _EPOCH_MAP: Map<u32, Epoch> = Map::new("ep");
pub(crate) const REWARDED_SET_HEIGHTS_FOR_EPOCH: Map<(u32, u64), ()> = Map::new("rsh");
pub(crate) const REWARDED_SET: Map<(u64, IdentityKey), NodeStatus> = Map::new("rs");
