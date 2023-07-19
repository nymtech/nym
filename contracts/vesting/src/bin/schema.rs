// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_schema::write_api;
use vesting_contract_common::{ExecuteMsg, InitMsg, MigrateMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: InitMsg,
        query: QueryMsg,
        execute: ExecuteMsg,
        migrate: MigrateMsg,
    }
}
