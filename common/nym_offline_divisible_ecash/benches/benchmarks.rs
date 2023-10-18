use std::collections::HashSet;
use std::ops::Neg;
use std::time::Duration;

use bls12_381::{G1Affine, G1Projective, G2Affine, G2Prepared, G2Projective, Gt, multi_miller_loop, Scalar};
use criterion::{Criterion, criterion_group, criterion_main};
use ff::Field;
use group::{Curve, Group};
use itertools::izip;
use rand::seq::SliceRandom;
use rand::thread_rng;

use nym_offline_divisible_ecash::{aggregate_verification_keys, aggregate_wallets,
                                  issue, issue_verify, PartialWallet,
                                  PayInfo, PublicKeyUser, SecretKeyUser, ttp_keygen_authorities, ttp_keygen_users, VerificationKeyAuth, withdrawal_request};
use nym_offline_divisible_ecash::identification::{identify, IdentifyResult};
use nym_offline_divisible_ecash::setup::{GroupParameters, Parameters};

#[allow(unused)]
fn double_pairing(g11: &G1Affine, g21: &G2Affine, g12: &G1Affine, g22: &G2Affine) {
    let gt1 = bls12_381::pairing(g11, g21);
    let gt2 = bls12_381::pairing(g12, g22);
    assert_eq!(gt1, gt2)
}

#[allow(unused)]
fn single_pairing(g11: &G1Affine, g21: &G2Affine) {
    let gt1 = bls12_381::pairing(g11, g21);
}

#[allow(unused)]
fn exponent_in_g1(g1: G1Projective, r: Scalar) {
    let g11 = (g1 * r);
}

#[allow(unused)]
fn exponent_in_g2(g2: G2Projective, r: Scalar) {
    let g22 = (g2 * r);
}

#[allow(unused)]
fn exponent_in_gt(gt: Gt, r: Scalar) {
    let gtt = (gt * r);
}

#[allow(unused)]
fn multi_miller_pairing_affine(g11: &G1Affine, g21: &G2Affine, g12: &G1Affine, g22: &G2Affine) {
    let miller_loop_result = multi_miller_loop(&[
        (g11, &G2Prepared::from(*g21)),
        (&g12.neg(), &G2Prepared::from(*g22)),
    ]);
    assert!(bool::from(
        miller_loop_result.final_exponentiation().is_identity()
    ))
}

#[allow(unused)]
fn multi_miller_pairing_with_prepared(
    g11: &G1Affine,
    g21: &G2Prepared,
    g12: &G1Affine,
    g22: &G2Prepared,
) {
    let miller_loop_result = multi_miller_loop(&[(g11, g21), (&g12.neg(), g22)]);
    assert!(bool::from(
        miller_loop_result.final_exponentiation().is_identity()
    ))
}

// the case of being able to prepare G2 generator
#[allow(unused)]
fn multi_miller_pairing_with_semi_prepared(
    g11: &G1Affine,
    g21: &G2Affine,
    g12: &G1Affine,
    g22: &G2Prepared,
) {
    let miller_loop_result =
        multi_miller_loop(&[(g11, &G2Prepared::from(*g21)), (&g12.neg(), g22)]);
    assert!(bool::from(
        miller_loop_result.final_exponentiation().is_identity()
    ))
}

