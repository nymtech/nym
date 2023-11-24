// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::ops::Neg;
use std::time::Duration;

use bls12_381::{
    multi_miller_loop, G1Affine, G1Projective, G2Affine, G2Prepared, G2Projective, Gt, Scalar,
};
use criterion::{criterion_group, criterion_main, Criterion};
use ff::Field;
use group::{Curve, Group};
use itertools::izip;
use rand::seq::SliceRandom;

use nym_compact_ecash::constants;
use nym_compact_ecash::identify::{identify, IdentifyResult};

use nym_compact_ecash::scheme::keygen::SecretKeyAuth;
use nym_compact_ecash::setup::setup;
use nym_compact_ecash::{
    aggregate_verification_keys, aggregate_wallets, generate_keypair_user, issue_verify,
    issue_wallet, ttp_keygen, withdrawal_request, PartialWallet, PayInfo, PublicKeyUser,
    SecretKeyUser, VerificationKeyAuth,
};

struct BenchCase {
    num_authorities: u64,
    threshold_p: f32,
    L: u64,
    spend_vv: u64,
    case_nr_pub_keys: u64,
}

fn bench_compact_ecash(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark-compact-ecash");
    // group.sample_size(300);
    // group.measurement_time(Duration::from_secs(1500));

    let case = BenchCase {
        num_authorities: 100,
        threshold_p: 0.7,
        L: 100,
        spend_vv: 1,
        case_nr_pub_keys: 99,
    };

    let params = setup(case.L);

    let grp = params.grp();
    let user_keypair = generate_keypair_user(&grp);
    let threshold = (case.threshold_p * case.num_authorities as f32).round() as u64;
    let authorities_keypairs = ttp_keygen(&grp, threshold, case.num_authorities).unwrap();
    let verification_keys_auth: Vec<VerificationKeyAuth> = authorities_keypairs
        .iter()
        .map(|keypair| keypair.verification_key())
        .collect();

    let indices: Vec<u64> = (1..case.num_authorities + 1).collect();
    let verification_key =
        aggregate_verification_keys(&verification_keys_auth, Some(&indices)).unwrap();
    // ISSUANCE PHASE

    let (req, req_info) = withdrawal_request(grp, &user_keypair.secret_key()).unwrap();

    // CLIENT BENCHMARK: prepare a single withdrawal request
    group.bench_function(
        &format!(
            "[Client] withdrawal_request_{}_authorities_{}_L_{}_threshold",
            case.num_authorities, case.L, case.threshold_p,
        ),
        |b| b.iter(|| withdrawal_request(grp, &user_keypair.secret_key()).unwrap()),
    );

    // ISSUING AUTHRORITY BENCHMARK: Benchmark the issue_wallet function
    // called by an authority to issue a blind signature on a partial wallet
    let mut rng = rand::thread_rng();
    let keypair = authorities_keypairs.choose(&mut rng).unwrap();
    group.bench_function(
        &format!("[Issuing Authority] issue_partial_wallet_with_L_{}", case.L,),
        |b| {
            b.iter(|| {
                issue_wallet(&grp, keypair.secret_key(), user_keypair.public_key(), &req).unwrap()
            })
        },
    );

    let mut wallet_blinded_signatures = Vec::new();
    for auth_keypair in authorities_keypairs {
        let blind_signature = issue_wallet(
            &grp,
            auth_keypair.secret_key(),
            user_keypair.public_key(),
            &req,
        );
        wallet_blinded_signatures.push(blind_signature.unwrap());
    }

    // CLIENT BENCHMARK: verify the issued partial wallet
    let w = wallet_blinded_signatures.get(0).clone().unwrap();
    let vk = verification_keys_auth.get(0).clone().unwrap();
    group.bench_function(
        &format!("[Client] issue_verify_a_partial_wallet_with_L_{}", case.L,),
        |b| b.iter(|| issue_verify(&grp, vk, &user_keypair.secret_key(), w, &req_info).unwrap()),
    );

    let unblinded_wallet_shares: Vec<PartialWallet> = izip!(
        wallet_blinded_signatures.iter(),
        verification_keys_auth.iter()
    )
    .map(|(w, vk)| issue_verify(&grp, vk, &user_keypair.secret_key(), w, &req_info).unwrap())
    .collect();

    // CLIENT BENCHMARK: aggregating all partial wallets
    group.bench_function(
        &format!(
            "[Client] aggregate_wallets_with_L_{}_threshold_{}",
            case.L, case.threshold_p,
        ),
        |b| {
            b.iter(|| {
                aggregate_wallets(
                    &grp,
                    &verification_key,
                    &user_keypair.secret_key(),
                    &unblinded_wallet_shares,
                    &req_info,
                )
                .unwrap()
            })
        },
    );

    // Aggregate partial wallets
    let aggr_wallet = aggregate_wallets(
        &grp,
        &verification_key,
        &user_keypair.secret_key(),
        &unblinded_wallet_shares,
        &req_info,
    )
    .unwrap();

    // SPENDING PHASE
    let pay_info = PayInfo { payinfo: [6u8; 88] };
    // CLIENT BENCHMARK: spend a single coin from the wallet
    group.bench_function(
        &format!(
            "[Client] spend_a_single_coin_L_{}_threshold_{}",
            case.L, case.threshold_p,
        ),
        |b| {
            b.iter(|| {
                aggr_wallet
                    .spend(
                        &params,
                        &verification_key,
                        &user_keypair.secret_key(),
                        &pay_info,
                        true,
                        case.spend_vv,
                    )
                    .unwrap()
            })
        },
    );

    let (payment, upd_wallet) = aggr_wallet
        .spend(
            &params,
            &verification_key,
            &user_keypair.secret_key(),
            &pay_info,
            false,
            case.spend_vv,
        )
        .unwrap();

    // MERCHANT BENCHMARK: verify whether the submitted payment is legit
    group.bench_function(
        &format!(
            "[Merchant] spend_verify_of_a_single_payment_L_{}_threshold_{}",
            case.L, case.threshold_p,
        ),
        |b| {
            b.iter(|| {
                payment
                    .spend_verify(&params, &verification_key, &pay_info)
                    .unwrap()
            })
        },
    );

    // BENCHMARK IDENTIFICATION
    // Let's generate a double spending payment

    // let's reverse the spending counter in the wallet to create a double spending payment
    let current_l = aggr_wallet.l.get();
    aggr_wallet.l.set(current_l - case.spend_vv);

    let pay_info2 = PayInfo { payinfo: [7u8; 88] };
    let (payment2, _) = aggr_wallet
        .spend(
            &params,
            &verification_key,
            &user_keypair.secret_key(),
            &pay_info2,
            true,
            case.spend_vv,
        )
        .unwrap();

    //  GENERATE KEYS FOR OTHER USERS
    let mut public_keys: Vec<PublicKeyUser> = Default::default();
    for i in 0..case.case_nr_pub_keys {
        let sk = grp.random_scalar();
        let sk_user = SecretKeyUser { sk };
        let pk_user = sk_user.public_key(&grp);
        public_keys.push(pk_user);
    }
    public_keys.push(user_keypair.public_key());

    // MERCHANT BENCHMARK: identify double spending
    group.bench_function(
        &format!(
            "[Merchant] identify_L_{}_threshold_{}_spend_vv_{}_pks_{}",
            case.L,
            case.threshold_p,
            case.spend_vv,
            public_keys.len()
        ),
        |b| {
            b.iter(|| {
                identify(
                    &params,
                    &verification_key,
                    payment.clone(),
                    payment2.clone(),
                    pay_info.clone(),
                    pay_info2.clone(),
                )
                .unwrap()
            })
        },
    );
    let identify_result = identify(
        &params,
        &verification_key,
        payment,
        payment2,
        pay_info.clone(),
        pay_info2.clone(),
    )
    .unwrap();
    assert_eq!(
        identify_result,
        IdentifyResult::DoubleSpendingPublicKeys(user_keypair.public_key())
    );
}

criterion_group!(benches, bench_compact_ecash);
criterion_main!(benches);
