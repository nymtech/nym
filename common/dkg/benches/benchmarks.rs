// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bls12_381::{G1Projective, G2Affine, G2Prepared, Scalar};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ff::Field;
use nym_dkg::bte::encryption::BabyStepGiantStepLookup;
use nym_dkg::bte::proof_chunking::ProofOfChunking;
use nym_dkg::bte::proof_discrete_log::ProofOfDiscreteLog;
use nym_dkg::bte::proof_sharing::ProofOfSecretSharing;
use nym_dkg::bte::{
    decrypt_share, encrypt_shares, keygen, proof_chunking, proof_sharing, setup, DecryptionKey,
    Params, PublicKey,
};
use nym_dkg::interpolation::polynomial::Polynomial;
use nym_dkg::{combine_shares, Dealing, NodeIndex, Share, Threshold};
use rand::CryptoRng;
use rand_core::{RngCore, SeedableRng};
use std::collections::BTreeMap;

pub fn precompute_default_bsgs_table(c: &mut Criterion) {
    c.bench_function("bsgs default table", |b| {
        b.iter(|| black_box(BabyStepGiantStepLookup::default()))
    });
}

pub fn precomputing_g2_generator_for_miller_loop(c: &mut Criterion) {
    let g2 = G2Affine::generator();
    c.bench_function("precomputing G2Prepared", |b| {
        b.iter(|| black_box(G2Prepared::from(g2)))
    });
}

fn prepare_keys(
    mut rng: impl RngCore + CryptoRng,
    nodes: usize,
) -> (BTreeMap<NodeIndex, PublicKey>, Vec<DecryptionKey>) {
    let params = setup();
    let mut node_indices = (0..nodes).map(|_| rng.next_u64()).collect::<Vec<_>>();
    node_indices.sort_unstable();

    let mut receivers = BTreeMap::new();
    let mut dks = Vec::new();
    for index in &node_indices {
        let (dk, pk) = keygen(&params, &mut rng);
        receivers.insert(*index, *pk.public_key());
        dks.push(dk)
    }

    (receivers, dks)
}

fn prepare_resharing(
    mut rng: impl RngCore + CryptoRng,
    params: &Params,
    nodes: usize,
    threshold: Threshold,
) -> (BTreeMap<NodeIndex, PublicKey>, Vec<Scalar>) {
    let (receivers, mut dks) = prepare_keys(&mut rng, nodes);

    let first_dealings = receivers
        .keys()
        .map(|&dealer_index| {
            Dealing::create(&mut rng, params, dealer_index, threshold, &receivers, None).0
        })
        .collect::<Vec<_>>();

    let mut derived_secrets = Vec::new();
    for (i, ref mut dk) in dks.iter_mut().enumerate() {
        let shares = first_dealings
            .iter()
            .map(|dealing| decrypt_share(params, dk, i, &dealing.ciphertexts, None).unwrap())
            .collect();

        let recovered_secret =
            combine_shares(shares, &receivers.keys().copied().collect::<Vec<_>>()).unwrap();

        derived_secrets.push(recovered_secret)
    }

    (receivers, derived_secrets)
}

pub fn creating_dealing_for_3_parties(c: &mut Criterion) {
    let dummy_seed = [42u8; 32];
    let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
    let params = setup();
    let threshold = 2;

    let (receivers, _) = prepare_keys(&mut rng, 3);

    c.bench_function("creating single dealing for 3 parties (threshold 2)", |b| {
        b.iter(|| {
            black_box({
                Dealing::create(
                    &mut rng,
                    &params,
                    receivers.keys().next().copied().unwrap(),
                    threshold,
                    &receivers,
                    None,
                )
            })
        })
    });
}

