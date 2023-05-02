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
    #[clap(help = "Optionally, the service provider to display")]
    pub nym_address: Option<String>,
}

pub async fn query(args: Args, client: &QueryClientWithNyxd) {
    match client.nym_api.get_service_providers().await {
        Ok(res) => {
            if let Some(nym_address) = args.nym_address {
                let service = res.iter().find(|service| {
                    service
                        .service
                        .nym_address
                        .to_string()
                        .eq_ignore_ascii_case(&nym_address)
                });
                println!(
                    "{}",
                    ::serde_json::to_string_pretty(&service).expect("json formatting error")
                );
            } else {
                let mut table = Table::new();

                table.set_header(vec!["Service Id", "Announcer", "Nym Address"]);
                for service in res {
                    table.add_row(vec![
                        service.service_id.to_string(),
                        service.service.announcer.to_string(),
                        service.service.service_type.to_string(),
                        service.service.nym_address.to_string(),
                    ]);
                }

                println!("The service providers in the directory are:");
                println!("{table}");
            }
        }
        Err(NymAPIError::NotFound) => {
            println!("nym-api reports no service provider endpoint available");
        }
        Err(e) => show_error(e),
    }
}
