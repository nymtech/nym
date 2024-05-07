// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ecash::bandwidth::IssuanceTicketBook;
use crate::error::Error;
use log::{debug, warn};
use nym_credentials_interface::{
    aggregate_expiration_signatures, aggregate_indices_signatures, Base58, CoinIndexSignature,
    CoinIndexSignatureShare, ExpirationDateSignature, ExpirationDateSignatureShare,
    VerificationKeyAuth, WalletSignatures,
};
use nym_validator_client::client::EcashApiClient;

// so we wouldn't break all the existing imports
pub use nym_ecash_time::{cred_exp_date, ecash_date_offset, ecash_today, EcashTime};

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

pub fn obtain_aggregated_verification_key(
    _api_clients: &[EcashApiClient],
) -> Result<VerificationKeyAuth, Error> {
    // TODO:
    // let total = api_clients.len();
    // let mut rng = thread_rng();
    // let indices = sample(&mut rng, total, total);
    // for index in indices {
    //     // randomly try apis until we succeed
    //     // if let Ok(res) = api_clients[index].api_client.get_aggregated_verification_key().await {
    //     //     //
    //     // }
    // }
    todo!()
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

    let expiration_date = cred_exp_date().unix_timestamp() as u64;
    for ecash_api_client in ecash_api_clients.iter() {
        match ecash_api_client
            .api_client
            .partial_expiration_date_signatures(None)
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

pub async fn obtain_aggregate_wallet(
    voucher: &IssuanceTicketBook,
    ecash_api_clients: &[EcashApiClient],
    threshold: u64,
) -> Result<WalletSignatures, Error> {
    if ecash_api_clients.len() < threshold as usize {
        return Err(Error::NoValidatorsAvailable);
    }
    let verification_key = aggregate_verification_keys(ecash_api_clients)?;

    let request = voucher.prepare_for_signing();

    let mut wallets = Vec::with_capacity(ecash_api_clients.len());

    // TODO: optimise and query just threshold
    for ecash_api_client in ecash_api_clients.iter() {
        debug!(
            "attempting to obtain partial credential from {}",
            ecash_api_client.api_client.api_url()
        );

        match voucher
            .obtain_partial_bandwidth_voucher_credential(
                &ecash_api_client.api_client,
                ecash_api_client.node_id,
                &ecash_api_client.verification_key,
                request.clone(),
            )
            .await
        {
            Ok(wallet) => wallets.push(wallet),
            Err(err) => {
                warn!("failed to obtain partial credential from API {ecash_api_client}: {err}",);
            }
        };
    }
    if wallets.len() < threshold as usize {
        return Err(Error::NotEnoughShares);
    }

    voucher.aggregate_signature_shares(&verification_key, &wallets, request)
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
