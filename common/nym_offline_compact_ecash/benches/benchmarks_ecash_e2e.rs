// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bls12_381::Scalar;
use criterion::{criterion_group, criterion_main, Criterion};

use itertools::izip;
use rand::seq::SliceRandom;

use nym_compact_ecash::constants;
use nym_compact_ecash::identify::{identify, IdentifyResult};

use nym_compact_ecash::error::Result;
use nym_compact_ecash::scheme::expiration_date_signatures::{
    aggregate_expiration_signatures, sign_expiration_date, ExpirationDateSignature,
    PartialExpirationDateSignature,
};
use nym_compact_ecash::scheme::keygen::SecretKeyAuth;
use nym_compact_ecash::scheme::setup::{
    aggregate_indices_signatures, setup, sign_coin_indices, CoinIndexSignature, Parameters,
    PartialCoinIndexSignature,
};
use nym_compact_ecash::{
    aggregate_verification_keys, aggregate_wallets, generate_keypair_user, issue, issue_verify,
    ttp_keygen, withdrawal_request, PartialWallet, PayInfo, PublicKeyUser, SecretKeyUser,
    VerificationKeyAuth,
};

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

struct BenchCase {
    num_authorities: u64,
    threshold_p: f32,
    ll: u64,
    spend_vv: u64,
    case_nr_pub_keys: u64,
}

