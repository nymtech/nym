// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

extern crate mixnet_contract;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};
use mixnet_contract::{ExecuteMsg, InstantiateMsg, MixNodeBond, QueryMsg};
use mixnet_contracts::mixnet_params::state::State;
use std::env::current_dir;
use std::fs::create_dir_all;

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(State), &out_dir);
    export_schema(&schema_for!(MixNodeBond), &out_dir);
}
