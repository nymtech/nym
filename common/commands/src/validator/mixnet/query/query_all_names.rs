// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use comfy_table::Table;
use nym_validator_client::nym_api::error::NymAPIError;

use crate::context::QueryClientWithNyxd;
use crate::utils::show_error;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(value_parser)]
    #[clap(help = "Optionally, the registered name to display")]
    pub name: Option<String>,
}

pub async fn query(args: Args, client: &QueryClientWithNyxd) {
    match client.nym_api.get_registered_names().await {
        Ok(res) => {
            if let Some(name) = args.name {
                let name = res.iter().find(|name_entry| {
                    name_entry.name.name.to_string().eq_ignore_ascii_case(&name)
                });
                println!(
                    "{}",
                    ::serde_json::to_string_pretty(&name).expect("json formatting error")
                );
            } else {
                let mut table = Table::new();

                table.set_header(vec!["Name Id", "Owner", "Name"]);
                for name_entry in res {
                    table.add_row(vec![
                        name_entry.name_id.to_string(),
                        name_entry.name.owner.to_string(),
                        name_entry.name.name.to_string(),
                    ]);
                }

                println!("The registered names in the directory are:");
                println!("{table}");
            }
        }
        Err(NymAPIError::NotFound) => {
            println!("nym-api reports no name endpoint available");
        }
        Err(e) => show_error(e),
    }
}
