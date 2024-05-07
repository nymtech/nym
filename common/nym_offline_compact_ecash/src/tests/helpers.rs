// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use itertools::izip;

use crate::common_types::SignerIndex;
use crate::error::Result;
use crate::scheme::coin_indices_signatures::{
    aggregate_indices_signatures, sign_coin_indices, CoinIndexSignature, CoinIndexSignatureShare,
    PartialCoinIndexSignature,
};
use crate::scheme::expiration_date_signatures::{
    aggregate_expiration_signatures, sign_expiration_date, ExpirationDateSignature,
    ExpirationDateSignatureShare, PartialExpirationDateSignature,
};
use crate::scheme::keygen::{KeyPairAuth, SecretKeyAuth};
use crate::scheme::Payment;
use crate::setup::Parameters;
use crate::{
    aggregate_verification_keys, aggregate_wallets, constants, generate_keypair_user, issue,
    issue_verify, withdrawal_request, PartialWallet, PayInfo, Scalar, VerificationKeyAuth,
};

pub fn generate_expiration_date_signatures(
    expiration_date: u64,
    secret_keys_authorities: &[&SecretKeyAuth],
    verification_keys_auth: &[VerificationKeyAuth],
    verification_key: &VerificationKeyAuth,
    indices: &[u64],
) -> Result<Vec<ExpirationDateSignature>> {
    let mut edt_partial_signatures: Vec<Vec<PartialExpirationDateSignature>> =
        Vec::with_capacity(constants::CRED_VALIDITY_PERIOD as usize);
    for sk_auth in secret_keys_authorities.iter() {
        //Test helpers
        #[allow(clippy::unwrap_used)]
        let sign = sign_expiration_date(sk_auth, expiration_date).unwrap();
        edt_partial_signatures.push(sign);
    }
    let combined_data: Vec<_> = indices
        .iter()
        .zip(
            verification_keys_auth
                .iter()
                .zip(edt_partial_signatures.iter()),
        )
        .map(|(i, (vk, sigs))| ExpirationDateSignatureShare {
            index: *i,
            key: vk.clone(),
            signatures: sigs.to_vec(),
        })
        .collect();

    aggregate_expiration_signatures(verification_key, expiration_date, &combined_data)
}

pub fn generate_coin_indices_signatures(
    params: &Parameters,
    secret_keys_authorities: &[&SecretKeyAuth],
    verification_keys_auth: &[VerificationKeyAuth],
    verification_key: &VerificationKeyAuth,
    indices: &[u64],
) -> Result<Vec<CoinIndexSignature>> {
    // create the partial signatures from each authority
    //Test helpers
    #[allow(clippy::unwrap_used)]
    let partial_signatures: Vec<Vec<PartialCoinIndexSignature>> = secret_keys_authorities
        .iter()
        .map(|sk_auth| sign_coin_indices(params, verification_key, sk_auth).unwrap())
        .collect();

    let combined_data: Vec<_> = indices
        .iter()
        .zip(verification_keys_auth.iter().zip(partial_signatures.iter()))
        .map(|(i, (vk, sigs))| CoinIndexSignatureShare {
            index: *i,
            key: vk.clone(),
            signatures: sigs.to_vec(),
        })
        .collect();

    aggregate_indices_signatures(params, verification_key, &combined_data)
}

pub fn payment_from_keys_and_expiration_date(
    ecash_keypairs: &Vec<KeyPairAuth>,
    indices: &[SignerIndex],
    expiration_date: u64,
) -> Result<(Payment, PayInfo)> {
    let total_coins = 32;
    let params = Parameters::new(total_coins);
    let spend_date = Scalar::from(expiration_date - 29 * 86400);
    let user_keypair = generate_keypair_user();

    let secret_keys_authorities: Vec<&SecretKeyAuth> = ecash_keypairs
        .iter()
        .map(|keypair| keypair.secret_key())
        .collect();
    let verification_keys_auth: Vec<VerificationKeyAuth> = ecash_keypairs
        .iter()
        .map(|keypair| keypair.verification_key())
        .collect();

    // aggregate verification keys
    let verification_key = aggregate_verification_keys(&verification_keys_auth, Some(indices))?;

    // generate valid dates signatures
    let dates_signatures = generate_expiration_date_signatures(
        expiration_date,
        &secret_keys_authorities,
        &verification_keys_auth,
        &verification_key,
        indices,
    )?;

    // generate coin indices signatures
    let coin_indices_signatures = generate_coin_indices_signatures(
        &params,
        &secret_keys_authorities,
        &verification_keys_auth,
        &verification_key,
        indices,
    )?;
    //SAFETY : method intended for test only
    #[allow(clippy::unwrap_used)]
    // request a wallet
    let (req, req_info) = withdrawal_request(user_keypair.secret_key(), expiration_date).unwrap();

    // generate blinded signatures
    let mut wallet_blinded_signatures = Vec::new();

    for keypair in ecash_keypairs {
        let blinded_signature = issue(
            keypair.secret_key(),
            user_keypair.public_key(),
            &req,
            expiration_date,
        )?;
        wallet_blinded_signatures.push(blinded_signature)
    }

    // Unblind
    //SAFETY : method intended for test only
    #[allow(clippy::unwrap_used)]
    let unblinded_wallet_shares: Vec<PartialWallet> = izip!(
        wallet_blinded_signatures.iter(),
        verification_keys_auth.iter()
    )
    .enumerate()
    .map(|(idx, (w, vk))| {
        issue_verify(vk, user_keypair.secret_key(), w, &req_info, idx as u64 + 1).unwrap()
    })
    .collect();

    // Aggregate partial wallets
    let mut aggr_wallet = aggregate_wallets(
        &verification_key,
        user_keypair.secret_key(),
        &unblinded_wallet_shares,
        &req_info,
    )?;

    // Let's try to spend some coins
    let pay_info = PayInfo {
        pay_info_bytes: [6u8; 72],
    };
    let spend_vv = 1;

    let payment = aggr_wallet.spend(
        &params,
        &verification_key,
        user_keypair.secret_key(),
        &pay_info,
        spend_vv,
        &dates_signatures,
        &coin_indices_signatures,
        spend_date,
    )?;

    Ok((payment, pay_info))
}
