// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::cli::{try_load_current_config, ConfigOverridableArgs};
use crate::rewarder::nyxd_client::NyxdClient;
use crate::rewarder::storage::RewarderStorage;
use crate::rewarder::ticketbook_issuance::types::CredentialIssuer;
use crate::rewarder::ticketbook_issuance::verifier::TicketbookIssuanceVerifier;
use crate::rewarder::Rewarder;
use anyhow::bail;
use nym_ecash_time::ecash_default_expiration_date;
use std::collections::HashSet;
use std::path::PathBuf;
use time::macros::format_description;
use time::Date;
use tracing::info;

fn parse_date(raw: &str) -> Result<Date, time::error::Parse> {
    let format = format_description!("[year]-[month]-[day]");
    Date::parse(raw, &format)
}

#[derive(Debug, clap::Args)]
pub struct Args {
    #[command(flatten)]
    config_override: ConfigOverridableArgs,

    /// Specifies custom location for the configuration file of nym validators rewarder.
    #[clap(long, env = "NYM_VALIDATOR_REWARDER_PROCESS_BLOCK_CONFIG_PATH")]
    custom_config_path: Option<PathBuf>,

    /// expiration date used for verifying the ticketbooks
    #[clap(long, value_parser = parse_date, default_value_t = ecash_default_expiration_date())]
    pub(crate) expiration_date: Date,

    /// identifier of the specified signer
    #[clap(long)]
    signer: String,
}

fn get_issuer(
    all: Vec<CredentialIssuer>,
    target: String,
    banned: &[String],
) -> anyhow::Result<CredentialIssuer> {
    let banned: HashSet<_> = banned.iter().collect();
    if !banned.is_empty() {
        info!("the following issuers have been banned in the past: {banned:?}");
    }
    if !all.is_empty() {
        info!("the following signers are available: ");
        for issuer in &all {
            info!("{issuer}");
        }
    }

    for issuer in all {
        if banned.contains(&issuer.operator_account.to_string()) {
            bail!("{issuer} has been banned");
        }

        // attempt to retrieve it from any field
        if issuer.operator_account.to_string() == target {
            return Ok(issuer);
        }

        if issuer.api_client.api_url().to_string().contains(&target) {
            return Ok(issuer);
        }

        if issuer.node_id.to_string() == target {
            return Ok(issuer);
        }

        if issuer.public_key.to_string() == target {
            return Ok(issuer);
        }
    }

    bail!("could not find {target} issuer")
}

pub(crate) async fn execute(args: Args) -> anyhow::Result<()> {
    let mut config =
        try_load_current_config(&args.custom_config_path)?.with_override(args.config_override);

    // HACK: make sure the verification ratio is always 100% so the target issuer gets checked
    config.ticketbook_issuance.full_verification_ratio = 1.0;

    let storage = RewarderStorage::init(&config.storage_paths.reward_history).await?;
    let nyxd_client = NyxdClient::new(&config)?;
    let rewarder_keypair = Rewarder::try_load_identity_keypair(&config)?;

    let issuers = nyxd_client.get_current_ticketbook_issuers().await?;
    let banned = storage.load_banned_ticketbook_issuers().await?;

    let issuer_to_check = get_issuer(issuers, args.signer, &banned)?;

    let mut verifier = TicketbookIssuanceVerifier::new(
        config.verification_config(),
        &rewarder_keypair,
        &config.ticketbook_issuance.whitelist,
        banned,
        args.expiration_date,
    );

    println!("Attempting to verify issuer state of {issuer_to_check}...");
    println!("this might take a while...");
    println!();

    let Some(completed_test) = verifier.check_issuer(issuer_to_check).await else {
        println!(
            "\t⚠️ no ticketbooks issued with expiration on {} or using outdated API",
            args.expiration_date
        );
        return Ok(());
    };

    println!("##### {} RESULTS #####", completed_test.details);
    if let Some(ban) = completed_test.issuer_ban {
        println!("❗ CHEATING DETECTED ❗");
        println!("\t{}", ban.reason);
        println!();
        println!("❗ EVIDENCE ❗");
        println!(
            "{}",
            serde_json::from_slice::<serde_json::Value>(&ban.serialised_evidence)
                .unwrap_or_default()
        );
        return Ok(());
    }

    println!(
        " - issued ticketbooks:\t{}",
        completed_test
            .issued_commitment
            .as_ref()
            .map(|c| c.body.deposits.len())
            .unwrap_or_default()
    );
    println!(
        " - merkle root commitment: {:?}",
        completed_test
            .issued_commitment
            .as_ref()
            .and_then(|a| a.body.merkle_root_hex())
    );
    println!(
        " - sampled deposits:\t{:?}",
        completed_test.sampled_deposits.keys().collect::<Vec<_>>()
    );
    println!(
        " - claimed response size:\t{:?}",
        completed_test
            .challenge_commitment_response
            .as_ref()
            .map(|r| r.body.max_data_response_size)
    );

    Ok(())
}
