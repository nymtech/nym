// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::QueryClientWithNyxd;
use crate::utils::{pretty_cosmwasm_coin, show_error};
use clap::Parser;
use comfy_table::Table;
use nym_validator_client::client::NymApiClientExt;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(value_parser)]
    #[clap(help = "Optionally, the gateway to display")]
    pub identity_key: Option<String>,
}

pub async fn query(args: Args, client: &QueryClientWithNyxd) {
    match client.nym_api.get_gateways().await {
        Ok(res) => match args.identity_key {
            Some(identity_key) => {
                let node = res.iter().find(|node| {
                    node.gateway
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

                table.set_header(vec!["Identity Key", "Owner", "Host", "Bond", "Version"]);
                for node in res {
                    table.add_row(vec![
                        node.gateway.identity_key.to_string(),
                        node.owner.to_string(),
                        node.gateway.host.to_string(),
                        pretty_cosmwasm_coin(&node.pledge_amount),
                        node.gateway.version.clone(),
                    ]);
                }

                println!("The gateways in the directory are:");
                println!("{table}");
            }
        },
        Err(e) => show_error(e),
    }
}