pub fn creating_reshared_dealing_for_3_parties(c: &mut Criterion) {
    let dummy_seed = [42u8; 32];
    let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
    let params = setup();
    let threshold = 2;

    let (receivers, mut derived_secrets) = prepare_resharing(&mut rng, &params, 3, threshold);

    c.bench_function(
        "creating single re-shared dealing for 3 parties (threshold 2)",
        |b| {
            b.iter(|| {
                black_box({
                    Dealing::create(
                        &mut rng,
                        &params,
                        receivers.keys().next().copied().unwrap(),
                        threshold,
                        &receivers,
                        Some(derived_secrets.pop().unwrap()),
                    )
                })
            })
        },
    );
}

pub fn verifying_dealing_made_for_3_parties_and_recovering_share(c: &mut Criterion) {
    let dummy_seed = [42u8; 32];
    let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
    let params = setup();
    let threshold = 2;

    let (receivers, mut dks) = prepare_keys(&mut rng, 3);
    let (dealing, _) = Dealing::create(
        &mut rng,
        &params,
        receivers.keys().next().copied().unwrap(),
        threshold,
        &receivers,
        None,
    );

    let first_key = dks.get_mut(0).unwrap();

    c.bench_function(
        "verifying single dealing made for 3 parties (threshold 2) and recovering share",
        |b| {
            b.iter(|| {
                assert!(dealing.verify(&params, threshold, &receivers, None).is_ok());
                black_box(
                    decrypt_share(&params, first_key, 0, &dealing.ciphertexts, None).unwrap(),
                );
            })
        },
    );
}

pub fn creating_dealing_for_20_parties(c: &mut Criterion) {
    let dummy_seed = [42u8; 32];
    let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
    let params = setup();
    let threshold = 14;

    let (receivers, _) = prepare_keys(&mut rng, 20);

    c.bench_function(
        "creating single dealing for 20 parties (threshold 14)",
        |b| {
            b.iter(|| {
                black_box({
                    Dealing::create(
                        &mut rng,
                        &params,
                        receivers.keys().next().copied().unwrap(),
                        threshold,
                        &receivers,
                        None,
                    )
                })
            })
        },
    );
}

pub fn creating_reshared_dealing_for_20_parties(c: &mut Criterion) {
    let dummy_seed = [42u8; 32];
    let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
    let params = setup();
    let threshold = 14;

    let (receivers, mut derived_secrets) = prepare_resharing(&mut rng, &params, 20, threshold);

    c.bench_function(
        "creating single re-shared dealing for 20 parties (threshold 14)",
        |b| {
            b.iter(|| {
                black_box({
                    Dealing::create(
                        &mut rng,
                        &params,
                        receivers.keys().next().copied().unwrap(),
                        threshold,
                        &receivers,
                        Some(derived_secrets.pop().unwrap()),
                    )
                })
            })
        },
    );
}

pub fn verifying_dealing_made_for_20_parties_and_recovering_share(c: &mut Criterion) {
    let dummy_seed = [42u8; 32];
    let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
    let params = setup();
    let threshold = 14;

    let (receivers, mut dks) = prepare_keys(&mut rng, 20);
    let (dealing, _) = Dealing::create(
        &mut rng,
        &params,
        receivers.keys().next().copied().unwrap(),
        threshold,
        &receivers,
        None,
    );

    let first_key = dks.get_mut(0).unwrap();

    c.bench_function(
        "verifying single dealing made for 20 parties (threshold 14) and recovering share",
        |b| {
            b.iter(|| {
                assert!(dealing.verify(&params, threshold, &receivers, None).is_ok());
                black_box(
                    decrypt_share(&params, first_key, 0, &dealing.ciphertexts, None).unwrap(),
                );
            })
        },
    );
}

pub fn creating_dealing_for_100_parties(c: &mut Criterion) {
    let dummy_seed = [42u8; 32];
    let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
    let params = setup();
    let threshold = 67;

    let (receivers, _) = prepare_keys(&mut rng, 100);

    c.bench_function(
        "creating single dealing for 100 parties (threshold 67)",
        |b| {
            b.iter(|| {
                black_box({
                    Dealing::create(
                        &mut rng,
                        &params,
                        receivers.keys().next().copied().unwrap(),
                        threshold,
                        &receivers,
                        None,
                    )
                })
            })
        },
    );
}

