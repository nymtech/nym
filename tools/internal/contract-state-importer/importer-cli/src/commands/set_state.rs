// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::helpers::importer_contract_address;
use crate::state::CachedState;
use anyhow::bail;
use clap::Args;
use importer_contract::{base85rs, ExecuteMsg};
use nym_validator_client::nyxd::cosmwasm_client::types::Model;
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::DirectSigningHttpRpcNyxdClient;
use std::fs::File;
use std::path::PathBuf;
use tracing::info;

#[derive(Args, Clone)]
pub struct SetStateArgs {
    /// Explicit address of the initialised importer contract.
    /// If not set, the value from the cached state will be attempted to be used
    #[clap(long)]
    pub importer_contract_address: Option<AccountId>,

    /// Path to the file containing state dump of a cosmwasm contract
    #[clap(long)]
    pub raw_state: PathBuf,
}

fn approximate_size(pair: &Model) -> usize {
    base85rs::encode(&pair.key).len() + base85rs::encode(&pair.value).len()
}

fn models_to_exec(data: Vec<Model>) -> ExecuteMsg {
    let pairs = data
        .into_iter()
        .map(|kv| (kv.key, kv.value))
        .collect::<Vec<_>>();

    pairs.into()
}

fn split_into_importable_execute_msgs(
    kv_pairs: Vec<Model>,
    approximate_max_chunk: usize,
) -> Vec<ExecuteMsg> {
    let mut chunks: Vec<ExecuteMsg> = Vec::new();

    let mut current_wip_chunk = Vec::new();
    let mut current_chunk_size = 0;
    for kv in kv_pairs {
        if current_chunk_size + approximate_size(&kv) > approximate_max_chunk {
            let taken = std::mem::take(&mut current_wip_chunk);
            chunks.push(models_to_exec(taken));
            current_chunk_size = 0;
        }
        current_chunk_size += approximate_size(&kv);
        current_wip_chunk.push(kv);
    }

    if !current_wip_chunk.is_empty() {
        chunks.push(models_to_exec(current_wip_chunk))
    }
    chunks
}

pub async fn set_importer_state(
    state_dump_path: PathBuf,
    explicit_importer_address: Option<AccountId>,
    client: &DirectSigningHttpRpcNyxdClient,
) -> anyhow::Result<()> {
    info!("attempting to set the importer contract state");

    // this is the value that we found to be optimal during v1->v2 mixnet migration
    const MAX_CHUNK_SIZE: usize = 350 * 1000;

    let importer_address = importer_contract_address(explicit_importer_address)?;

    if let Ok(state) = CachedState::load() {
        if state.state_imported && state.importer_address == importer_address {
            bail!("the state has already been imported for {importer_address}")
        }
    }

    let dump_file = File::open(state_dump_path)?;
    info!("attempting to decode the state dump. for bigger contracts this might take a while...");
    let kv_pairs: Vec<Model> = serde_json::from_reader(&dump_file)?;

    info!("there are {} key-value pairs to import", kv_pairs.len());
    info!("attempting to split them into {MAX_CHUNK_SIZE}B chunks ExecuteMsgs...");

    let chunks = split_into_importable_execute_msgs(kv_pairs, MAX_CHUNK_SIZE);
    info!("obtained {} execute msgs", chunks.len());

    let total = chunks.len();
    for (i, msg) in chunks.into_iter().enumerate() {
        info!("executing message {}/{total}...", i + 1);
        let res = client
            .execute(
                &importer_address,
                &msg,
                None,
                "importing contract state",
                Vec::new(),
            )
            .await?;
        info!("  ✅ OK: {}", res.transaction_hash);
    }

    info!("Finished migrating storage to {importer_address}!");

    CachedState {
        importer_address,
        state_imported: true,
    }
    .save()?;

    Ok(())
}

pub async fn execute_set_state(
    args: SetStateArgs,
    client: DirectSigningHttpRpcNyxdClient,
) -> anyhow::Result<()> {
    set_importer_state(args.raw_state, args.importer_contract_address, &client).await?;

    Ok(())
}
