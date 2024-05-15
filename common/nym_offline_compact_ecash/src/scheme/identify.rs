// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants;
use crate::error::Result;
use crate::scheme::expiration_date_signatures::{
    aggregate_expiration_signatures, sign_expiration_date, ExpirationDateSignature,
    PartialExpirationDateSignature,
};
use crate::scheme::keygen::{PublicKeyUser, SecretKeyAuth, VerificationKeyAuth};
use crate::scheme::setup::{
    aggregate_indices_signatures, sign_coin_indices, CoinIndexSignature, Parameters,
    PartialCoinIndexSignature,
};
use crate::scheme::{compute_pay_info_hash, Payment};

use crate::PayInfo;

#[derive(Debug, Eq, PartialEq)]
pub enum IdentifyResult {
    NotADuplicatePayment,
    DuplicatePayInfo(PayInfo),
    DoubleSpendingPublicKeys(PublicKeyUser),
}

pub fn identify(
    payment1: Payment,
    payment2: Payment,
    pay_info1: PayInfo,
    pay_info2: PayInfo,
) -> IdentifyResult {
    let mut k = 0;
    let mut j = 0;
    for (id1, pay1_ss) in payment1.ss.iter().enumerate() {
        for (id2, pay2_ss) in payment2.ss.iter().enumerate() {
            if pay1_ss == pay2_ss {
                k = id1;
                j = id2;
                break;
            }
        }
    }
    if payment1
        .ss
        .iter()
        .any(|pay1_ss| payment2.ss.contains(pay1_ss))
    {
        if pay_info1 == pay_info2 {
            IdentifyResult::DuplicatePayInfo(pay_info1)
        } else {
            let rr_k_payment1 = compute_pay_info_hash(&pay_info1, k as u64);
            let rr_j_payment2 = compute_pay_info_hash(&pay_info2, j as u64);
            let rr_diff = rr_k_payment1 - rr_j_payment2;
            let pk = (payment2.tt[j] * rr_k_payment1 - payment1.tt[k] * rr_j_payment2)
                * rr_diff.invert().unwrap();
            let pk_user = PublicKeyUser { pk };
            IdentifyResult::DoubleSpendingPublicKeys(pk_user)
        }
    } else {
        IdentifyResult::NotADuplicatePayment
    }
}

pub fn generate_expiration_date_signatures(
    params: &Parameters,
    expiration_date: u64,
    secret_keys_authorities: &[SecretKeyAuth],
    verification_keys_auth: &[VerificationKeyAuth],
    verification_key: &VerificationKeyAuth,
    indices: &[u64],
) -> Result<Vec<ExpirationDateSignature>> {
    let mut edt_partial_signatures: Vec<Vec<PartialExpirationDateSignature>> =
        Vec::with_capacity(constants::CRED_VALIDITY_PERIOD as usize);
    for sk_auth in secret_keys_authorities.iter() {
        let sign = sign_expiration_date(sk_auth, expiration_date);
        edt_partial_signatures.push(sign);
    }
    let combined_data: Vec<(
        u64,
        VerificationKeyAuth,
        Vec<PartialExpirationDateSignature>,
    )> = indices
        .iter()
        .zip(
            verification_keys_auth
                .iter()
                .zip(edt_partial_signatures.iter()),
        )
        .map(|(i, (vk, sigs))| (*i, vk.clone(), sigs.clone()))
        .collect();

    aggregate_expiration_signatures(params, verification_key, expiration_date, &combined_data)
}