pub fn creating_reshared_dealing_for_100_parties(c: &mut Criterion) {
    let dummy_seed = [42u8; 32];
    let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
    let params = setup();
    let threshold = 67;

    let (receivers, mut derived_secrets) = prepare_resharing(&mut rng, &params, 100, threshold);

    c.bench_function(
        "creating single re-shared dealing for 100 parties (threshold 67)",
        |b| {
            b.iter(|| {
                black_box({
                    Dealing::create(
                        &mut rng,
                        &params,
                        receivers.keys().next().copied().unwrap(),
                        threshold,
                        &receivers,
                        Some(derived_secrets.pop().unwrap()),
                    )
                })
            })
        },
    );
}

pub fn verifying_dealing_made_for_100_parties_and_recovering_share(c: &mut Criterion) {
    let dummy_seed = [42u8; 32];
    let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
    let params = setup();
    let threshold = 67;

    let (receivers, mut dks) = prepare_keys(&mut rng, 100);
    let (dealing, _) = Dealing::create(
        &mut rng,
        &params,
        receivers.keys().next().copied().unwrap(),
        threshold,
        &receivers,
        None,
    );

    let first_key = dks.get_mut(0).unwrap();

    c.bench_function(
        "verifying single dealing made for 100 parties (threshold 67) and recovering share",
        |b| {
            b.iter(|| {
                assert!(dealing.verify(&params, threshold, &receivers, None).is_ok());
                black_box(
                    decrypt_share(&params, first_key, 0, &dealing.ciphertexts, None).unwrap(),
                );
            })
        },
    );
}

pub fn creating_proof_of_key_possession(c: &mut Criterion) {
    let dummy_seed = [42u8; 32];
    let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

    let g1 = G1Projective::generator();
    let x = Scalar::random(&mut rng);
    let y = g1 * x;

    c.bench_function("creating proof of key possession", |b| {
        b.iter(|| black_box(ProofOfDiscreteLog::construct(&mut rng, &y, &x)))
    });
}

pub fn verifying_proof_of_key_possession(c: &mut Criterion) {
    let dummy_seed = [42u8; 32];
    let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

    let g1 = G1Projective::generator();
    let x = Scalar::random(&mut rng);
    let y = g1 * x;

    let zk_proof = ProofOfDiscreteLog::construct(&mut rng, &y, &x);
    c.bench_function("verifying proof of key possession", |b| {
        b.iter(|| black_box(zk_proof.verify(&y)))
    });
}

pub fn creating_proof_of_chunking_for_100_parties(c: &mut Criterion) {
    let dummy_seed = [42u8; 32];
    let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
    let params = setup();

    let (receivers, _) = prepare_keys(&mut rng, 100);

    let polynomial = Polynomial::new_random(&mut rng, 67);
    let shares = receivers
        .keys()
        .map(|&node_index| polynomial.evaluate_at(&Scalar::from(node_index)).into())
        .collect::<Vec<_>>();

    let remote_share_key_pairs = shares.iter().zip(receivers.values()).collect::<Vec<_>>();
    let ordered_public_keys = receivers.values().copied().collect::<Vec<_>>();

    let (ciphertexts, hazmat) = encrypt_shares(&remote_share_key_pairs, &params, &mut rng);

    c.bench_function("creating proof of chunking for 100 parties", |b| {
        b.iter(|| {
            let chunking_instance =
                proof_chunking::Instance::new(&ordered_public_keys, &ciphertexts);
            black_box(
                ProofOfChunking::construct(&mut rng, chunking_instance, hazmat.r(), &shares)
                    .expect("failed to construct proof of chunking"),
            )
        })
    });
}

