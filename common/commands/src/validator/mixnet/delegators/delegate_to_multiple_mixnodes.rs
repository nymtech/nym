// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::fs::OpenOptions;

use clap::Parser;
use comfy_table::Table;
use csv::WriterBuilder;
use log::{info, warn};
use nym_mixnet_contract_common::ExecuteMsg;
use nym_mixnet_contract_common::ExecuteMsg::{DelegateToMixnode, UndelegateFromMixnode};

use nym_mixnet_contract_common::PendingEpochEventKind::{Delegate, Undelegate};
use nym_validator_client::nyxd::contract_traits::{NymContractsProvider, PagedMixnetQueryClient};
use nym_validator_client::nyxd::Coin;

use crate::context::SigningClient;
use crate::utils::pretty_coin;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub memo: Option<String>,

    #[clap(
        long,
        help = "Input csv files with delegation amounts. Format: (mixID, amount(in NYM))"
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
        let file_contents = fs::read_to_string(path)?;
        let mut rows = Vec::new();
        let mut mix_id_set = HashSet::new();

        for line in file_contents
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
        {
            let tokens: Vec<_> = line.split(',').collect();
            if tokens.len() != 2 {
                anyhow::bail!("Incorrect format: {}", line);
            }
            let mix_id = tokens[0].trim().to_string();
            let input_amount = BigDecimal::parse_bytes(tokens[1].trim().as_bytes(), 10)
                .ok_or_else(|| anyhow::anyhow!("Invalid number format"))?;
            let scaled_amount = input_amount.with_scale(6);

            if scaled_amount > BigDecimal::from(1_000_000) {
                warn!("Delegation amount is high. Please make sure your input is in NYM and not unym denomination");
            }

            let smallest_unit_multiplier = BigDecimal::from_u64(1_000_000).unwrap(); // For 6 decimal places
            let amount_in_smallest_unit = scaled_amount * smallest_unit_multiplier;

            let amount = amount_in_smallest_unit.to_u128().ok_or_else(|| {
                anyhow::anyhow!("Amount after scaling cannot be represented in u128")
            })?;

            if !mix_id_set.insert(mix_id.clone()) {
                anyhow::bail!("Duplicate mix_id found: {}", mix_id);
            }

            rows.push(InputFileRow {
                mix_id,
                amount: Coin {
                    amount,
                    denom: "unym".to_string(),
                },
            });
        }
        Ok(InputFileReader { rows })
    }
}

