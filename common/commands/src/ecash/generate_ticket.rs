// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::utils::CommonConfigsWrapper;
use anyhow::{anyhow, bail};
use clap::Parser;
use colored::Colorize;
use comfy_table::Table;
use nym_credential_storage::initialise_persistent_storage;
use nym_credential_storage::storage::Storage;
use nym_credentials::ecash::bandwidth::serialiser::VersionedSerialise;
use std::path::PathBuf;

#[derive(Debug, Parser)]
pub struct Args {
    /// Specify the index of the ticket to retrieve from the ticketbook.
    /// By default, the current unspent value is used.
    #[clap(long, group = "output")]
    pub(crate) ticket_index: Option<u64>,

    /// Specify whether we should display payments for ALL available tickets
    #[clap(long, group = "output")]
    pub(crate) full: bool,

    /// Base58-encoded identity of the provider (must be 32 bytes long)
    #[clap(long)]
    pub(crate) provider: String,

    /// Config file of the client that is supposed to use the credential.
    #[clap(long, group = "source")]
    pub(crate) client_config: Option<PathBuf>,

    /// Path to the dedicated credential storage database
    #[clap(long, group = "source")]
    pub(crate) credential_storage: Option<PathBuf>,
}

pub async fn execute(args: Args) -> anyhow::Result<()> {
    let credentials_store = if let Some(explicit) = args.credential_storage {
        explicit
    } else {
        // SAFETY: at least one of them MUST HAVE been specified
        let cfg = args.client_config.unwrap();

        let loaded = CommonConfigsWrapper::try_load(cfg)?;

        if let Ok(id) = loaded.try_get_id() {
            println!("loaded config file for client '{id}'");
        }

        let Ok(credentials_store) = loaded.try_get_credentials_store() else {
            bail!("the loaded config does not have a credentials store information")
        };
        credentials_store
    };

    let decoded_provider = bs58::decode(&args.provider).into_vec()?;
    if decoded_provider.len() != 32 {
        bail!("the provided provider information is malformed")
    }
    let provider_arr: [u8; 32] = decoded_provider.try_into().unwrap();

    let persistent_storage = initialise_persistent_storage(&credentials_store).await;
    let Some(mut next_ticketbook) = persistent_storage
        .get_next_unspent_usable_ticketbook(0)
        .await?
    else {
        bail!(
            "there are no valid ticketbooks in the storage at {}",
            credentials_store.display()
        )
    };

    let epoch_id = next_ticketbook.ticketbook.epoch_id();
    let expiration_date = next_ticketbook.ticketbook.expiration_date();

    let verification_key = persistent_storage
        .get_master_verification_key(epoch_id)
        .await?
        .ok_or_else(|| {
            anyhow!("ticketbook got incorrectly imported - the master verification key is missing")
        })?;
    let expiration_signatures = persistent_storage
        .get_expiration_date_signatures(expiration_date)
        .await?
        .ok_or_else(|| {
            anyhow!(
                "ticketbook got incorrectly imported - the expiration date signatures are missing"
            )
        })?;
    let coin_indices_signatures = persistent_storage
        .get_coin_index_signatures(epoch_id)
        .await?
        .ok_or_else(|| {
            anyhow!("ticketbook got incorrectly imported - the coin index signatures are missing")
        })?;

    let ticketbook_data = next_ticketbook.ticketbook.pack();

    let next_ticket = args
        .ticket_index
        .unwrap_or(next_ticketbook.ticketbook.spent_tickets());
    let pay_info = next_ticketbook.ticketbook.generate_pay_info(provider_arr);

    println!("{}", "TICKETBOOK DATA:".bold());
    println!("{}", bs58::encode(&ticketbook_data.data).into_string());
    println!();

    // display it only for a single ticket
    if !args.full {
        println!("attempting to generate payment for ticket {next_ticket}...");
        println!();
        next_ticketbook.ticketbook.update_spent_tickets(next_ticket);

        let req = next_ticketbook.ticketbook.prepare_for_spending(
            &verification_key,
            pay_info.into(),
            &coin_indices_signatures,
            &expiration_signatures,
            1,
        )?;

        let payment = req.payment;

        println!("{}", format!("PAYMENT FOR TICKET {next_ticket}: ").bold());
        println!("{}", bs58::encode(&payment.to_bytes()).into_string());
        return Ok(());
    }

    println!(
        "generating payment information for {} tickets. this might take a while!...",
        next_ticketbook.ticketbook.params_total_tickets()
    );

    // otherwise generate all the payments
    let last_spent = next_ticketbook.ticketbook.spent_tickets();

    let mut table = Table::new();
    table.set_header(vec!["index", "binary data", "spend status"]);

    for i in 0..next_ticketbook.ticketbook.params_total_tickets() {
        let status = if i < last_spent {
            "SPENT".red()
        } else {
            "NOT SPENT".green()
        };

        next_ticketbook.ticketbook.update_spent_tickets(i);

        let req = next_ticketbook.ticketbook.prepare_for_spending(
            &verification_key,
            pay_info.into(),
            &coin_indices_signatures,
            &expiration_signatures,
            1,
        )?;

        let payment = req.payment;
        let payment_bytes = payment.to_bytes();
        let len = payment_bytes.len();
        let display_size = 100;
        let remaining = len - display_size;

        table.add_row(vec![
            i.to_string(),
            format!(
                "{}…{remaining}bytes remaining",
                bs58::encode(&payment_bytes[..display_size]).into_string()
            ),
            status.to_string(),
        ]);
    }

    println!("{}", "AVAILABLE TICKETS".bold());
    println!("{table}");

    Ok(())
}