pub fn verifying_proof_of_chunking_for_100_parties(c: &mut Criterion) {
    let dummy_seed = [42u8; 32];
    let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
    let params = setup();

    let (receivers, _) = prepare_keys(&mut rng, 100);

    let polynomial = Polynomial::new_random(&mut rng, 67);
    let shares = receivers
        .keys()
        .map(|&node_index| polynomial.evaluate_at(&Scalar::from(node_index)).into())
        .collect::<Vec<_>>();

    let remote_share_key_pairs = shares.iter().zip(receivers.values()).collect::<Vec<_>>();
    let ordered_public_keys = receivers.values().copied().collect::<Vec<_>>();

    let (ciphertexts, hazmat) = encrypt_shares(&remote_share_key_pairs, &params, &mut rng);

    let chunking_instance = proof_chunking::Instance::new(&ordered_public_keys, &ciphertexts);
    let proof_of_chunking =
        ProofOfChunking::construct(&mut rng, chunking_instance, hazmat.r(), &shares)
            .expect("failed to construct proof of chunking");

    c.bench_function("verifying proof of chunking for 100 parties", |b| {
        b.iter(|| {
            let chunking_instance =
                proof_chunking::Instance::new(&ordered_public_keys, &ciphertexts);
            black_box(proof_of_chunking.verify(chunking_instance))
        })
    });
}

pub fn creating_proof_of_secret_sharing_for_100_parties(c: &mut Criterion) {
    let dummy_seed = [42u8; 32];
    let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
    let params = setup();

    let (receivers, _) = prepare_keys(&mut rng, 100);

    let polynomial = Polynomial::new_random(&mut rng, 67);
    let shares = receivers
        .keys()
        .map(|&node_index| polynomial.evaluate_at(&Scalar::from(node_index)).into())
        .collect::<Vec<_>>();

    let remote_share_key_pairs = shares.iter().zip(receivers.values()).collect::<Vec<_>>();

    let (ciphertexts, hazmat) = encrypt_shares(&remote_share_key_pairs, &params, &mut rng);

    let combined_ciphertexts = ciphertexts.combine_ciphertexts();
    let combined_r = hazmat.combine_rs();
    let combined_rr = ciphertexts.combine_rs();
    let public_coefficients = polynomial.public_coefficients();

    c.bench_function("creating proof of secret sharing for 100 parties", |b| {
        b.iter(|| {
            let sharing_instance = proof_sharing::Instance::new(
                &receivers,
                &public_coefficients,
                &combined_rr,
                &combined_ciphertexts,
            );
            black_box(
                ProofOfSecretSharing::construct(&mut rng, sharing_instance, &combined_r, &shares)
                    .expect("failed to construct proof of secret sharing"),
            )
        })
    });
}

pub fn verifying_proof_of_secret_sharing_for_100_parties(c: &mut Criterion) {
    let dummy_seed = [42u8; 32];
    let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
    let params = setup();

    let (receivers, _) = prepare_keys(&mut rng, 100);

    let polynomial = Polynomial::new_random(&mut rng, 67);
    let shares = receivers
        .keys()
        .map(|&node_index| polynomial.evaluate_at(&Scalar::from(node_index)).into())
        .collect::<Vec<_>>();

    let remote_share_key_pairs = shares.iter().zip(receivers.values()).collect::<Vec<_>>();

    let (ciphertexts, hazmat) = encrypt_shares(&remote_share_key_pairs, &params, &mut rng);

    let combined_ciphertexts = ciphertexts.combine_ciphertexts();
    let combined_r = hazmat.combine_rs();
    let combined_rr = ciphertexts.combine_rs();
    let public_coefficients = polynomial.public_coefficients();
    let sharing_instance = proof_sharing::Instance::new(
        &receivers,
        &public_coefficients,
        &combined_rr,
        &combined_ciphertexts,
    );
    let proof_of_secret_sharing =
        ProofOfSecretSharing::construct(&mut rng, sharing_instance, &combined_r, &shares)
            .expect("failed to construct proof of secret sharing");

    c.bench_function("verifying proof of secret sharing for 100 parties", |b| {
        b.iter(|| {
            let sharing_instance = proof_sharing::Instance::new(
                &receivers,
                &public_coefficients,
                &combined_rr,
                &combined_ciphertexts,
            );
            black_box(proof_of_secret_sharing.verify(sharing_instance))
        })
    });
}