fn write_to_csv(
    output_details: Vec<[String; 3]>,
    output_file: Option<String>,
) -> Result<(), anyhow::Error> {
    if let Some(file_path) = output_file {
        // Determine if the file exists and is not empty
        let file_exists = fs::metadata(&file_path)
            .map(|metadata| metadata.len() > 0)
            .unwrap_or(false);

        // Open the file for appending or creation
        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&file_path)?;

        if !file_exists {
            let mut wtr = csv::Writer::from_writer(&file);
            wtr.write_record(["Operation", "Transaction Hash", "Timestamp"])?;
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

async fn fetch_delegation_data(
    client: &SigningClient,
) -> Result<HashMap<String, Coin>, anyhow::Error> {
    let address = client.address();
    // Fetch all delegations for the user
    let delegations = match client.get_all_delegator_delegations(&address).await {
        Ok(delegations) => delegations,
        Err(e) => {
            anyhow::bail!("Error fetching delegations: {}", e)
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
            anyhow::bail!("Error fetching pending epoch events: {}", e);
        }
    };

    for event in pending_events {
        match event.event.kind {
            // If a pending undelegate tx is found, remove it from delegation map
            Undelegate { owner, mix_id, .. } => {
                if owner == address.as_ref()
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
                if owner == address.as_ref() {
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

    Ok(existing_delegation_map)
}

pub async fn delegate_to_multiple_mixnodes(args: Args, client: SigningClient) {
    let records = match InputFileReader::new(&args.input) {
        Ok(records) => records,
        Err(e) => {
            println!("Error reading input file: {}", e);
            return;
        }
    };

    let existing_delegation_map = fetch_delegation_data(&client)
        .await
        .expect("Could not fetch existing delegations");

    let mut delegation_table = Table::new();
    let mut undelegation_table = Table::new();
    let mut delegation_msgs: Vec<(ExecuteMsg, Vec<Coin>)> = Vec::new();
    let mut undelegation_msgs: Vec<(ExecuteMsg, Vec<Coin>)> = Vec::new();

    delegation_table.set_header(["Mix ID", "Input Amount", "Adjusted Amount"]);
    undelegation_table.set_header(["Mix ID"]);

    for row in &records.rows {
        let input_amount = row.amount.amount;
        let existing_delegation_amount = existing_delegation_map
            .get(&row.mix_id)
            .map_or(0, |coin| coin.amount);

        match existing_delegation_amount.cmp(&input_amount) {
            Ordering::Equal => continue, // No action needed if amounts are equal

            Ordering::Less => {
                // Delegate the difference if the existing delegation is less
                let difference = Coin {
                    amount: input_amount - existing_delegation_amount,
                    denom: row.amount.denom.clone(),
                };
                let mix_id = row.mix_id.clone().parse::<u32>().unwrap();
                delegation_msgs.push((DelegateToMixnode { mix_id }, vec![difference.clone()]));
                delegation_table.add_row(&[
                    row.mix_id.clone(),
                    pretty_coin(&row.amount),
                    pretty_coin(&difference),
                ]);
            }

            Ordering::Greater => {
                let mix_id = row.mix_id.clone().parse::<u32>().unwrap();
                let coins: Vec<Coin> = vec![];
                undelegation_msgs.push((UndelegateFromMixnode { mix_id }, coins));
                undelegation_table.add_row(&[row.mix_id.clone()]);

                if row.amount.amount > 0 {
                    delegation_msgs.push((DelegateToMixnode { mix_id }, vec![row.amount.clone()]));
                    delegation_table.add_row(&[
                        row.mix_id.clone(),
                        pretty_coin(&row.amount),
                        pretty_coin(&row.amount),
                    ]);
                }
            }
        }
    }

    if delegation_msgs.is_empty() && undelegation_msgs.is_empty() {
        println!("Nothing to do. Delegations are up-to-date!");
        return;
    }

    if !undelegation_msgs.is_empty() {
        println!("Undelegation records : \n{}\n\n", undelegation_table);
    }

    if !delegation_msgs.is_empty() {
        println!("Delegation records : \n{}\n\n", delegation_table);
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

    let mut output_details: Vec<[String; 3]> = Vec::new();
    let now = time::OffsetDateTime::now_utc();
    let now = now
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap();

    let mixnet_contract = client
        .mixnet_contract_address()
        .expect("mixnet contract address is not available");

    // Execute all undelegation transactions
    if !undelegation_msgs.is_empty() {
        let res = client
            .execute_multiple(
                mixnet_contract,
                undelegation_msgs.clone(),
                None,
                format!(
                    "Undelegate from {} nodes via nym-cli",
                    undelegation_msgs.len()
                ),
            )
            .await
            .expect("Could not undelegate!");

        println!(
            "Undelegation transaction successful : {}",
            res.transaction_hash
        );
        output_details.push([
            "Undelegate".to_string(),
            res.transaction_hash.to_string(),
            now.clone(),
        ]);
    }

    // Execute all  delegation delegations
    if !delegation_msgs.is_empty() {
        let res = client
            .execute_multiple(
                mixnet_contract,
                delegation_msgs,
                None,
                format!(
                    "Delegatations to {} nodes via nym-cli",
                    undelegation_msgs.len()
                ),
            )
            .await
            .expect("Could not delegate");

        println!(
            "Delegation transaction successful : {}",
            res.transaction_hash
        );
        output_details.push([
            "Delegate".to_string(),
            res.transaction_hash.to_string(),
            now.clone(),
        ]);
    }

    if args.output.is_some() {
        if let Err(e) = write_to_csv(output_details, args.output) {
            info!("Failed to write to CSV, {}", e);
        }
    }
}
