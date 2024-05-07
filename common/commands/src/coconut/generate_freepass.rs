// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::SigningClient;
use anyhow::{anyhow, bail};
use clap::ArgGroup;
use clap::Parser;
use futures::StreamExt;
use log::{error, info};
use nym_coconut_dkg_common::types::EpochId;
use nym_credential_utils::utils::block_until_coconut_is_available;
use nym_credentials::coconut::utils::freepass_exp_date;
use nym_credentials::coconut::utils::today;
use nym_credentials::{
    obtain_aggregate_verification_key, IssuanceBandwidthCredential, IssuedBandwidthCredential,
};
use nym_credentials_interface::aggregate_expiration_signatures;
use nym_credentials_interface::ExpirationDateSignatureShare;
use nym_credentials_interface::VerificationKeyAuth;
use nym_validator_client::coconut::all_ecash_api_clients;
use nym_validator_client::nyxd::contract_traits::{DkgQueryClient, NymContractsProvider};
use nym_validator_client::nyxd::CosmWasmClient;
use nym_validator_client::signing::AccountData;
use nym_validator_client::CoconutApiClient;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use time::format_description::well_known::Rfc3339;
use time::macros::time;
use time::OffsetDateTime;
use zeroize::Zeroizing;

fn parse_rfc3339_expiration_date(raw: &str) -> Result<OffsetDateTime, time::error::Parse> {
    OffsetDateTime::parse(raw, &Rfc3339)
}

#[derive(Debug, Parser)]
#[clap(group(ArgGroup::new("expiration").required(true)))]
pub struct Args {
    /// Specifies the expiration date of the free pass(es)
    /// Can't be set to more than a week into the future.
    #[clap(long, group = "expiration", value_parser = parse_rfc3339_expiration_date)]
    pub(crate) expiration_date: Option<OffsetDateTime>,

    /// The expiration of the free pass(es) expresses as unix timestamp.
    /// Can't be set to more than a week into the future.
    #[clap(long, group = "expiration")]
    pub(crate) expiration_timestamp: Option<i64>,

    /// The number of free passes to issue
    #[clap(long, default_value = "1")]
    pub(crate) amount: u64,

    /// Path to the output directory for generated free passes.
    #[clap(long)]
    pub(crate) output_dir: PathBuf,
}

async fn get_freepass(
    api_clients: Vec<CoconutApiClient>,
    aggregate_vk: &VerificationKeyAuth,
    threshold: u64,
    epoch_id: EpochId,
    signing_account: &AccountData,
    expiration_date: OffsetDateTime,
) -> anyhow::Result<IssuedBandwidthCredential> {
    let issuance_pass = IssuanceBandwidthCredential::new_freepass(expiration_date);
    let signing_data = issuance_pass.prepare_for_signing();

    let shares = Arc::new(tokio::sync::Mutex::new((Vec::new(), Vec::new())));
    futures::stream::iter(api_clients)
        .for_each_concurrent(None, |client| async {
            // move the client into the block
            let client = client;
            let api_url = client.api_client.api_url();

            info!("contacting {api_url} for blinded free pass");

            match issuance_pass
                .obtain_partial_freepass_credential(
                    &client.api_client,
                    client.node_id,
                    signing_account,
                    &client.verification_key,
                    signing_data.clone(),
                )
                .await
            {
                Ok(partial_wallet) => {
                    shares.lock().await.0.push(partial_wallet);
                }
                Err(err) => {
                    error!("failed to obtain partial free pass from {api_url}: {err}")
                }
            }

            info!("contacting {api_url} for expiration date signatures");
            match client
                .api_client
                .expiration_date_signatures_timestamp(expiration_date.unix_timestamp() as u64)
                .await
            {
                Ok(signature) => {
                    let index = client.node_id;
                    let key_share = client.verification_key.clone();
                    shares.lock().await.1.push(ExpirationDateSignatureShare {
                        index,
                        key: key_share,
                        signatures: signature.signatures,
                    });
                }
                Err(err) => {
                    error!("failed to obtain expiration date signature from {api_url}: {err}");
                }
            }
        })
        .await;

    // SAFETY: the futures have completed, so we MUST have the only arc reference
    #[allow(clippy::unwrap_used)]
    let (wallet_shares, signatures_shares) = Arc::into_inner(shares).unwrap().into_inner();

    if wallet_shares.len() < threshold as usize {
        bail!("we managed to obtain only {} partial credentials while the minimum threshold is {threshold}", wallet_shares.len());
    }

    let wallet =
        issuance_pass.aggregate_signature_shares(aggregate_vk, &wallet_shares, signing_data)?;

    if signatures_shares.len() < threshold as usize {
        bail!("we managed to obtain only {} partial expiration date signatures while the minimum threshold is {threshold}", signatures_shares.len());
    }

    let exp_date_sigs = aggregate_expiration_signatures(
        aggregate_vk,
        expiration_date.unix_timestamp() as u64,
        &signatures_shares,
    )?;
    Ok(issuance_pass.into_issued_credential(wallet, exp_date_sigs, epoch_id))
}

