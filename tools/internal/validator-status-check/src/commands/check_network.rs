// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::helpers::{get_known_dealers, get_signer_status};
use crate::models::SignerStatus;
use comfy_table::Table;
use nym_bin_common::output_format::OutputFormat;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(clap::Args, Debug)]
pub(crate) struct Args {
    #[clap(short, long, default_value_t = OutputFormat::default())]
    output: OutputFormat,
}

#[derive(Serialize, Deserialize)]
struct NetworkStatus {
    available_api_nodes: f64,
    available_rpc_nodes: f64,
    detailed: Vec<SignerStatus>,
}

impl From<Vec<SignerStatus>> for NetworkStatus {
    fn from(value: Vec<SignerStatus>) -> Self {
        let nodes = value.len() as f64;
        let api = value.iter().filter(|s| s.api_up()).count() as f64;
        let rpc = value.iter().filter(|s| s.rpc_up()).count() as f64;

        NetworkStatus {
            available_api_nodes: api / nodes,
            available_rpc_nodes: rpc / nodes,
            detailed: value,
        }
    }
}

impl Display for NetworkStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "available signers: {:.2}%, available rpc nodes: {:.2}%",
            self.available_api_nodes, self.available_rpc_nodes
        )?;

        let mut table = Table::new();
        table.set_header(vec![
            "signer",
            "api version",
            "rpc status",
            "rpc endpoint",
            "abci version",
        ]);
        for signer in &self.detailed {
            table.add_row(signer.to_table_row());
        }
        write!(f, "{table}")
    }
}

pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    let dealers = get_known_dealers().await?;

    let mut signers = Vec::new();
    for dealer in dealers {
        signers.push(get_signer_status(&dealer.announce_address).await)
    }

    let out = args.output.format(&NetworkStatus::from(signers));
    println!("{out}");

    Ok(())
}
