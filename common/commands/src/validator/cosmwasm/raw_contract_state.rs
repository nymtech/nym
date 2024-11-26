// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::QueryClient;
use clap::Parser;
use cosmrs::AccountId;
use log::trace;
use nym_validator_client::nyxd::CosmWasmClient;
use std::fs;
use std::fs::File;
use std::path::PathBuf;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long, value_parser)]
    #[clap(help = "The address of contract to get the state of")]
    pub contract: AccountId,

    #[clap(short, long)]
    #[clap(help = "Output file for the retrieved contract state")]
    pub output: PathBuf,
}

pub async fn execute(args: Args, client: QueryClient) -> anyhow::Result<()> {
    trace!("args: {args:?}");

    let output = File::create(&args.output)?;
    let raw = client.query_all_contract_state(&args.contract).await?;

    serde_json::to_writer(output, &raw)?;
    println!(
        "wrote {} key-value from {} pairs into '{}'",
        raw.len(),
        args.contract,
        fs::canonicalize(args.output)?.display()
    );

    Ok(())
}
