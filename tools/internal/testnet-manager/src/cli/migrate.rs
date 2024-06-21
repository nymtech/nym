// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NetworkManagerError;
use clap::Parser;
use nym_validator_client::nyxd::cosmwasm_client::types::{ContractCodeId, EmptyMsg};

// nyxd-style command so, for example `migrate ecash 123 '{}'`
#[derive(Debug, Parser)]
pub(crate) struct Args {
    pub contract_name: String,

    pub code_id: ContractCodeId,

    pub message: serde_json::Value,
}

pub(crate) fn execute(args: Args) -> Result<(), NetworkManagerError> {
    todo!()
}