#[allow(unused)]
fn bench_pairings(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark-pairings");
    group.measurement_time(Duration::from_secs(200));

    let mut rng = rand::thread_rng();

    let g1 = G1Affine::generator();
    let g2 = G2Affine::generator();
    let r = Scalar::random(&mut rng);
    let s = Scalar::random(&mut rng);

    let g11 = (g1 * r).to_affine();
    let g21 = (g2 * s).to_affine();
    let g21_prep = G2Prepared::from(g21);

    let g12 = (g1 * s).to_affine();
    let g22 = (g2 * r).to_affine();
    let g22_prep = G2Prepared::from(g22);

    let gt = bls12_381::pairing(&g11, &g21);
    let gen1 = G1Projective::generator();
    let gen2 = G2Projective::generator();

    group.bench_function("exponent operation in G1", |b| {
        b.iter(|| exponent_in_g1(gen1, r))
    });

    group.bench_function("exponent operation in G2", |b| {
        b.iter(|| exponent_in_g2(gen2, r))
    });

    group.bench_function("exponent operation in Gt", |b| {
        b.iter(|| exponent_in_gt(gt, r))
    });

    group.bench_function("single pairing", |b| {
        b.iter(|| single_pairing(&g11, &g21))
    });

    group.bench_function("double pairing", |b| {
        b.iter(|| double_pairing(&g11, &g21, &g12, &g22))
    });

    group.bench_function("multi miller in affine", |b| {
        b.iter(|| multi_miller_pairing_affine(&g11, &g21, &g12, &g22))
    });

    group.bench_function("multi miller with prepared g2", |b| {
        b.iter(|| multi_miller_pairing_with_prepared(&g11, &g21_prep, &g12, &g22_prep))
    });

    group.bench_function("multi miller with semi-prepared g2", |b| {
        b.iter(|| multi_miller_pairing_with_semi_prepared(&g11, &g21, &g12, &g22_prep))
    });
}

struct BenchCase {
    num_authorities: u64,
    threshold_p: f32,
    L: u64,
    spend_vv: u64,
    case_nr_pub_keys: u64,
}

