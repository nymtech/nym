// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs;
use std::fs::OpenOptions;

use clap::Parser;
use comfy_table::Table;
use csv::WriterBuilder;
use log::{info, warn};

use nym_mixnet_contract_common::PendingEpochEventKind::{Delegate, Undelegate};
use nym_validator_client::nyxd::contract_traits::{MixnetSigningClient, PagedMixnetQueryClient};
use nym_validator_client::nyxd::Coin;

use crate::context::SigningClient;
use crate::utils::pretty_coin;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub memo: Option<String>,

    #[clap(
        long,
        help = "Input csv files with delegation amounts. Format: mixid, amount in NYM"
    )]
    pub input: String,

    #[clap(
        long,
        help = "An output file path (CSV format) to create or append a log of results to"
    )]
    pub output: Option<String>,
}

#[derive(Debug)]
pub struct InputFileRow {
    pub mix_id: String,
    pub amount: Coin,
}
#[derive(Debug)]
pub struct InputFileReader {
    pub rows: Vec<InputFileRow>,
}

impl InputFileReader {
    pub fn new(path: &str) -> Result<InputFileReader, anyhow::Error> {
        let mut rows: Vec<InputFileRow> = Vec::new();
        let file_contents = fs::read_to_string(path)?;
        let mut mix_id_list: Vec<String> = Vec::new();
        let lines: Vec<String> = file_contents.lines().map(String::from).collect();
        for line in lines {
            // Skip over blank lines without throwing an error
            if line.trim().is_empty() {
                continue;
            }

            let tokens: Vec<_> = line.split(',').collect();
            // Return error if any of the line is malformed
            if tokens.len() != 2 {
                return Err(anyhow::anyhow!(
                    "Malformed input file, please make sure the file is in the correct format (mix_id, amount)"
                ));
            }

            let mix_id = tokens[0].trim().to_string();
            let token_input_raw = tokens[1].trim().parse::<u128>()?;
            if token_input_raw > 1_000_000 {
                warn!(
                    "Delegation amount exceeds 1,000,000. \
                Make sure the input amount is in nym and not unym denomination!"
                );
            }

            let tokens_in_unym: u128 = token_input_raw * 1_000_000;

            let amount = Coin {
                amount: tokens_in_unym,
                denom: "unym".to_string(),
            };
            if mix_id_list.contains(&mix_id) {
                return Err(anyhow::anyhow!(
                    "Input document has duplicate delegation record for {}",
                    mix_id.clone()
                ));
            } else {
                rows.push(InputFileRow {
                    mix_id: mix_id.clone(),
                    amount,
                });
                mix_id_list.push(mix_id);
            }
        }
        Ok(InputFileReader { rows })
    }
}

fn write_to_csv(
    output_details: Vec<[String; 4]>,
    output_file: Option<String>,
) -> Result<(), anyhow::Error> {
    if let Some(file_path) = output_file {
        // Determine if the file exists and is not empty
        let file_exists = fs::metadata(&file_path)
            .map(|metadata| metadata.len() > 0)
            .unwrap_or(false);

        // Open the file for appending or creation
        let file = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(&file_path)?;

        if !file_exists {
            let mut wtr = csv::Writer::from_writer(&file);
            wtr.write_record(["Operation", "Mix ID", "Amount", "tx hash"])?;
            wtr.flush()?;
        }

        let mut wtr = WriterBuilder::new()
            .has_headers(!file_exists)
            .from_writer(file);

        // Write the details to the CSV file
        for detail in output_details {
            wtr.write_record(&detail)?;
        }
        wtr.flush()?;
        info!("All operations saved to output file");
    }
    Ok(())
}

