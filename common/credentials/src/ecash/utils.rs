// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ecash::bandwidth::{CredentialSigningData, IssuanceTicketBook};
use crate::error::Error;
use futures::stream::{FuturesUnordered, StreamExt};
use log::{debug, warn};
use nym_credentials_interface::{
    aggregate_expiration_signatures, aggregate_indices_signatures, Base58, CoinIndexSignature,
    CoinIndexSignatureShare, ExpirationDateSignature, ExpirationDateSignatureShare, PartialWallet,
    VerificationKeyAuth, WalletSignatures,
};
use nym_validator_client::client::EcashApiClient;
use nym_validator_client::nym_api::{EpochId, NymApiClientExt};
use rand::seq::SliceRandom;
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use tokio::time::timeout;

#[cfg(target_arch = "wasm32")]
use wasmtimer::tokio::timeout;

/// Blind signing requests sent beyond the threshold, so that this many unresponsive signers
/// don't delay the issuance.
pub const SPARE_SIGNER_REQUESTS: usize = 2;

/// Upper bound on a single signer's response to a blind signing request.
pub const SIGNER_REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

// so we wouldn't break all the existing imports
pub use nym_ecash_time::{
    cred_exp_date, ecash_date_offset, ecash_default_expiration_date, ecash_today, EcashTime,
};

pub fn aggregate_verification_keys(
    api_clients: &[EcashApiClient],
) -> Result<VerificationKeyAuth, Error> {
    if api_clients.is_empty() {
        return Err(Error::NoValidatorsAvailable);
    }

    let indices: Vec<_> = api_clients
        .iter()
        .map(|api_client| api_client.node_id)
        .collect();
    let shares: Vec<_> = api_clients
        .iter()
        .map(|api_client| api_client.verification_key.clone())
        .collect();

    Ok(nym_credentials_interface::aggregate_verification_keys(
        &shares,
        Some(&indices),
    )?)
}

pub async fn obtain_expiration_date_signatures(
    ecash_api_clients: &[EcashApiClient],
    verification_key: &VerificationKeyAuth,
    threshold: u64,
) -> Result<Vec<ExpirationDateSignature>, Error> {
    if ecash_api_clients.is_empty() {
        return Err(Error::NoValidatorsAvailable);
    }

    let mut signatures_shares: Vec<_> = Vec::with_capacity(ecash_api_clients.len());

    let expiration_date = cred_exp_date().ecash_unix_timestamp();
    for ecash_api_client in ecash_api_clients.iter() {
        match ecash_api_client
            .api_client
            .partial_expiration_date_signatures(None, None)
            .await
        {
            Ok(signature) => {
                let index = ecash_api_client.node_id;
                let key_share = ecash_api_client.verification_key.clone();
                signatures_shares.push(ExpirationDateSignatureShare {
                    index,
                    key: key_share,
                    signatures: signature.signatures,
                });
            }
            Err(err) => {
                warn!(
                    "failed to obtain expiration date signature from {}: {err}",
                    ecash_api_client.api_client.api_url()
                );
            }
        }
    }

    if signatures_shares.len() < threshold as usize {
        return Err(Error::NotEnoughShares);
    }

    //this already takes care of partial signatures validation
    aggregate_expiration_signatures(verification_key, expiration_date, &signatures_shares)
        .map_err(Error::CompactEcashError)
}

