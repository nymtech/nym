// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_compact_ecash::scheme::expiration_date_signatures::{
    aggregate_expiration_signatures, sign_expiration_date, verify_valid_dates_signatures,
    ExpirationDateSignatureShare, PartialExpirationDateSignature,
};

use criterion::{criterion_group, criterion_main, Criterion};
use nym_compact_ecash::constants;
use nym_compact_ecash::scheme::keygen::SecretKeyAuth;
use nym_compact_ecash::{aggregate_verification_keys, ttp_keygen, VerificationKeyAuth};

fn bench_partial_sign_expiration_date(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark-sign-verify-expiration-date");
    let expiration_date = 1703183958;

    let authorities_keys = ttp_keygen(2, 3).unwrap();
    let sk_i_auth = authorities_keys[0].secret_key();
    let vk_i_auth = authorities_keys[0].verification_key();
    let partial_exp_sig = sign_expiration_date(sk_i_auth, expiration_date).unwrap();

    // ISSUING AUTHORITY BENCHMARK: issue a set of (partial) signatures for a given expiration date
    group.bench_function(
        &format!(
            "[IssuingAuthority] sign_expiration_date_{}_validity_period",
            constants::CRED_VALIDITY_PERIOD,
        ),
        |b| b.iter(|| sign_expiration_date(sk_i_auth, expiration_date)),
    );

    // CLIENT: verify the correctness of the set of (partial) signatures for a given expiration date
    assert!(verify_valid_dates_signatures(&vk_i_auth, &partial_exp_sig, expiration_date).is_ok());
    group.bench_function(
        &format!(
            "[Client] verify_valid_dates_signatures_{}_validity_period",
            constants::CRED_VALIDITY_PERIOD,
        ),
        |b| b.iter(|| verify_valid_dates_signatures(&vk_i_auth, &partial_exp_sig, expiration_date)),
    );
}

fn bench_aggregate_expiration_date_signatures(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark-aggregate-verify-expiration-date-signatures");
    let expiration_date = 1703183958;

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

    let mut partial_signatures: Vec<Vec<PartialExpirationDateSignature>> =
        Vec::with_capacity(constants::CRED_VALIDITY_PERIOD as usize);
    for sk in secret_keys_authorities.iter() {
        let sign = sign_expiration_date(sk, expiration_date).unwrap();
        partial_signatures.push(sign);
    }

    let combined_data: Vec<_> = indices
        .iter()
        .zip(verification_keys_auth.iter().zip(partial_signatures.iter()))
        .map(|(i, (vk, sigs))| ExpirationDateSignatureShare {
            index: *i,
            key: vk.clone(),
            signatures: sigs.to_vec(),
        })
        .collect();

    // CLIENT: verify all the partial signature vectors and aggregate into a single vector of signed valid dates
    group.bench_function(
        &format!(
            "[Client] aggregate_expiration_signatures_from_{}_issuing_authorities_{}_validity_period",
            constants::CRED_VALIDITY_PERIOD, authorities_keypairs.len(),
        ),
        |b| {
            b.iter(|| {
                aggregate_expiration_signatures(
                    &verification_key,
                    expiration_date,
                    &combined_data,
                )
            })
        },
    );
}

criterion_group!(
    benches,
    bench_partial_sign_expiration_date,
    bench_aggregate_expiration_date_signatures
);
criterion_main!(benches);