pub async fn delegate_to_multiple_mixnodes(args: Args, client: SigningClient) {
    let address = client.address();
    let records = match InputFileReader::new(&args.input) {
        Ok(records) => records,
        Err(e) => {
            println!("Error reading input file: {}", e);
            return;
        }
    };

    // Fetch all delegations for the user
    let delegations = match client.get_all_delegator_delegations(&address).await {
        Ok(delegations) => delegations,
        Err(e) => {
            println!("Error fetching delegator delegations: {}", e);
            return;
        }
    };

    // Build a map to make it easier to handle delegation data
    let mut existing_delegation_map: HashMap<String, Coin> = HashMap::new();
    let mut pending_delegation_map: HashMap<String, Coin> = HashMap::new();

    for delegation in delegations {
        existing_delegation_map
            .insert(delegation.mix_id.to_string(), Coin::from(delegation.amount));
    }

    // Look for pending delegate / undelegate events which might be of interest to us
    let pending_events = match client.get_all_pending_epoch_events().await {
        Ok(events) => events,
        Err(e) => {
            println!("Error fetching pending epoch events: {}", e);
            return;
        }
    };

    for event in pending_events {
        match event.event.kind {
            // If a pending undelegate tx is found, remove it from delegation map
            Undelegate { owner, mix_id, .. } => {
                if owner == client.address().as_ref()
                    && existing_delegation_map.get(&mix_id.to_string()).is_some()
                {
                    existing_delegation_map.remove(&mix_id.to_string());
                }
            }

            // If a pending delegation event is found, gather them to consolidate later
            Delegate {
                owner,
                mix_id,
                amount,
                ..
            } => {
                if owner == client.address().as_ref() {
                    let mut amount = Coin::from(amount);
                    if let Some(pending_record) = pending_delegation_map.get(&mix_id.to_string()) {
                        amount.amount += pending_record.amount;
                    }
                    pending_delegation_map.insert(mix_id.to_string(), amount);
                }
            }
            _ => {}
        };
    }

    // Consolidate pending events into delegation map
    for (mix_id, amount) in pending_delegation_map {
        existing_delegation_map
            .entry(mix_id)
            .and_modify(|e| e.amount += amount.amount)
            .or_insert(amount);
    }

    let mut delegation_table = Table::new();
    let mut undelegation_table = Table::new();
    let mut delegations_to_be_made: Vec<(String, Coin)> = Vec::new();
    let mut undelegations_to_be_made = Vec::new();

    delegation_table.set_header(["Mix ID", "Input Amount", "Adjusted Amount"]);
    undelegation_table.set_header(["Mix ID"]);

    for row in records.rows.iter() {
        // Check if there's an existing delegation for this mix_id
        if let Some(existing_delegation_record) = existing_delegation_map.get(&row.mix_id) {
            let existing_delegation_amount = existing_delegation_record.amount;
            let input_amount = row.amount.amount;

            match existing_delegation_amount.cmp(&input_amount) {
                // Nothing to do if the delegation record matches, just continue
                Ordering::Equal => continue,

                // If existing delegation is lesser, we need to delegate only remaining amount
                Ordering::Less => {
                    let adjusted_amount = Coin {
                        amount: input_amount - existing_delegation_amount,
                        denom: existing_delegation_record.denom.clone(),
                    };

                    delegation_table.add_row([
                        row.mix_id.clone(),
                        pretty_coin(&row.amount.clone()),
                        pretty_coin(&adjusted_amount),
                    ]);
                    delegations_to_be_made.push((row.mix_id.clone(), adjusted_amount));
                }

                // If existing delegation is greater, we need to undelegate and delegate the specified amount
                Ordering::Greater => {
                    undelegations_to_be_made.push(row.mix_id.clone());
                    undelegation_table.add_row([row.mix_id.clone()]);

                    delegations_to_be_made.push((row.mix_id.clone(), row.amount.clone()));
                    delegation_table.add_row([
                        row.mix_id.clone(),
                        pretty_coin(&row.amount),
                        pretty_coin(&row.amount),
                    ]);
                }
            }; // match close
        } else {
            delegations_to_be_made.push((row.mix_id.clone(), row.amount.clone()));
            delegation_table.add_row([
                row.mix_id.clone(),
                pretty_coin(&row.amount.clone()),
                pretty_coin(&row.amount.clone()),
            ]);
        }
    }

    if delegations_to_be_made.is_empty() && undelegations_to_be_made.is_empty() {
        println!("Nothing to do. Delegations are up-to-date!");
        return;
    }
    if !delegations_to_be_made.is_empty() {
        println!("Delegation records : \n{}\n\n", delegation_table);
    }

    if !undelegations_to_be_made.is_empty() {
        println!("Undelegation records : \n{}\n\n", undelegation_table);
    }

    let ans = inquire::Confirm::new("Do you want to continue with the shown operations?")
        .with_default(false)
        .with_help_message("You must confirm before the transactions are signed")
        .prompt();

    if let Err(e) = ans {
        info!("Aborting, {}...", e);
        return;
    }

    if let Ok(false) = ans {
        info!("Aborting:: User denied proceeding with signing!");
        return;
    }

    let mut output_details: Vec<[String; 4]> = Vec::new();

    // Perform undelegation operations
    for mix_id in undelegations_to_be_made {
        let res = client
            .undelegate_from_mixnode(mix_id.parse::<u32>().unwrap(), None)
            .await
            .expect("Failed to undelegate from mixnode");
        info!(
            "Undelegation from {} successful. tx: {}",
            mix_id, &res.transaction_hash
        );
        if args.output.is_some() {
            output_details.push([
                "Undelegate".into(),
                mix_id.clone(),
                "-".into(),
                res.transaction_hash.to_string(),
            ]);
        }
    }

    // Perform delegation operations
    for (mix_id, amount) in delegations_to_be_made {
        let res = client
            .delegate_to_mixnode(mix_id.parse::<u32>().unwrap(), amount.clone(), None)
            .await
            .expect("Failed to delegate to mixnode");

        info!(
            "Delegation to {} successful. tx: {}",
            mix_id, &res.transaction_hash
        );
        if args.output.is_some() {
            output_details.push([
                "Delegate".into(),
                mix_id.clone(),
                pretty_coin(&amount),
                res.transaction_hash.to_string(),
            ]);
        }
    }

    if args.output.is_some() {
        if let Err(e) = write_to_csv(output_details, args.output) {
            info!("Failed to write to CSV, {}", e);
        }
    }
}