fn bench_divisible_ecash(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark-divisible-ecash");
    group.sample_size(300);
    group.measurement_time(Duration::from_secs(1500));

    let case = BenchCase {
        num_authorities: 100,
        threshold_p: 0.7,
        L: 100,
        spend_vv: 10,
        case_nr_pub_keys: 99,
    };

    // SETUP PHASE
    let grp = GroupParameters::new().unwrap();
    let params = Parameters::new(grp.clone());

    // KEY GENERATION FOR THE AUTHORITIES
    let threshold = (case.threshold_p * case.num_authorities as f32).round() as u64;
    let authorities_keypairs = ttp_keygen_authorities(&params, threshold, case.num_authorities).unwrap();
    let verification_keys_auth: Vec<VerificationKeyAuth> = authorities_keypairs
        .iter()
        .map(|keypair| keypair.verification_key())
        .collect();

    let indices: Vec<u64> = (1..case.num_authorities + 1).collect();
    let verification_key =
        aggregate_verification_keys(&verification_keys_auth, Some(&indices)).unwrap();

    // KEY GENERATION FOR THE USER
    let sk = grp.random_scalar();
    let sk_user = SecretKeyUser { sk };
    let pk_user = SecretKeyUser::public_key(&sk_user, &grp);

    //  GENERATE KEYS FOR OTHER USERS
    let mut pk_all_users = HashSet::new();
    for i in 0..50 {
        let sk = grp.random_scalar();
        let sk_user = SecretKeyUser { sk };
        let pk_user = sk_user.public_key(&grp);
        pk_all_users.insert(pk_user);
    }
    pk_all_users.insert(pk_user.clone());

    // WITHDRAWAL REQUEST
    let (withdrawal_req, req_info) = withdrawal_request(&params, &sk_user).unwrap();

    // CLIENT BENCHMARK: prepare a single withdrawal request
    group.bench_function(
        &format!(
            "[Client] withdrawal_request_{}_authorities_{}_L_{}_threshold",
            case.num_authorities, case.L, case.threshold_p,
        ),
        |b| b.iter(|| withdrawal_request(&params, &sk_user).unwrap()),
    );

    // ISSUE PARTIAL WALLETS
    // first one meaningful one just for benchmark
    let mut rng = rand::thread_rng();
    let keypair = authorities_keypairs.choose(&mut rng).unwrap();
    group.bench_function(
        &format!("[Issuing Authority] issue_partial_wallet_with_L_{}", case.L, ),
        |b| {
            b.iter(|| {
                issue(
                    &params,
                    &withdrawal_req,
                    pk_user.clone(),
                    &keypair.secret_key(),
                ).unwrap()
            })
        },
    );


    let mut wallet_blinded_signatures = Vec::new();
    for auth_keypair in authorities_keypairs {
        let blind_signature = issue(
            &params,
            &withdrawal_req,
            pk_user.clone(),
            &auth_keypair.secret_key(),
        ).unwrap();
        wallet_blinded_signatures.push(blind_signature);
    }

    // CLIENT BENCHMARK: verify the issued partial wallet
    let w = wallet_blinded_signatures.get(0).clone().unwrap();
    let vk = verification_keys_auth.get(0).clone().unwrap();
    group.bench_function(
        &format!("[Client] issue_verify_a_partial_wallet_with_L_{}", case.L, ),
        |b| b.iter(|| issue_verify(&grp, vk, &sk_user, w, &req_info).unwrap()),
    );

    let partial_wallets: Vec<PartialWallet> = izip!(
        wallet_blinded_signatures.iter(),
        verification_keys_auth.iter()
    )
        .map(|(w, vk)| issue_verify(&grp, &vk, &sk_user, &w, &req_info).unwrap())
        .collect();


    // AGGREGATE WALLET
    let mut wallet = aggregate_wallets(&grp, &verification_key, &sk_user, &partial_wallets).unwrap();

    // CLIENT BENCHMARK: aggregating all partial wallets
    group.bench_function(
        &format!(
            "[Client] aggregate_wallets_with_L_{}_threshold_{}",
            case.L, case.threshold_p,
        ),
        |b| {
            b.iter(|| {
                aggregate_wallets(&grp, &verification_key, &sk_user, &partial_wallets)
                    .unwrap()
            })
        },
    );

    let pay_info = PayInfo { info: [67u8; 32] };
    let (payment, wallet) = wallet.spend(&params, &verification_key, &sk_user, &pay_info, case.spend_vv, false).unwrap();

    // CLIENT BENCHMARK: spend a single coin from the wallet
    group.bench_function(
        &format!(
            "[Client] spend_a_single_coin_L_{}_threshold_{}",
            case.L, case.threshold_p,
        ),
        |b| {
            b.iter(|| {
                wallet.spend(&params, &verification_key, &sk_user, &pay_info, case.spend_vv, true)
                    .unwrap()
            })
        },
    );

    // MERCHANT BENCHMARK: verify whether the submitted payment is legit
    group.bench_function(
        &format!(
            "[Merchant] spend_verify_of_a_single_payment_L_{}_threshold_{}",
            case.L, case.threshold_p,
        ),
        |b| {
            b.iter(|| {
                payment.spend_verify(&params, &verification_key, &pay_info)
                    .unwrap()
            })
        },
    );

    // BENCHMARK IDENTIFICATION
    // Let's generate a double spending payment

    // let's reverse the spending counter in the wallet to create a double spending payment
    let current_l = wallet.l();
    wallet.l.set(current_l - 1);

    let pay_info2 = PayInfo { info: [52u8; 32] };
    let (payment2, wallet) = wallet.spend(&params, &verification_key, &sk_user, &pay_info2, 10, false).unwrap();

    // MERCHANT BENCHMARK: identify double spending
    group.bench_function(
        &format!(
            "[Merchant] identify_L_{}_threshold_{}_spend_vv_{}_pks_{}",
            case.L, case.threshold_p, case.spend_vv, pk_all_users.len()
        ),
        |b| {
            b.iter(|| {
                identify(&params, &verification_key, &pk_all_users, payment.clone(), payment2.clone(), pay_info, pay_info2).unwrap()
            })
        },
    );


    let identify_result = identify(&params, &verification_key, &pk_all_users, payment.clone(), payment2.clone(), pay_info, pay_info2).unwrap();
    assert_eq!(identify_result, IdentifyResult::DoubleSpendingPublicKeys(pk_user));
}

criterion_group!(benches, bench_divisible_ecash);
criterion_main!(benches);