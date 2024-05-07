// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use criterion::{criterion_group, criterion_main, Criterion};
use nym_compact_ecash::scheme::coin_indices_signatures::{
    aggregate_indices_signatures, sign_coin_indices, verify_coin_indices_signatures,
    CoinIndexSignatureShare, PartialCoinIndexSignature,
};
use nym_compact_ecash::scheme::keygen::SecretKeyAuth;
use nym_compact_ecash::setup::Parameters;
use nym_compact_ecash::{aggregate_verification_keys, ttp_keygen, VerificationKeyAuth};

fn bench_coin_signing(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark-sign-verify-coin-signing");

    let ll = 32;
    let params = Parameters::new(ll);
    let authorities_keypairs = ttp_keygen(2, 3).unwrap();
    let indices: [u64; 3] = [1, 2, 3];

    // Pick one authority to do the signing
    let sk_i_auth = authorities_keypairs[0].secret_key();
    let vk_i_auth = authorities_keypairs[0].verification_key();

    // list of verification keys of each authority
    let verification_keys_auth: Vec<VerificationKeyAuth> = authorities_keypairs
        .iter()
        .map(|keypair| keypair.verification_key())
        .collect();
    // the global master verification key
    let verification_key =
        aggregate_verification_keys(&verification_keys_auth, Some(&indices)).unwrap();

    let partial_signatures = sign_coin_indices(&params, &verification_key, sk_i_auth).unwrap();

    // ISSUING AUTHORITY BENCHMARK: issue a set of (partial) signatures for coin indices
    group.bench_function(
        &format!(
            "[IssuingAuthority] sign_coin_indices_L_{}",
            params.get_total_coins()
        ),
        |b| b.iter(|| sign_coin_indices(&params, &verification_key, sk_i_auth)),
    );

    // CLIENT: verify the correctness of the (partial)) signatures for coin indices
    assert!(
        verify_coin_indices_signatures(&verification_key, &vk_i_auth, &partial_signatures).is_ok()
    );
    group.bench_function(
        &format!(
            "[Client] verify_coin_indices_signatures_L_{}",
            params.get_total_coins()
        ),
        |b| {
            b.iter(|| {
                verify_coin_indices_signatures(&verification_key, &vk_i_auth, &partial_signatures)
            })
        },
    );
}

fn bench_aggregate_coin_indices_signatures(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark-aggregate-coin-signing");

    let ll = 32;
    let params = Parameters::new(ll);
    let authorities_keypairs = ttp_keygen(7, 10).unwrap();
    let indices: [u64; 10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    // list of secret keys of each authority
    let secret_keys_authorities: Vec<&SecretKeyAuth> = authorities_keypairs
        .iter()
        .map(|keypair| keypair.secret_key())
        .collect();
    // list of verification keys of each authority
    let verification_keys_auth: Vec<VerificationKeyAuth> = authorities_keypairs
        .iter()
        .map(|keypair| keypair.verification_key())
        .collect();
    // the global master verification key
    let verification_key =
        aggregate_verification_keys(&verification_keys_auth, Some(&indices)).unwrap();

    // create the partial signatures from each authority
    let partial_signatures: Vec<Vec<PartialCoinIndexSignature>> = secret_keys_authorities
        .iter()
        .map(|sk_auth| sign_coin_indices(&params, &verification_key, sk_auth).unwrap())
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

    // CLIENT: verify all the partial signature vectors and aggregate into a single vector of signed coin indices
    group.bench_function(
        &format!(
            "[Client] aggregate_coin_indices_signatures_from_{}_issuing_authorities_L_{}",
            authorities_keypairs.len(),
            params.get_total_coins(),
        ),
        |b| b.iter(|| aggregate_indices_signatures(&params, &verification_key, &combined_data)),
    );
}

criterion_group!(
    benches,
    bench_coin_signing,
    bench_aggregate_coin_indices_signatures
);
criterion_main!(benches);
