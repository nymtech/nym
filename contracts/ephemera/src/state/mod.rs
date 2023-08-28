// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cw4::Cw4Contract;
use cw_storage_plus::Item;
use serde::{Deserialize, Serialize};

// unique items
pub const STATE: Item<State> = Item::new("state");

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct State {
    pub mix_denom: String,
    pub group_addr: Cw4Contract,
}