fn bench_compact_ecash(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark-compact-ecash");
    // group.sample_size(300);
    // group.measurement_time(Duration::from_secs(1500));

    let expiration_date = 1703721600; // Dec 28 2023
    let spend_date = Scalar::from(1701960386); // Dec 07 2023

    let case = BenchCase {
        num_authorities: 100,
        threshold_p: 0.7,
        ll: 1000,
        spend_vv: 1,
        case_nr_pub_keys: 99,
    };

    // SETUP PHASE and KEY GENERATION
    let params = setup(case.ll);

    let grp = params.grp();
    let user_keypair = generate_keypair_user(grp);
    let threshold = (case.threshold_p * case.num_authorities as f32).round() as u64;
    let authorities_keypairs = ttp_keygen(grp, threshold, case.num_authorities).unwrap();
    let secret_keys_authorities: Vec<SecretKeyAuth> = authorities_keypairs
        .iter()
        .map(|keypair| keypair.secret_key())
        .collect();
    let verification_keys_auth: Vec<VerificationKeyAuth> = authorities_keypairs
        .iter()
        .map(|keypair| keypair.verification_key())
        .collect();

    let indices: Vec<u64> = (1..case.num_authorities + 1).collect();
    let verification_key =
        aggregate_verification_keys(&verification_keys_auth, Some(&indices)).unwrap();

    // PRE-GENERATION OF THE EXPORATION DATE SIGNATURES AND THE COIN INDICES SIGNATURES
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

    // ISSUANCE PHASE
    let (req, req_info) =
        withdrawal_request(grp, &user_keypair.secret_key(), expiration_date).unwrap();

    // CLIENT BENCHMARK: prepare a single withdrawal request
    group.bench_function(
        &format!(
            "[Client] withdrawal_request_{}_authorities_{}_L_{}_threshold",
            case.num_authorities, case.ll, case.threshold_p,
        ),
        |b| {
            b.iter(|| withdrawal_request(grp, &user_keypair.secret_key(), expiration_date).unwrap())
        },
    );

    // ISSUING AUTHRORITY BENCHMARK: Benchmark the issue function
    // called by an authority to issue a blind signature on a partial wallet
    let mut rng = rand::thread_rng();
    let keypair = authorities_keypairs.choose(&mut rng).unwrap();
    group.bench_function(
        &format!(
            "[Issuing Authority] issue_partial_wallet_with_L_{}",
            case.ll,
        ),
        |b| {
            b.iter(|| {
                issue(
                    grp,
                    keypair.secret_key(),
                    user_keypair.public_key(),
                    &req,
                    expiration_date,
                )
            })
        },
    );

    let mut wallet_blinded_signatures = Vec::new();
    for auth_keypair in &authorities_keypairs {
        let blind_signature = issue(
            grp,
            auth_keypair.secret_key(),
            user_keypair.public_key(),
            &req,
            expiration_date,
        );
        wallet_blinded_signatures.push(blind_signature.unwrap());
    }

    // CLIENT BENCHMARK: verify the issued partial wallet
    let w = wallet_blinded_signatures.first().unwrap();
    let vk = verification_keys_auth.first().unwrap();
    group.bench_function(
        &format!("[Client] issue_verify_a_partial_wallet_with_L_{}", case.ll,),
        |b| b.iter(|| issue_verify(grp, vk, &user_keypair.secret_key(), w, &req_info, 1).unwrap()),
    );

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

    // CLIENT BENCHMARK: aggregating all partial wallets
    group.bench_function(
        &format!(
            "[Client] aggregate_wallets_with_L_{}_threshold_{}",
            case.ll, case.threshold_p,
        ),
        |b| {
            b.iter(|| {
                aggregate_wallets(
                    grp,
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
        grp,
        &verification_key,
        &user_keypair.secret_key(),
        &unblinded_wallet_shares,
        &req_info,
    )
    .unwrap();

    // SPENDING PHASE
    let pay_info = PayInfo {
        pay_info_bytes: [6u8; 72],
    };
    // CLIENT BENCHMARK: spend a single coin from the wallet
    group.bench_function(
        &format!(
            "[Client] spend_a_single_coin_L_{}_threshold_{}",
            case.ll, case.threshold_p,
        ),
        |b| {
            b.iter(|| {
                aggr_wallet
                    .spend(
                        &params,
                        &verification_key,
                        &user_keypair.secret_key(),
                        &pay_info,
                        false,
                        case.spend_vv,
                        dates_signatures.clone(),
                        coin_indices_signatures.clone(),
                        spend_date,
                    )
                    .unwrap()
            })
        },
    );

    let (payment, _) = aggr_wallet
        .spend(
            &params,
            &verification_key,
            &user_keypair.secret_key(),
            &pay_info,
            false,
            case.spend_vv,
            dates_signatures.clone(),
            coin_indices_signatures.clone(),
            spend_date,
        )
        .unwrap();

    // MERCHANT BENCHMARK: verify whether the submitted payment is legit
    group.bench_function(
        &format!(
            "[Merchant] spend_verify_of_a_single_payment_L_{}_threshold_{}",
            case.ll, case.threshold_p,
        ),
        |b| {
            b.iter(|| {
                payment
                    .spend_verify(&params, &verification_key, &pay_info, spend_date)
                    .unwrap()
            })
        },
    );

    // BENCHMARK IDENTIFICATION
    // Let's generate a double spending payment

    // let's reverse the spending counter in the wallet to create a double spending payment
    let current_l = aggr_wallet.l.get();
    aggr_wallet.l.set(current_l - case.spend_vv);

    let pay_info2 = PayInfo {
        pay_info_bytes: [7u8; 72],
    };
    let (payment2, _) = aggr_wallet
        .spend(
            &params,
            &verification_key,
            &user_keypair.secret_key(),
            &pay_info2,
            true,
            case.spend_vv,
            dates_signatures.clone(),
            coin_indices_signatures.clone(),
            spend_date,
        )
        .unwrap();

    //  GENERATE KEYS FOR OTHER USERS
    let mut public_keys: Vec<PublicKeyUser> = Default::default();
    for _ in 0..case.case_nr_pub_keys {
        let sk = grp.random_scalar();
        let sk_user = SecretKeyUser::from_bytes(&sk.to_bytes()).unwrap();
        let pk_user = sk_user.public_key(grp);
        public_keys.push(pk_user);
    }
    public_keys.push(user_keypair.public_key());

    // MERCHANT BENCHMARK: identify double spending
    group.bench_function(
        &format!(
            "[Merchant] identify_L_{}_threshold_{}_spend_vv_{}_pks_{}",
            case.ll,
            case.threshold_p,
            case.spend_vv,
            public_keys.len()
        ),
        |b| {
            b.iter(|| {
                identify(
                    payment.clone(),
                    payment2.clone(),
                    pay_info.clone(),
                    pay_info2.clone(),
                )
            })
        },
    );
    let identify_result = identify(payment, payment2, pay_info.clone(), pay_info2.clone());
    assert_eq!(
        identify_result,
        IdentifyResult::DoubleSpendingPublicKeys(user_keypair.public_key())
    );
}

criterion_group!(benches, bench_compact_ecash);
criterion_main!(benches);