pub async fn execute(args: Args, client: SigningClient) -> anyhow::Result<()> {
    let address = client.address();

    if !args.output_dir.is_dir() {
        bail!("the provided output directory is not a directory!");
    }

    if args.output_dir.read_dir()?.next().is_some() {
        bail!("the provided output directory is not empty!");
    }

    let Some(bandwidth_contract) = client.coconut_bandwidth_contract_address() else {
        bail!("the bandwidth contract address is not set")
    };

    let Some(bandwidth_admin) = client
        .get_contract(bandwidth_contract)
        .await
        .map(|c| c.contract_info.admin)?
    else {
        bail!("the bandwidth contract doesn't have any admin set")
    };

    // sanity checks since nym-apis will reject invalid requests anyway
    if address != bandwidth_admin {
        bail!("the provided mnemonic does not correspond to the current admin of the bandwidth contract")
    }

    let expiration_date = match args.expiration_date {
        Some(date) => date,
        // SAFETY: one of those arguments must have been set
        None => OffsetDateTime::from_unix_timestamp(args.expiration_timestamp.unwrap())?,
    };

    let expiration_date = expiration_date.replace_time(time!(0:00));

    if expiration_date > freepass_exp_date() {
        bail!("the provided free pass request has too long expiry (expiry is set to on {expiration_date})")
    }

    if expiration_date < today() {
        bail!("the provided free pass expiry is set in the past!")
    }

    // issuance start
    block_until_coconut_is_available(&client).await?;

    let signing_account = client.signing_account()?;

    let epoch_id = client.get_current_epoch().await?.epoch_id;
    let threshold = client
        .get_current_epoch_threshold()
        .await?
        .ok_or(anyhow!("no threshold available"))?;
    let api_clients = all_ecash_api_clients(&client, epoch_id).await?;

    if api_clients.len() < threshold as usize {
        bail!(
            "we have only {} api clients available while the minimum threshold is {threshold}",
            api_clients.len()
        )
    }
    let aggregate_vk = obtain_aggregate_verification_key(&api_clients)?;

    for i in 0..args.amount {
        let human_index = i + 1;
        info!("trying to obtain free pass {human_index}/{}", args.amount);
        let free_pass = get_freepass(
            api_clients.clone(),
            &aggregate_vk,
            threshold,
            epoch_id,
            &signing_account,
            expiration_date,
        )
        .await?;
        let credential_data = Zeroizing::new(free_pass.pack_v1());
        let output = args.output_dir.join(format!("freepass_{i}.nym"));
        info!("saving the freepass to '{}'", output.display());
        File::create(output)?.write_all(&credential_data)?;
    }

    Ok(())
}