pub fn generate_coin_indices_signatures(
    params: &Parameters,
    secret_keys_authorities: &[SecretKeyAuth],
    verification_keys_auth: &[VerificationKeyAuth],
    verification_key: &VerificationKeyAuth,
    indices: &[u64],
) -> Result<Vec<CoinIndexSignature>> {
    // create the partial signatures from each authority
    let partial_signatures: Vec<Vec<PartialCoinIndexSignature>> = secret_keys_authorities
        .iter()
        .map(|sk_auth| sign_coin_indices(params, verification_key, sk_auth))
        .collect();

    let combined_data: Vec<(u64, VerificationKeyAuth, Vec<PartialCoinIndexSignature>)> = indices
        .iter()
        .zip(verification_keys_auth.iter().zip(partial_signatures.iter()))
        .map(|(i, (vk, sigs))| (*i, vk.clone(), sigs.clone()))
        .collect();

    aggregate_indices_signatures(params, verification_key, &combined_data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheme::identify::{identify, IdentifyResult};
    use crate::scheme::keygen::{PublicKeyUser, SecretKeyUser};
    use crate::scheme::setup::setup;
    use crate::{
        aggregate_verification_keys, aggregate_wallets, generate_keypair_user, issue, issue_verify,
        ttp_keygen, withdrawal_request, PartialWallet, PayInfo, VerificationKeyAuth,
    };
    use bls12_381::Scalar;
    use itertools::izip;

    #[test]
    fn duplicate_payments_with_the_same_pay_info() {
        let total_coins = 32;
        let params = setup(total_coins);
        // NOTE: Make sure that the date timestamp are calculated at 00:00:00!!
        let expiration_date = 1703721600; // Dec 28 2023 00:00:00
        let spend_date = Scalar::from(1701907200); // Dec 07 2023 00:00:00
        let grp = params.grp();
        let user_keypair = generate_keypair_user(grp);

        let (req, req_info) =
            withdrawal_request(grp, &user_keypair.secret_key(), expiration_date).unwrap();
        let authorities_keypairs = ttp_keygen(grp, 2, 3).unwrap();
        let indices: [u64; 3] = [1, 2, 3];
        let secret_keys_authorities: Vec<SecretKeyAuth> = authorities_keypairs
            .iter()
            .map(|keypair| keypair.secret_key())
            .collect();
        let verification_keys_auth: Vec<VerificationKeyAuth> = authorities_keypairs
            .iter()
            .map(|keypair| keypair.verification_key())
            .collect();

        let verification_key =
            aggregate_verification_keys(&verification_keys_auth, Some(&[1, 2, 3])).unwrap();

        // generate valid dates signatures
        let dates_signatures = generate_expiration_date_signatures(
            &params,
            expiration_date,
            &secret_keys_authorities,
            &verification_keys_auth,
            &verification_key,
            &indices,
        )
        .unwrap();

        // generate coin indices signatures
        let coin_indices_signatures = generate_coin_indices_signatures(
            &params,
            &secret_keys_authorities,
            &verification_keys_auth,
            &verification_key,
            &indices,
        )
        .unwrap();

        let mut wallet_blinded_signatures = Vec::new();
        for auth_keypair in authorities_keypairs {
            let blind_signature = issue(
                grp,
                auth_keypair.secret_key(),
                user_keypair.public_key(),
                &req,
                expiration_date,
            );
            wallet_blinded_signatures.push(blind_signature.unwrap());
        }

        let unblinded_wallet_shares: Vec<PartialWallet> = izip!(
            wallet_blinded_signatures.iter(),
            verification_keys_auth.iter()
        )
        .enumerate()
        .map(|(idx, (w, vk))| {
            issue_verify(
                grp,
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
            grp,
            &verification_key,
            &user_keypair.secret_key(),
            &unblinded_wallet_shares,
            &req_info,
        )
        .unwrap();

        // Let's try to spend some coins
        let pay_info1 = PayInfo {
            pay_info_bytes: [6u8; 72],
        };
        let spend_vv = 1;

        let (payment1, _upd_wallet) = aggr_wallet
            .spend(
                &params,
                &verification_key,
                &user_keypair.secret_key(),
                &pay_info1,
                false,
                spend_vv,
                dates_signatures,
                coin_indices_signatures,
                spend_date,
            )
            .unwrap();

        assert!(payment1
            .spend_verify(&params, &verification_key, &pay_info1, spend_date)
            .unwrap());

        let payment2 = payment1.clone();
        assert!(payment2
            .spend_verify(&params, &verification_key, &pay_info1, spend_date)
            .unwrap());

        let pay_info2 = pay_info1.clone();
        let identify_result = identify(payment1, payment2, pay_info1.clone(), pay_info2.clone());
        assert_eq!(
            identify_result,
            IdentifyResult::DuplicatePayInfo(pay_info1.clone())
        );
    }

    #[test]
    fn ok_if_two_different_payments() {
        let total_coins = 32;
        let params = setup(total_coins);
        let grp = params.grp();
        // NOTE: Make sure that the date timestamp are calculated at 00:00:00!!
        let expiration_date = 1703721600; // Dec 28 2023 00:00:00
        let spend_date = Scalar::from(1701907200); // Dec 07 2023 00:00:00
        let user_keypair = generate_keypair_user(grp);

        let (req, req_info) =
            withdrawal_request(grp, &user_keypair.secret_key(), expiration_date).unwrap();
        let authorities_keypairs = ttp_keygen(grp, 2, 3).unwrap();
        let indices: [u64; 3] = [1, 2, 3];
        let secret_keys_authorities: Vec<SecretKeyAuth> = authorities_keypairs
            .iter()
            .map(|keypair| keypair.secret_key())
            .collect();
        let verification_keys_auth: Vec<VerificationKeyAuth> = authorities_keypairs
            .iter()
            .map(|keypair| keypair.verification_key())
            .collect();

        let verification_key =
            aggregate_verification_keys(&verification_keys_auth, Some(&[1, 2, 3])).unwrap();

        // generate valid dates signatures
        let dates_signatures = generate_expiration_date_signatures(
            &params,
            expiration_date,
            &secret_keys_authorities,
            &verification_keys_auth,
            &verification_key,
            &indices,
        )
        .unwrap();

        // generate coin indices signatures
        let coin_indices_signatures = generate_coin_indices_signatures(
            &params,
            &secret_keys_authorities,
            &verification_keys_auth,
            &verification_key,
            &indices,
        )
        .unwrap();

        let mut wallet_blinded_signatures = Vec::new();
        for auth_keypair in authorities_keypairs {
            let blind_signature = issue(
                grp,
                auth_keypair.secret_key(),
                user_keypair.public_key(),
                &req,
                expiration_date,
            );
            wallet_blinded_signatures.push(blind_signature.unwrap());
        }

        let unblinded_wallet_shares: Vec<PartialWallet> = izip!(
            wallet_blinded_signatures.iter(),
            verification_keys_auth.iter()
        )
        .enumerate()
        .map(|(idx, (w, vk))| {
            issue_verify(
                grp,
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
            grp,
            &verification_key,
            &user_keypair.secret_key(),
            &unblinded_wallet_shares,
            &req_info,
        )
        .unwrap();

        // Let's try to spend some coins
        let pay_info1 = PayInfo {
            pay_info_bytes: [6u8; 72],
        };
        let spend_vv = 1;

        let (payment1, upd_wallet) = aggr_wallet
            .spend(
                &params,
                &verification_key,
                &user_keypair.secret_key(),
                &pay_info1,
                false,
                spend_vv,
                dates_signatures.clone(),
                coin_indices_signatures.clone(),
                spend_date,
            )
            .unwrap();

        assert!(payment1
            .spend_verify(&params, &verification_key, &pay_info1, spend_date)
            .unwrap());

        let pay_info2 = PayInfo {
            pay_info_bytes: [7u8; 72],
        };
        let (payment2, _) = upd_wallet
            .spend(
                &params,
                &verification_key,
                &user_keypair.secret_key(),
                &pay_info2,
                false,
                spend_vv,
                dates_signatures,
                coin_indices_signatures,
                spend_date,
            )
            .unwrap();

        assert!(payment2
            .spend_verify(&params, &verification_key, &pay_info2, spend_date)
            .unwrap());

        let identify_result = identify(payment1, payment2, pay_info1.clone(), pay_info2.clone());
        assert_eq!(identify_result, IdentifyResult::NotADuplicatePayment);
    }

    #[test]
    fn two_payments_with_one_repeating_serial_number_but_different_pay_info() {
        let total_coins = 32;
        let params = setup(total_coins);
        let grp = params.grp();
        // NOTE: Make sure that the date timestamp are calculated at 00:00:00!!
        let expiration_date = 1703721600; // Dec 28 2023 00:00:00
        let spend_date = Scalar::from(1701907200); // Dec 07 2023 00:00:00
        let user_keypair = generate_keypair_user(grp);

        //  GENERATE KEYS FOR OTHER USERS
        let mut public_keys: Vec<PublicKeyUser> = Default::default();
        for _i in 0..50 {
            let sk = grp.random_scalar();
            let sk_user = SecretKeyUser { sk };
            let pk_user = sk_user.public_key(grp);
            public_keys.push(pk_user.clone());
        }
        public_keys.push(user_keypair.public_key().clone());

        let (req, req_info) =
            withdrawal_request(grp, &user_keypair.secret_key(), expiration_date).unwrap();
        let authorities_keypairs = ttp_keygen(grp, 2, 3).unwrap();
        let indices: [u64; 3] = [1, 2, 3];
        let secret_keys_authorities: Vec<SecretKeyAuth> = authorities_keypairs
            .iter()
            .map(|keypair| keypair.secret_key())
            .collect();
        let verification_keys_auth: Vec<VerificationKeyAuth> = authorities_keypairs
            .iter()
            .map(|keypair| keypair.verification_key())
            .collect();

        let verification_key =
            aggregate_verification_keys(&verification_keys_auth, Some(&[1, 2, 3])).unwrap();

        // generate valid dates signatures
        let dates_signatures = generate_expiration_date_signatures(
            &params,
            expiration_date,
            &secret_keys_authorities,
            &verification_keys_auth,
            &verification_key,
            &indices,
        )
        .unwrap();

        // generate coin indices signatures
        let coin_indices_signatures = generate_coin_indices_signatures(
            &params,
            &secret_keys_authorities,
            &verification_keys_auth,
            &verification_key,
            &indices,
        )
        .unwrap();

        let mut wallet_blinded_signatures = Vec::new();
        for auth_keypair in authorities_keypairs {
            let blind_signature = issue(
                grp,
                auth_keypair.secret_key(),
                user_keypair.public_key(),
                &req,
                expiration_date,
            );
            wallet_blinded_signatures.push(blind_signature.unwrap());
        }

        let unblinded_wallet_shares: Vec<PartialWallet> = izip!(
            wallet_blinded_signatures.iter(),
            verification_keys_auth.iter()
        )
        .enumerate()
        .map(|(idx, (w, vk))| {
            issue_verify(
                grp,
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
            grp,
            &verification_key,
            &user_keypair.secret_key(),
            &unblinded_wallet_shares,
            &req_info,
        )
        .unwrap();

        // Let's try to spend some coins
        let pay_info1 = PayInfo {
            pay_info_bytes: [6u8; 72],
        };
        let spend_vv = 1;

        let (payment1, _upd_wallet) = aggr_wallet
            .spend(
                &params,
                &verification_key,
                &user_keypair.secret_key(),
                &pay_info1,
                false,
                spend_vv,
                dates_signatures.clone(),
                coin_indices_signatures.clone(),
                spend_date,
            )
            .unwrap();

        assert!(payment1
            .spend_verify(&params, &verification_key, &pay_info1, spend_date)
            .unwrap());

        // let's reverse the spending counter in the wallet to create a double spending payment
        let current_l = aggr_wallet.l.get();
        aggr_wallet.l.set(current_l - 1);

        let pay_info2 = PayInfo {
            pay_info_bytes: [7u8; 72],
        };

        let (payment2, _) = aggr_wallet
            .spend(
                &params,
                &verification_key,
                &user_keypair.secret_key(),
                &pay_info2,
                false,
                spend_vv,
                dates_signatures.clone(),
                coin_indices_signatures.clone(),
                spend_date,
            )
            .unwrap();

        assert!(payment2
            .spend_verify(&params, &verification_key, &pay_info2, spend_date)
            .unwrap());

        let identify_result = identify(payment1, payment2, pay_info1.clone(), pay_info2.clone());
        assert_eq!(
            identify_result,
            IdentifyResult::DoubleSpendingPublicKeys(user_keypair.public_key())
        );
    }

    #[test]
    fn two_payments_with_multiple_repeating_serial_numbers_but_different_pay_info() {
        let total_coins = 32;
        let params = setup(total_coins);
        let grp = params.grp();
        // NOTE: Make sure that the date timestamp are calculated at 00:00:00!!
        let expiration_date = 1703721600; // Dec 28 2023 00:00:00
        let spend_date = Scalar::from(1701907200); // Dec 07 2023 00:00:00
        let user_keypair = generate_keypair_user(grp);

        //  GENERATE KEYS FOR OTHER USERS
        let mut public_keys: Vec<PublicKeyUser> = Default::default();
        for _ in 0..50 {
            let sk = grp.random_scalar();
            let sk_user = SecretKeyUser { sk };
            let pk_user = sk_user.public_key(grp);
            public_keys.push(pk_user.clone());
        }
        public_keys.push(user_keypair.public_key().clone());

        let (req, req_info) =
            withdrawal_request(grp, &user_keypair.secret_key(), expiration_date).unwrap();
        let authorities_keypairs = ttp_keygen(grp, 2, 3).unwrap();
        let indices: [u64; 3] = [1, 2, 3];
        let secret_keys_authorities: Vec<SecretKeyAuth> = authorities_keypairs
            .iter()
            .map(|keypair| keypair.secret_key())
            .collect();

        let verification_keys_auth: Vec<VerificationKeyAuth> = authorities_keypairs
            .iter()
            .map(|keypair| keypair.verification_key())
            .collect();

        let verification_key =
            aggregate_verification_keys(&verification_keys_auth, Some(&[1, 2, 3])).unwrap();

        // generate valid dates signatures
        let dates_signatures = generate_expiration_date_signatures(
            &params,
            expiration_date,
            &secret_keys_authorities,
            &verification_keys_auth,
            &verification_key,
            &indices,
        )
        .unwrap();

        // generate coin indices signatures
        let coin_indices_signatures = generate_coin_indices_signatures(
            &params,
            &secret_keys_authorities,
            &verification_keys_auth,
            &verification_key,
            &indices,
        )
        .unwrap();

        let mut wallet_blinded_signatures = Vec::new();
        for auth_keypair in authorities_keypairs {
            let blind_signature = issue(
                grp,
                auth_keypair.secret_key(),
                user_keypair.public_key(),
                &req,
                expiration_date,
            );
            wallet_blinded_signatures.push(blind_signature.unwrap());
        }

        let unblinded_wallet_shares: Vec<PartialWallet> = izip!(
            wallet_blinded_signatures.iter(),
            verification_keys_auth.iter()
        )
        .enumerate()
        .map(|(idx, (w, vk))| {
            issue_verify(
                grp,
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
            grp,
            &verification_key,
            &user_keypair.secret_key(),
            &unblinded_wallet_shares,
            &req_info,
        )
        .unwrap();

        // Let's try to spend some coins
        let pay_info1 = PayInfo {
            pay_info_bytes: [6u8; 72],
        };
        let spend_vv = 10;

        let (payment1, _) = aggr_wallet
            .spend(
                &params,
                &verification_key,
                &user_keypair.secret_key(),
                &pay_info1,
                false,
                spend_vv,
                dates_signatures.clone(),
                coin_indices_signatures.clone(),
                spend_date,
            )
            .unwrap();

        assert!(payment1
            .spend_verify(&params, &verification_key, &pay_info1, spend_date)
            .unwrap());

        // let's reverse the spending counter in the wallet to create a double spending payment
        let current_l = aggr_wallet.l.get();
        aggr_wallet.l.set(current_l - 10);

        let pay_info2 = PayInfo {
            pay_info_bytes: [7u8; 72],
        };
        let (payment2, _) = aggr_wallet
            .spend(
                &params,
                &verification_key,
                &user_keypair.secret_key(),
                &pay_info2,
                false,
                spend_vv,
                dates_signatures.clone(),
                coin_indices_signatures.clone(),
                spend_date,
            )
            .unwrap();

        let identify_result = identify(payment1, payment2, pay_info1.clone(), pay_info2.clone());
        assert_eq!(
            identify_result,
            IdentifyResult::DoubleSpendingPublicKeys(user_keypair.public_key())
        );
    }
}
