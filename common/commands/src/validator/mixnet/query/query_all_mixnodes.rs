// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use comfy_table::Table;

use crate::context::QueryClientWithValidatorAPI;
use crate::utils::{pretty_cosmwasm_coin, show_error};

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(value_parser)]
    #[clap(help = "Optionally, the mixnode to display")]
    pub identity_key: Option<String>,
}

pub async fn query(args: Args, client: &QueryClientWithValidatorAPI) {
    match client.validator_api.get_mixnodes().await {
        Ok(res) => match args.identity_key {
            Some(identity_key) => {
                let node = res.iter().find(|node| {
                    node.mix_node
                        .identity_key
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

                table.set_header(vec![
                    "Identity Key",
                    "Owner",
                    "Host",
                    "Bond",
                    "Total Delegations",
                    "Version",
                ]);
                for node in res {
                    table.add_row(vec![
                        node.mix_node.identity_key.to_string(),
                        node.owner.to_string(),
                        node.mix_node.host.to_string(),
                        pretty_cosmwasm_coin(&node.pledge_amount),
                        pretty_cosmwasm_coin(&node.total_delegation()),
                        node.mix_node.version,
                    ]);
                }

                println!("The mixnodes in the directory are:");
                println!("{table}");
            }
        },
        Err(e) => show_error(e),
    }
}
