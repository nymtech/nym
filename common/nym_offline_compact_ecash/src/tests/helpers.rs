// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use itertools::izip;

use crate::aggregate_verification_keys;
use crate::aggregate_wallets;
use crate::error::CompactEcashError;
use crate::generate_keypair_user;
use crate::issue;
use crate::issue_verify;
use crate::scheme::keygen::KeyPairAuth;
use crate::scheme::keygen::SecretKeyAuth;
use crate::scheme::Payment;
use crate::setup::setup;
use crate::setup::GroupParameters;
use crate::tests::e2e::generate_coin_indices_signatures;
use crate::tests::e2e::generate_expiration_date_signatures;
use crate::utils;
use crate::withdrawal_request;
use crate::PartialWallet;
use crate::PayInfo;
use crate::Scalar;
use crate::VerificationKeyAuth;

pub fn payment_from_keys_and_expiration_date(
    grp_params: &GroupParameters,
    ecash_keypairs: &Vec<KeyPairAuth>,
    indices: &[utils::SignerIndex],
    expiration_date: u64,
) -> Result<(Payment, PayInfo), CompactEcashError> {
    let total_coins = 32;
    let params = setup(total_coins);
    let spend_date = Scalar::from(expiration_date - 29 * 86400);
    let user_keypair = generate_keypair_user(grp_params);

    let secret_keys_authorities: Vec<SecretKeyAuth> = ecash_keypairs
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
        &params,
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
    let (req, req_info) =
        withdrawal_request(grp_params, &user_keypair.secret_key(), expiration_date).unwrap();

    // generate blinded signatures
    let mut wallet_blinded_signatures = Vec::new();

    for keypair in ecash_keypairs {
        let blinded_signature = issue(
            grp_params,
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
        issue_verify(
            grp_params,
            vk,
            &user_keypair.secret_key(),
            w,
            &req_info,
            idx as u64 + 1,
        )
        .unwrap()
    })
    .collect();

    // Aggregate partial wallets
    let aggr_wallet = aggregate_wallets(
        grp_params,
        &verification_key,
        &user_keypair.secret_key(),
        &unblinded_wallet_shares,
        &req_info,
    )?;

    // Let's try to spend some coins
    let pay_info = PayInfo {
        pay_info_bytes: [6u8; 72],
    };
    let spend_vv = 1;

    let (payment, _) = aggr_wallet.spend(
        &params,
        &verification_key,
        &user_keypair.secret_key(),
        &pay_info,
        false,
        spend_vv,
        dates_signatures,
        coin_indices_signatures,
        spend_date,
    )?;

    Ok((payment, pay_info))
}
