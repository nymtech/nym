// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Addr;
use cw_controllers::Admin;
use cw_storage_plus::Item;
use serde::{Deserialize, Serialize};

pub const ADMIN: Admin = Admin::new("admin");

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct Config {
    pub multisig_addr: Addr,
    pub pool_addr: Addr,
    pub mix_denom: String,
}

pub const CONFIG: Item<Config> = Item::new("config");