pub fn single_share_encryption(c: &mut Criterion) {
    let dummy_seed = [42u8; 32];
    let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
    let params = setup();
    let (_, pk) = keygen(&params, &mut rng);

    let polynomial = Polynomial::new_random(&mut rng, 3);
    let share: Share = polynomial.evaluate_at(&Scalar::from(42)).into();

    c.bench_function("single share encryption", |b| {
        b.iter(|| {
            black_box(encrypt_shares(
                &[(&share, pk.public_key())],
                &params,
                &mut rng,
            ))
        })
    });
}

pub fn share_encryption_100(c: &mut Criterion) {
    let dummy_seed = [42u8; 32];
    let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
    let params = setup();

    let (receivers, _) = prepare_keys(&mut rng, 100);
    let polynomial = Polynomial::new_random(&mut rng, 3);
    let shares = receivers
        .keys()
        .map(|&node_index| polynomial.evaluate_at(&Scalar::from(node_index)).into())
        .collect::<Vec<_>>();

    let remote_share_key_pairs = shares.iter().zip(receivers.values()).collect::<Vec<_>>();

    c.bench_function("100 shares encryption", |b| {
        b.iter(|| black_box(encrypt_shares(&remote_share_key_pairs, &params, &mut rng)))
    });
}

pub fn share_decryption(c: &mut Criterion) {
    let dummy_seed = [42u8; 32];
    let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
    let params = setup();
    let (dk, pk) = keygen(&params, &mut rng);

    let polynomial = Polynomial::new_random(&mut rng, 3);
    let share: Share = polynomial.evaluate_at(&Scalar::from(42)).into();
    let (ciphertexts, _) = encrypt_shares(&[(&share, pk.public_key())], &params, &mut rng);

    c.bench_function("single share decryption", |b| {
        b.iter(|| black_box(decrypt_share(&params, &dk, 0, &ciphertexts, None)))
    });
}

criterion_group!(
    utils,
    precompute_default_bsgs_table,
    precomputing_g2_generator_for_miller_loop,
);

criterion_group!(
    dealings_creation,
    creating_dealing_for_3_parties,
    creating_dealing_for_20_parties,
    creating_dealing_for_100_parties,
);

criterion_group!(
    reshared_dealings_creation,
    creating_reshared_dealing_for_3_parties,
    creating_reshared_dealing_for_20_parties,
    creating_reshared_dealing_for_100_parties,
);

// note: in our setting each party will have to create at least 4 dealings (one per attribute in credential)
// and verify 99 * 4 of them (4 from each other dealer)
criterion_group!(
    dealings_verification,
    verifying_dealing_made_for_3_parties_and_recovering_share,
    verifying_dealing_made_for_20_parties_and_recovering_share,
    verifying_dealing_made_for_100_parties_and_recovering_share,
);

criterion_group!(
    proofs_of_knowledge,
    creating_proof_of_key_possession,
    verifying_proof_of_key_possession,
    creating_proof_of_chunking_for_100_parties,
    verifying_proof_of_chunking_for_100_parties,
    creating_proof_of_secret_sharing_for_100_parties,
    verifying_proof_of_secret_sharing_for_100_parties
);

criterion_group!(
    encryption,
    single_share_encryption,
    share_encryption_100,
    share_decryption,
);

criterion_main!(
    utils,
    dealings_creation,
    reshared_dealings_creation,
    dealings_verification,
    proofs_of_knowledge,
    encryption
);

// TODO: benchmark using affine vs projective representation throughout the crate
// (when conversion / serialization / computation is involved)
