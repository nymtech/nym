// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Addr;
use cw4::Cw4Contract;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub multisig_addr: Addr,
    pub group_addr: Cw4Contract,
    pub mix_denom: String,
}
