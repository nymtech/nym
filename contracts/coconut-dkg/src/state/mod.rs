// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Addr;
use cw4::Cw4Contract;
use cw_controllers::Admin;
use cw_storage_plus::Item;
use serde::{Deserialize, Serialize};

// unique items
pub const STATE: Item<State> = Item::new("state");
pub const MULTISIG: Admin = Admin::new("multisig");

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct State {
    pub mix_denom: String,
    pub multisig_addr: Addr,
    pub group_addr: Cw4Contract,

    /// Specifies the number of elements in the derived keys
    pub key_size: u32,
}