pub async fn obtain_coin_indices_signatures(
    ecash_api_clients: &[EcashApiClient],
    verification_key: &VerificationKeyAuth,
    threshold: u64,
) -> Result<Vec<CoinIndexSignature>, Error> {
    if ecash_api_clients.is_empty() {
        return Err(Error::NoValidatorsAvailable);
    }

    let mut signatures_shares: Vec<_> = Vec::with_capacity(ecash_api_clients.len());

    for ecash_api_client in ecash_api_clients.iter() {
        match ecash_api_client
            .api_client
            .partial_coin_indices_signatures(None)
            .await
        {
            Ok(signature) => {
                let index = ecash_api_client.node_id;
                let key_share = ecash_api_client.verification_key.clone();
                signatures_shares.push(CoinIndexSignatureShare {
                    index,
                    key: key_share,
                    signatures: signature.signatures,
                });
            }
            Err(err) => {
                warn!(
                    "failed to obtain expiration date signature from {}: {err}",
                    ecash_api_client.api_client.api_url()
                );
            }
        }
    }

    if signatures_shares.len() < threshold as usize {
        return Err(Error::NotEnoughShares);
    }

    //this takes care of validating partial signatures
    aggregate_indices_signatures(
        nym_credentials_interface::ecash_parameters(),
        verification_key,
        &signatures_shares,
    )
    .map_err(Error::CompactEcashError)
}

/// `epoch_id` is the epoch whose signers `ecash_api_clients` describes, and is stated on every
/// request so that a ceremony concluding partway through the collection cannot leave some shares
/// signed under one epoch and the rest under another - which could never be aggregated.
pub async fn obtain_aggregate_wallet(
    voucher: &IssuanceTicketBook,
    ecash_api_clients: &[EcashApiClient],
    threshold: u64,
    epoch_id: EpochId,
) -> Result<WalletSignatures, Error> {
    if ecash_api_clients.len() < threshold as usize {
        return Err(Error::NoValidatorsAvailable);
    }
    let verification_key = aggregate_verification_keys(ecash_api_clients)?;

    let request = voucher.prepare_for_signing();

    // random order so that no fixed subset of signers ends up doing all the work
    let mut signers: Vec<_> = ecash_api_clients.iter().collect();
    signers.shuffle(&mut rand::rng());
    let mut signers = signers.into_iter();

    let mut in_flight = FuturesUnordered::new();
    for signer in signers
        .by_ref()
        .take(threshold as usize + SPARE_SIGNER_REQUESTS)
    {
        in_flight.push(obtain_partial_wallet(voucher, signer, &request, epoch_id));
    }

    let mut wallets = Vec::with_capacity(threshold as usize);
    while let Some(res) = in_flight.next().await {
        match res {
            Ok(wallet) => {
                wallets.push(wallet);
                if wallets.len() >= threshold as usize {
                    break;
                }
            }
            Err(_) => {
                if let Some(signer) = signers.next() {
                    in_flight.push(obtain_partial_wallet(voucher, signer, &request, epoch_id));
                }
            }
        }
    }
    // abandon the requests still in flight, we have all the shares we need
    drop(in_flight);

    if wallets.len() < threshold as usize {
        return Err(Error::NotEnoughShares);
    }

    voucher.aggregate_signature_shares(&verification_key, &wallets, request)
}

async fn obtain_partial_wallet(
    voucher: &IssuanceTicketBook,
    signer: &EcashApiClient,
    request: &CredentialSigningData,
    epoch_id: EpochId,
) -> Result<PartialWallet, Error> {
    debug!("attempting to obtain partial credential from {signer}");

    let res = timeout(
        SIGNER_REQUEST_TIMEOUT,
        voucher.obtain_partial_ticketbook_credential(
            &signer.api_client,
            signer.node_id,
            &signer.verification_key,
            request.clone(),
            epoch_id,
        ),
    )
    .await
    .unwrap_or_else(|_| {
        Err(Error::CredentialShareTimeout {
            signer: signer.to_string(),
        })
    });

    if let Err(err) = &res {
        warn!("failed to obtain partial credential from {signer}: {err}");
    }
    res
}

pub fn signatures_to_string<B>(sigs: &[B]) -> String
where
    B: Base58,
{
    sigs.iter()
        .map(|sig| sig.to_bs58())
        .collect::<Vec<_>>()
        .join(",")
}

pub fn signatures_from_string<B>(bs58_sigs: String) -> Result<Vec<B>, Error>
where
    B: Base58,
{
    bs58_sigs
        .split(',')
        .map(B::try_from_bs58)
        .collect::<Result<Vec<_>, _>>()
        .map_err(Error::CompactEcashError)
}
