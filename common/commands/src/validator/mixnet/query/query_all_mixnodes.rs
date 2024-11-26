// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::QueryClientWithNyxd;
use crate::utils::show_error;
use clap::Parser;
use comfy_table::Table;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(value_parser)]
    #[clap(help = "Optionally, the mixnode to display")]
    pub identity_key: Option<String>,
}

pub async fn query(args: Args, client: &QueryClientWithNyxd) {
    match client.get_all_cached_described_nodes().await {
        Ok(res) => match args.identity_key {
            Some(identity_key) => {
                let node = res.iter().find(|node| {
                    node.ed25519_identity_key()
                        .to_string()
                        .eq_ignore_ascii_case(&identity_key)
                });
                println!(
                    "{}",
                    ::serde_json::to_string_pretty(&node).expect("json formatting error")
                );
            }
            None => {
                let mut table = Table::new();

                table.set_header(vec!["Node Id", "Identity Key", "Version", "Is Legacy"]);
                for node in res
                    .into_iter()
                    .filter(|node| node.description.declared_role.mixnode)
                {
                    table.add_row(vec![
                        node.node_id.to_string(),
                        node.ed25519_identity_key().to_base58_string(),
                        node.description.build_information.build_version,
                        (!node.contract_node_type.is_nym_node()).to_string(),
                    ]);
                }

                println!("The mixnodes in the directory are:");
                println!("{table}");
            }
        },
        Err(e) => show_error(e),
    }
}
