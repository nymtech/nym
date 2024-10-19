// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::helpers::importer_contract_address;
use crate::state::CachedState;
use anyhow::bail;
use clap::ArgGroup;
use clap::Args;
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::DirectSigningHttpRpcNyxdClient;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use tracing::info;

#[derive(Args, Clone)]
#[clap(group(ArgGroup::new("contract").required(true)))]
pub struct SwapContractArgs {
    /// Explicit address of the initialised importer contract.
    /// If not set, the value from the cached state will be attempted to be used
    #[clap(long)]
    pub importer_contract_address: Option<AccountId>,

    /// Code id of the previously uploaded cosmwasm smart contract that will be applied to the imported state
    #[clap(long, group = "contract")]
    pub target_contract_code_id: Option<u64>,

    /// Path to a cosmwasm smart contract that will be uploaded and applied to the imported state
    #[clap(long, group = "contract")]
    pub target_contract_path: Option<PathBuf>,

    /// The custom migrate message used for migrating into target contract.
    /// If none is provided an empty object will be used instead, i.e. '{}'
    #[clap(long)]
    pub migrate_msg: Option<serde_json::Value>,
}

pub async fn swap_contract(
    target_code_id: Option<u64>,
    target_contract_path: Option<PathBuf>,
    explicit_importer_address: Option<AccountId>,
    migrate_msg: Option<serde_json::Value>,
    client: &DirectSigningHttpRpcNyxdClient,
) -> anyhow::Result<()> {
    info!("attempting to swap the contract code");

    let importer_address = importer_contract_address(explicit_importer_address)?;

    if let Ok(state) = CachedState::load() {
        if !state.state_imported && state.importer_address == importer_address {
            bail!("the state hasn't been imported for {importer_address}")
        }
    }

    // one of those must have been set via clap
    let code_id = match target_code_id {
        Some(explicit) => explicit,
        None => {
            // upload the contract
            let mut data = Vec::new();
            File::open(target_contract_path.unwrap())?.read_to_end(&mut data)?;

            let res = client.upload(data, "<empty>", None).await?;
            info!(
                "  ✅ uploaded the target contract in {}",
                res.transaction_hash
            );
            res.code_id
        }
    };

    let migrate_msg = migrate_msg.unwrap_or(serde_json::Value::Object(Default::default()));
    let res = client
        .migrate(
            &importer_address,
            code_id,
            &migrate_msg,
            "migrating into target contract",
            None,
        )
        .await?;
    info!(
        "  ✅ migrated into the target contract: {}",
        res.transaction_hash
    );

    Ok(())
}

pub async fn execute_swap_contract(
    args: SwapContractArgs,
    client: DirectSigningHttpRpcNyxdClient,
) -> anyhow::Result<()> {
    swap_contract(
        args.target_contract_code_id,
        args.target_contract_path,
        args.importer_contract_address,
        args.migrate_msg,
        &client,
    )
    .await?;

    Ok(())
}
