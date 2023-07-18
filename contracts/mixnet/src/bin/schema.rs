// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_schema::write_api;
use mixnet_contract_common::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,
        migrate: MigrateMsg,
    }
}

/*
pub fn generate_api_impl(input: &Options) -> ExprStruct {
    let Options {
        name,
        version,
        instantiate,
        execute,
        query,
        migrate,
        sudo,
        responses,
    } = input;

    parse_quote! {
        ::cosmwasm_schema::Api {
            contract_name: #name.to_string(),
            contract_version: #version.to_string(),
            instantiate: ::cosmwasm_schema::schema_for!(#instantiate),
            execute: #execute,
            query: #query,
            migrate: #migrate,
            sudo: #sudo,
            responses: #responses,
        }
    }
}

 */
