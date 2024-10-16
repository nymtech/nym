// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::commands::prepare::create_importer_contract;
use crate::commands::set_state::set_importer_state;
use crate::commands::swap_contract::swap_contract;
use clap::ArgGroup;
use clap::Args;
use nym_validator_client::DirectSigningHttpRpcNyxdClient;
use std::path::PathBuf;
use tracing::info;

#[derive(Args, Clone)]
#[clap(group(ArgGroup::new("contract").required(true)))]
pub struct InitialiseWithStateArgs {
    /// Path to the .wasm file with the importer contract
    /// If not provided, the CLI will attempt to traverse the parent directories until it finds
    /// "target/wasm32-unknown-unknown/release/importer_contract.wasm"
    #[clap(long)]
    pub importer_contract_path: Option<PathBuf>,

    /// Path to the file containing state dump of a cosmwasm contract
    #[clap(long)]
    pub raw_state: PathBuf,

    /// Code id of the previously uploaded cosmwasm smart contract that will be applied to the imported state
    #[clap(long, group = "contract")]
    pub target_contract_code_id: Option<u64>,

    /// Path to a cosmwasm smart contract that will be uploaded and applied to the imported state
    #[clap(long, group = "contract")]
    pub target_contract_path: Option<PathBuf>,

    #[clap(long)]
    pub migrate_msg: Option<serde_json::Value>,
}

pub async fn initialise_with_state(
    args: InitialiseWithStateArgs,
    client: DirectSigningHttpRpcNyxdClient,
) -> anyhow::Result<()> {
    let importer_address = create_importer_contract(args.importer_contract_path, &client).await?;
    set_importer_state(args.raw_state, Some(importer_address.clone()), &client).await?;
    swap_contract(
        args.target_contract_code_id,
        args.target_contract_path,
        Some(importer_address.clone()),
        args.migrate_msg,
        &client,
    )
    .await?;

    info!("the contract is ready at {importer_address}");

    Ok(())
}
