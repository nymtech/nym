// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::QueryClientWithNyxd;
use crate::utils::{pretty_decimal_with_denom, show_error};
use clap::Parser;
use comfy_table::Table;
use nym_validator_client::client::NymApiClientExt;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(value_parser)]
    #[clap(help = "Optionally, the mixnode to display")]
    pub identity_key: Option<String>,
}

pub async fn query(args: Args, client: &QueryClientWithNyxd) {
    match client.nym_api.get_mixnodes().await {
        Ok(res) => match args.identity_key {
            Some(identity_key) => {
                let node = res.iter().find(|node| {
                    node.bond_information
                        .mix_node
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
                    "Mix id",
                    "Identity Key",
                    "Owner",
                    "Host",
                    "Bond",
                    "Total Delegations",
                    "Version",
                ]);
                for node in res {
                    let denom = &node.bond_information.original_pledge().denom;
                    table.add_row(vec![
                        node.mix_id().to_string(),
                        node.bond_information.mix_node.identity_key.clone(),
                        node.bond_information.owner.clone().into_string(),
                        node.bond_information.mix_node.host.clone(),
                        pretty_decimal_with_denom(node.rewarding_details.operator, denom),
                        pretty_decimal_with_denom(node.rewarding_details.delegates, denom),
                        node.bond_information.mix_node.version,
                    ]);
                }

                println!("The mixnodes in the directory are:");
                println!("{table}");
            }
        },
        Err(e) => show_error(e),
    }
}
