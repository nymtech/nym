// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::helpers::importer_contract_path;
use crate::state::CachedState;
use clap::Args;
use importer_contract::contract::EmptyMessage;
use nym_validator_client::nyxd::cosmwasm_client::types::InstantiateOptions;
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::DirectSigningHttpRpcNyxdClient;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use tracing::info;

#[derive(Args, Clone)]
pub struct PrepareArgs {
    /// Path to the .wasm file with the importer contract
    /// If not provided, the CLI will attempt to traverse the parent directories until it finds
    /// "target/wasm32-unknown-unknown/release/importer_contract.wasm"
    #[clap(long)]
    pub importer_contract_path: Option<PathBuf>,
}

pub async fn create_importer_contract(
    explicit_contract_path: Option<PathBuf>,
    client: &DirectSigningHttpRpcNyxdClient,
) -> anyhow::Result<AccountId> {
    info!("attempting to create the importer contract");

    let importer_path = importer_contract_path(explicit_contract_path)?;
    info!(
        "going to use the following importer contract: '{}'",
        fs::canonicalize(&importer_path)?.display()
    );

    let mut data = Vec::new();
    File::open(importer_path)?.read_to_end(&mut data)?;

    let res = client.upload(data, "<empty>", None).await?;
    let importer_code_id = res.code_id;
    info!(
        "  ✅ uploaded the importer contract in {}",
        res.transaction_hash
    );

    let res = client
        .instantiate(
            importer_code_id,
            &EmptyMessage {},
            "importer-contract".into(),
            "<empty>",
            Some(InstantiateOptions::default().with_admin(client.address())),
            None,
        )
        .await?;
    let importer_address = res.contract_address;
    info!(
        "  ✅ instantiated the importer contract in {}",
        res.transaction_hash
    );

    info!("IMPORTER CONTRACT ADDRESS: {importer_address}");

    CachedState {
        importer_address: importer_address.clone(),
        state_imported: false,
    }
    .save()?;

    Ok(importer_address)
}

pub async fn execute_prepare_contract(
    args: PrepareArgs,
    client: DirectSigningHttpRpcNyxdClient,
) -> anyhow::Result<()> {
    create_importer_contract(args.importer_contract_path, &client).await?;

    Ok(())
}
