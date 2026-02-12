// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// fine in benchmarking code
#![allow(clippy::expect_used)]
#![allow(clippy::unwrap_used)]

use criterion::{Criterion, criterion_group, criterion_main};

use nym_kkt::{
    ciphersuite::{Ciphersuite, EncapsulationKey, HashFunction, KEM, SignatureScheme},
    context::KKTMode,
    frame::KKTFrame,
    key_utils::{
        generate_keypair_libcrux, generate_keypair_mceliece, generate_keypair_mlkem,
        hash_encapsulation_key,
    },
    session::{
        initiator_ingest_response, initiator_process, responder_ingest_message, responder_process,
    },
};
use rand09::prelude::*;

pub fn gen_mlkem768_keypair(c: &mut Criterion) {
    c.bench_function("Generate MlKem768 Keypair", |b| {
        b.iter(|| {
            libcrux_kem::key_gen(libcrux_kem::Algorithm::MlKem768, &mut rand09::rng()).unwrap()
        });
    });
}

pub fn kkt_benchmark(c: &mut Criterion) {
    let mut rng = rand09::rng();

    let mut secret_initiator: [u8; 32] = [0u8; 32];
    rng.fill_bytes(&mut secret_initiator);

    let mut secret_responder: [u8; 32] = [0u8; 32];
    rng.fill_bytes(&mut secret_responder);

    for kem in [KEM::MlKem768, KEM::XWing, KEM::X25519, KEM::McEliece] {
        for hash_function in [
            HashFunction::Blake3,
            HashFunction::SHA256,
            HashFunction::Shake128,
            HashFunction::Shake256,
        ] {
            let ciphersuite = Ciphersuite::resolve_ciphersuite(
                kem,
                hash_function,
                SignatureScheme::Ed25519,
                None,
            )
            .unwrap();

            // generate kem public keys

            let (responder_kem_public_key, initiator_kem_public_key) = match kem {
                KEM::MlKem768 => (
                    EncapsulationKey::MlKem768(generate_keypair_mlkem(&mut rng).1),
                    EncapsulationKey::MlKem768(generate_keypair_mlkem(&mut rng).1),
                ),
                KEM::XWing => (
                    EncapsulationKey::XWing(generate_keypair_libcrux(&mut rng, kem).unwrap().1),
                    EncapsulationKey::XWing(generate_keypair_libcrux(&mut rng, kem).unwrap().1),
                ),
                KEM::X25519 => (
                    EncapsulationKey::X25519(generate_keypair_libcrux(&mut rng, kem).unwrap().1),
                    EncapsulationKey::X25519(generate_keypair_libcrux(&mut rng, kem).unwrap().1),
                ),
                KEM::McEliece => (
                    EncapsulationKey::McEliece(generate_keypair_mceliece(&mut rng).1),
                    EncapsulationKey::McEliece(generate_keypair_mceliece(&mut rng).1),
                ),
            };

            let i_kem_key_bytes = initiator_kem_public_key.encode();

            let r_kem_key_bytes = responder_kem_public_key.encode();

            let i_dir_hash = hash_encapsulation_key(
                &ciphersuite.hash_function(),
                ciphersuite.hash_len(),
                &i_kem_key_bytes,
            );

            let r_dir_hash = hash_encapsulation_key(
                &ciphersuite.hash_function(),
                ciphersuite.hash_len(),
                &r_kem_key_bytes,
            );

            // Anonymous Initiator, OneWay
            {
                c.bench_function(
                    &format!("{kem}, {hash_function} | Anonymous Initiator: Generate Request",),
                    |b| {
                        b.iter(|| {
                            initiator_process(&mut rng, KKTMode::OneWay, ciphersuite, None).unwrap()
                        });
                    },
                );

                let (mut i_context, i_frame) =
                    initiator_process(&mut rng, KKTMode::OneWay, ciphersuite, None).unwrap();

                c.bench_function(
                    &format!(
                        "{kem}, {hash_function} | Anonymous Initiator: Encode Frame - Request",
                    ),
                    |b| b.iter(|| i_frame.to_bytes()),
                );

                let i_frame_bytes = i_frame.to_bytes();

                c.bench_function(
                    &format!(
                        "{kem}, {hash_function} | Anonymous Initiator: Decode Frame - Request",
                    ),
                    |b| b.iter(|| KKTFrame::from_bytes(&i_frame_bytes).unwrap()),
                );

                let (i_frame_r, r_context) = KKTFrame::from_bytes(&i_frame_bytes).unwrap();

                c.bench_function(
                    &format!(
                        "{kem}, {hash_function} | Anonymous Initiator: Responder Ingest Frame",
                    ),
                    |b| {
                        b.iter(|| responder_ingest_message(&r_context, None, &i_frame_r).unwrap());
                    },
                );

                let (mut r_context, _) =
                    responder_ingest_message(&r_context, None, &i_frame_r).unwrap();

                c.bench_function(
                    &format!(
                        "{kem}, {hash_function} | Anonymous Initiator: Responder Generate Response",
                    ),
                    |b| {
                        b.iter(|| {
                            responder_process(
                                &mut r_context,
                                i_frame_r.session_id(),
                                &responder_kem_public_key,
                            )
                            .unwrap()
                        });
                    },
                );
                let r_frame = responder_process(
                    &mut r_context,
                    i_frame_r.session_id(),
                    &responder_kem_public_key,
                )
                .unwrap();

                c.bench_function(
                    &format!(
                        "{kem}, {hash_function} | Anonymous Initiator: Responder Encode Frame",
                    ),
                    |b| b.iter(|| r_frame.to_bytes()),
                );

                c.bench_function(
                    &format!(
                        "{kem}, {hash_function} | Anonymous Initiator: Initiator Ingest Response",
                    ),
                    |b| {
                        b.iter(|| {
                            initiator_ingest_response(
                                &mut i_context,
                                &r_frame,
                                &r_frame.context().unwrap(),
                                &r_dir_hash,
                            )
                            .unwrap()
                        });
                    },
                );

                let obtained_key = initiator_ingest_response(
                    &mut i_context,
                    &r_frame,
                    &r_frame.context().unwrap(),
                    &r_dir_hash,
                )
                .unwrap();

                assert_eq!(obtained_key.encode(), r_kem_key_bytes)
            }
            // Initiator, OneWay
            {
                let (mut i_context, i_frame) =
                    initiator_process(&mut rng, KKTMode::OneWay, ciphersuite, None).unwrap();

                c.bench_function(
                    &format!("{kem}, {hash_function} | Initiator OneWay: Generate Request",),
                    |b| {
                        b.iter(|| {
                            initiator_process(&mut rng, KKTMode::OneWay, ciphersuite, None).unwrap()
                        });
                    },
                );

                c.bench_function(
                    &format!("{kem}, {hash_function} | Initiator OneWay: Encode Frame - Request",),
                    |b| b.iter(|| i_frame.to_bytes()),
                );

                let i_frame_bytes = i_frame.to_bytes();

                c.bench_function(
                    &format!("{kem}, {hash_function} | Initiator OneWay: Decode Frame - Request",),
                    |b| b.iter(|| KKTFrame::from_bytes(&i_frame_bytes).unwrap()),
                );

                let (i_frame_r, r_context) = KKTFrame::from_bytes(&i_frame_bytes).unwrap();

                c.bench_function(
                    &format!("{kem}, {hash_function} | Initiator OneWay: Responder Ingest Frame",),
                    |b| {
                        b.iter(|| responder_ingest_message(&r_context, None, &i_frame_r).unwrap());
                    },
                );

                let (mut r_context, r_obtained_key) =
                    responder_ingest_message(&r_context, None, &i_frame_r).unwrap();

                assert!(r_obtained_key.is_none());

                c.bench_function(
                    &format!(
                        "{kem}, {hash_function} | Initiator OneWay: Responder Generate Response",
                    ),
                    |b| {
                        b.iter(|| {
                            responder_process(
                                &mut r_context,
                                i_frame_r.session_id(),
                                &responder_kem_public_key,
                            )
                            .unwrap()
                        });
                    },
                );

                let r_frame = responder_process(
                    &mut r_context,
                    i_frame_r.session_id(),
                    &responder_kem_public_key,
                )
                .unwrap();

                c.bench_function(
                    &format!("{kem}, {hash_function} | Initiator OneWay: Responder Encode Frame",),
                    |b| {
                        b.iter(|| r_frame.to_bytes());
                    },
                );

                c.bench_function(
                    &format!(
                        "{kem}, {hash_function} | Initiator OneWay: Initiator Ingest Response",
                    ),
                    |b| {
                        b.iter(|| {
                            initiator_ingest_response(
                                &mut i_context,
                                &r_frame,
                                &r_frame.context().unwrap(),
                                &r_dir_hash,
                            )
                            .unwrap()
                        });
                    },
                );

                let i_obtained_key = initiator_ingest_response(
                    &mut i_context,
                    &r_frame,
                    &r_frame.context().unwrap(),
                    &r_dir_hash,
                )
                .unwrap();

                assert_eq!(i_obtained_key.encode(), r_kem_key_bytes)
            }

            // Initiator, Mutual
            {
                c.bench_function(
                    &format!("{kem}, {hash_function} | Initiator Mutual: Generate Request",),
                    |b| {
                        b.iter(|| {
                            initiator_process(
                                &mut rng,
                                KKTMode::Mutual,
                                ciphersuite,
                                Some(&initiator_kem_public_key),
                            )
                            .unwrap()
                        });
                    },
                );

                let (mut i_context, i_frame) = initiator_process(
                    &mut rng,
                    KKTMode::Mutual,
                    ciphersuite,
                    Some(&initiator_kem_public_key),
                )
                .unwrap();

                c.bench_function(
                    &format!("{kem}, {hash_function} | Initiator Mutual: Encode Frame - Request",),
                    |b| {
                        b.iter(|| i_frame.to_bytes());
                    },
                );

                let i_frame_bytes = i_frame.to_bytes();

                c.bench_function(
                    &format!("{kem}, {hash_function} | Initiator Mutual: Decode Frame - Request",),
                    |b| {
                        b.iter(|| KKTFrame::from_bytes(&i_frame_bytes).unwrap());
                    },
                );

                let (i_frame_r, r_context) = KKTFrame::from_bytes(&i_frame_bytes).unwrap();

                c.bench_function(
                    &format!("{kem}, {hash_function} | Initiator Mutual: Responder Ingest Frame",),
                    |b| {
                        b.iter(|| {
                            responder_ingest_message(&r_context, Some(&i_dir_hash), &i_frame_r)
                                .unwrap()
                        });
                    },
                );

                let (mut r_context, r_obtained_key) =
                    responder_ingest_message(&r_context, Some(&i_dir_hash), &i_frame_r).unwrap();

                assert_eq!(r_obtained_key.unwrap().encode(), i_kem_key_bytes);

                c.bench_function(
                    &format!(
                        "{kem}, {hash_function} | Initiator Mutual: Responder Generate Response",
                    ),
                    |b| {
                        b.iter(|| {
                            responder_process(
                                &mut r_context,
                                i_frame_r.session_id(),
                                &responder_kem_public_key,
                            )
                            .unwrap()
                        });
                    },
                );

                let r_frame = responder_process(
                    &mut r_context,
                    i_frame_r.session_id(),
                    &responder_kem_public_key,
                )
                .unwrap();

                c.bench_function(
                    &format!("{kem}, {hash_function} | Initiator Mutual: Responder Encode Frame",),
                    |b| {
                        b.iter(|| {
                            r_frame.to_bytes();
                        });
                    },
                );

                c.bench_function(
                    &format!(
                        "{kem}, {hash_function} | Initiator Mutual: Initiator Ingest Response",
                    ),
                    |b| {
                        b.iter(|| {
                            initiator_ingest_response(
                                &mut i_context,
                                &r_frame,
                                &r_frame.context().unwrap(),
                                &r_dir_hash,
                            )
                            .unwrap()
                        });
                    },
                );

                let obtained_key = initiator_ingest_response(
                    &mut i_context,
                    &r_frame,
                    &r_frame.context().unwrap(),
                    &r_dir_hash,
                )
                .unwrap();

                assert_eq!(obtained_key.encode(), r_kem_key_bytes)
            }
        }
    }
}

criterion_group!(benches, gen_mlkem768_keypair, kkt_benchmark);
criterion_main!(benches);
