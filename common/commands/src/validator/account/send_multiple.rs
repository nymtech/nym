// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use crate::utils::pretty_coin;
use clap::Parser;
use comfy_table::Table;
use cosmrs::rpc::endpoint::tx::Response;
use log::{error, info};
use nym_validator_client::nyxd::{AccountId, Coin};
use serde_json::json;
use std::str::FromStr;
use std::{fs, io::Write};

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
        table.add_row(vec![row.address.to_string(), pretty_coin(&row.amount)]);
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
        .map(|row| (row.address.clone(), vec![row.amount.clone()]))
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

    let now = time::OffsetDateTime::now_utc();
    let now = now.format(&time::format_description::well_known::Rfc3339)?;

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
    pub amount: Coin,
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
            let tokens: Vec<_> = line.split(',').collect();
            if tokens.len() < 3 {
                return Err(anyhow::anyhow!(
                    "'{}' does not have enough columns, expecting <address>,<amount>,<denom>",
                    line
                ));
            }
            // try parse amount to u128
            let amount = u128::from_str(tokens[1])
                .map_err(|_| anyhow::anyhow!("'{}' has an invalid amount", line))?;

            let denom: String = tokens[2].into();

            // multiply when a whole token amount, e.g. 50nym (50.123456nym is not allowed, that must be input as 50123456unym)
            let (amount, denom) = if !denom.starts_with('u') {
                (amount * 1_000_000u128, format!("u{}", denom))
            } else {
                (amount, denom)
            };

            let address = AccountId::from_str(tokens[0])
                .map_err(|e| anyhow::anyhow!("'{}' has an invalid address: {}", line, e))?;

            let amount = Coin { amount, denom };

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

        assert_eq!(50_000_000u128, input_csv.rows[0].amount.amount);
        assert_eq!(50u128, input_csv.rows[1].amount.amount);
        assert_eq!(50_000_000u128, input_csv.rows[2].amount.amount);
        assert_eq!(50u128, input_csv.rows[3].amount.amount);

        assert_eq!("unym", input_csv.rows[0].amount.denom);
        assert_eq!("unym", input_csv.rows[1].amount.denom);
        assert_eq!("unyx", input_csv.rows[2].amount.denom);
        assert_eq!("unyx", input_csv.rows[3].amount.denom);
    }
}
