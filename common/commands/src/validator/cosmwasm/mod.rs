// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{Args, Subcommand};

pub mod execute_contract;
pub mod generators;
pub mod init_contract;
pub mod migrate_contract;
pub mod raw_contract_state;
pub mod upload_contract;

#[derive(Debug, Args)]
#[clap(args_conflicts_with_subcommands = true, subcommand_required = true)]
pub struct Cosmwasm {
    #[clap(subcommand)]
    pub command: CosmwasmCommands,
}

#[derive(Debug, Subcommand)]
pub enum CosmwasmCommands {
    /// Upload a smart contract WASM blob
    Upload(crate::validator::cosmwasm::upload_contract::Args),
    /// Init a WASM smart contract
    Init(crate::validator::cosmwasm::init_contract::Args),
    /// Generate an instantiate message
    GenerateInitMessage(generators::GenerateMessage),
    /// Migrate a WASM smart contract
    Migrate(crate::validator::cosmwasm::migrate_contract::Args),
    /// Execute a WASM smart contract method
    Execute(crate::validator::cosmwasm::execute_contract::Args),
    /// Obtain raw contract state of a cosmwasm smart contract
    RawContractState(crate::validator::cosmwasm::raw_contract_state::Args),
}
