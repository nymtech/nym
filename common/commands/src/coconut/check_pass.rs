// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::context::{QueryClient, SigningClient};
use anyhow::{anyhow, bail};
use clap::ArgGroup;
use clap::Parser;
use futures::StreamExt;
use log::{error, info};
use nym_coconut_dkg_common::types::EpochId;
use nym_credential_utils::utils::block_until_coconut_is_available;
use nym_credentials::coconut::bandwidth::bandwidth_credential_params;
use nym_credentials::coconut::bandwidth::freepass::MAX_FREE_PASS_VALIDITY;
use nym_credentials::coconut::utils;
use nym_credentials::{
    obtain_aggregate_verification_key, IssuanceBandwidthCredential, IssuedBandwidthCredential,
};
use nym_credentials_interface::VerificationKey;
use nym_validator_client::coconut::all_coconut_api_clients;
use nym_validator_client::nyxd::contract_traits::{DkgQueryClient, NymContractsProvider};
use nym_validator_client::nyxd::CosmWasmClient;
use nym_validator_client::signing::AccountData;
use nym_validator_client::CoconutApiClient;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use zeroize::Zeroizing;

#[derive(Debug, Parser)]
pub struct Args {
    /// Path to the output directory for generated free passes.
    #[clap(long)]
    pub(crate) pass: PathBuf,
}

// async fn get_freepass(
//     api_clients: Vec<CoconutApiClient>,
//     aggregate_vk: &VerificationKey,
//     threshold: u64,
//     epoch_id: EpochId,
//     signing_account: &AccountData,
//     expiration_date: OffsetDateTime,
// ) -> anyhow::Result<IssuedBandwidthCredential> {
//     let issuance_pass = IssuanceBandwidthCredential::new_freepass(Some(expiration_date));
//     let signing_data = issuance_pass.prepare_for_signing();
//
//     let credential_shares = Arc::new(tokio::sync::Mutex::new(Vec::new()));
//
//     futures::stream::iter(api_clients)
//         .for_each_concurrent(None, |client| async {
//             // move the client into the block
//             let client = client;
//             let api_url = client.api_client.api_url();
//
//             info!("contacting {api_url} for blinded free pass");
//
//             match issuance_pass
//                 .obtain_partial_freepass_credential(
//                     &client.api_client,
//                     signing_account,
//                     &client.verification_key,
//                     signing_data.clone(),
//                 )
//                 .await
//             {
//                 Ok(partial_credential) => {
//                     credential_shares
//                         .lock()
//                         .await
//                         .push((partial_credential, client.node_id).into());
//                 }
//                 Err(err) => {
//                     error!("failed to obtain partial free pass from {api_url}: {err}")
//                 }
//             }
//         })
//         .await;
//
//     // SAFETY: the futures have completed, so we MUST have the only arc reference
//     #[allow(clippy::unwrap_used)]
//     let credential_shares = Arc::into_inner(credential_shares).unwrap().into_inner();
//
//     if credential_shares.len() < threshold as usize {
//         bail!("we managed to obtain only {} partial credentials while the minimum threshold is {threshold}", credential_shares.len());
//     }
//
//     let signature = issuance_pass.aggregate_signature_shares(aggregate_vk, &credential_shares)?;
//     Ok(issuance_pass.into_issued_credential(signature, epoch_id))
// }

pub async fn execute(args: Args, client: QueryClient) -> anyhow::Result<()> {
    let raw_credential = fs::read(args.pass)?;
    let credential = IssuedBandwidthCredential::unpack_v1(&raw_credential)?;

    println!("{:?}", credential.get_plain_public_attributes());
    println!("{}", credential.epoch_id());
    println!("{:?}", credential.variant_data());

    let api_clients = all_coconut_api_clients(&client, credential.epoch_id()).await?;

    let vk = utils::obtain_aggregate_verification_key(&api_clients)?;

    let foomp = credential.prepare_for_spending(&vk)?;

    let res = foomp.verify(bandwidth_credential_params(), &vk);
    println!("res: {res}");
    Ok(())
}
