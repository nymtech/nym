// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use chrono::{DateTime, Utc};
use clap::Parser;
use comfy_table::Table;
use cosmrs::rpc::endpoint::tx::Response;
use cosmwasm_std::{Coin as CosmWasmCoin, Uint128};
use log::{error, info, warn};
use serde_json::json;
use std::ops::MulAssign;
use std::str::FromStr;
use std::time::SystemTime;
use std::{fs, io::Write};

use nym_validator_client::nyxd::{AccountId, Coin};

use crate::context::SigningClient;
use crate::utils::pretty_cosmwasm_coin;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    pub memo: Option<String>,

    #[clap(
        long,
        help = "Input file path (CSV format) with account/amount pairs to send"
    )]
    pub input: String,

    #[clap(
        long,
        help = "An output file path (CSV format) to create or append a log of results to"
    )]
    pub output: Option<String>,
}

pub async fn send_multiple(args: Args, client: &SigningClient) {
    let memo = args
        .memo
        .unwrap_or_else(|| "Sending tokens with nym-cli".to_owned());

    let rows = InputFileReader::new(&args.input);
    if let Err(e) = rows {
        error!("Failed to read input file: {}", e);
        return;
    }
    let rows = rows.unwrap();

    let mut table = Table::new();

    if rows.rows.is_empty() {
        error!("No transactions to send");
        return;
    }

    println!(
        "The following transfer will be made from account {} to:",
        client.address()
    );
    table.set_header(vec!["Address", "Amount"]);

    for row in rows.rows.iter() {
        table.add_row(vec![
            row.address.to_string(),
            pretty_cosmwasm_coin(&row.amount),
        ]);
    }

    println!("{table}");

    let ans = inquire::Confirm::new("Do you want to continue with the transfers?")
        .with_default(false)
        .with_help_message("You must confirm before the transaction will be sent")
        .prompt();

    if let Err(e) = ans {
        info!("Aborting, {}...", e);
        return;
    }
    if let Ok(false) = ans {
        info!("Aborting!");
        return;
    }

    info!("Transferring from {}...", client.address());

    let multiple_sends: Vec<(AccountId, Vec<Coin>)> = rows
        .rows
        .iter()
        .map(|row| (row.address.clone(), vec![row.amount.clone().into()]))
        .collect();

    let res = client
        .send_multiple(multiple_sends, memo, None)
        .await
        .expect("failed to send tokens!");

    info!("Sending result: {}", json!(res));

    println!();
    println!(
        "Nodesguru: https://nym.explorers.guru/transaction/{}",
        &res.hash
    );
    println!("Mintscan: https://www.mintscan.io/nyx/txs/{}", &res.hash);
    println!("Transaction result code: {}", &res.tx_result.code.value());
    println!("Transaction hash: {}", &res.hash);

    if let Some(output_filename) = args.output {
        println!("\nWriting output log to {}", output_filename);

        if let Err(e) = write_output_file(rows, res, &output_filename) {
            error!(
                "Failed to write output file {} with error {}",
                output_filename, e
            );
        }
    }
}

fn write_output_file(
    rows: InputFileReader,
    res: Response,
    output_filename: &String,
) -> Result<(), anyhow::Error> {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(output_filename)?;

    let now = SystemTime::now();
    let now: DateTime<Utc> = now.into();
    let now = now.to_rfc3339();

    let data = rows
        .rows
        .iter()
        .map(|row| {
            format!(
                "{},{},{},{},{}",
                row.address, row.amount.amount, row.amount.denom, now, res.hash
            )
        })
        .collect::<Vec<String>>()
        .join("\n");

    Ok(file.write_all(format!("{}\n", data).as_bytes())?)
}

#[derive(Debug)]
pub struct InputFileRow {
    pub address: AccountId,
    pub amount: CosmWasmCoin,
}
pub struct InputFileReader {
    pub rows: Vec<InputFileRow>,
}

impl InputFileReader {
    pub fn new(path: &str) -> Result<InputFileReader, anyhow::Error> {
        let mut rows: Vec<InputFileRow> = vec![];
        let file_contents = fs::read_to_string(path)?;

        let lines: Vec<String> = file_contents.lines().map(String::from).collect();
        for line in lines {
            let tokens: Vec<_> = line.split(&[',']).filter(|k| !k.is_empty()).collect();
            if tokens.len() < 3 {
                warn!(
                    "'{}' does not have enough columns, expecting <address>,<amount>,<denom>",
                    line
                );
                continue;
            }

            // try parse amount to u128
            let amount = Uint128::from_str(tokens[1]);
            if amount.is_err() {
                warn!("'{}' has an invalid amount", line);
                continue;
            }
            let mut amount = amount.unwrap();
            let mut denom: String = tokens[2].into();

            // multiply when a whole token amount, e.g. 50nym (50.123456nym is not allowed, that must be input as 50123456unym)
            if !denom.starts_with('u') {
                amount.mul_assign(Uint128::new(1_000_000u128));
                denom = format!("u{}", denom);
            }

            let amount = CosmWasmCoin::new(amount.into(), denom);

            // try parse address
            let address = AccountId::from_str(tokens[0]);
            if let Err(e) = address {
                warn!("'{}' has an invalid address: {}", line, e);
                continue;
            }
            let address = address.unwrap();

            rows.push(InputFileRow { address, amount })
        }

        Ok(InputFileReader { rows })
    }
}

#[cfg(test)]
mod test_multiple_send_input_csv {
    use super::*;
    use nym_validator_client::nyxd::AccountId;
    use std::str::FromStr;
    #[test]
    fn works_on_happy_path() {
        let input_csv = InputFileReader::new("fixtures/test_send_multiple.csv").unwrap();
        assert_eq!(
            AccountId::from_str("n1q85lscptz860j3dx92f8phaeaw08j2l5dt7adq").unwrap(),
            input_csv.rows[0].address
        );

        println!("{:?}", input_csv.rows);

        assert_eq!(50_000_000u128, input_csv.rows[0].amount.amount.into());
        assert_eq!(50u128, input_csv.rows[1].amount.amount.into());
        assert_eq!(50_000_000u128, input_csv.rows[2].amount.amount.into());
        assert_eq!(50u128, input_csv.rows[3].amount.amount.into());

        assert_eq!("unym", input_csv.rows[0].amount.denom);
        assert_eq!("unym", input_csv.rows[1].amount.denom);
        assert_eq!("unyx", input_csv.rows[2].amount.denom);
        assert_eq!("unyx", input_csv.rows[3].amount.denom);
    }
}
