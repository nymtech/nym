// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use criterion::{criterion_group, criterion_main, Criterion};
use libcrux_psq::impls::MlKem768;
use nym_crypto::asymmetric::ed25519;
use nym_psq_kkt::kkt::{
    KKTInitiator, KKTResponder, KKT_REQ_LEN, KKT_RES_LEN_MLKEM768, KKT_TAG_LEN,
};
use rand::prelude::*;

pub fn gen_ed25519_keypair(c: &mut Criterion) {
    c.bench_function("Generate Ed25519 Keypair", |b| {
        b.iter(|| {
            let mut s: [u8; 32] = [0u8; 32];
            rand::rng().fill_bytes(&mut s);
            ed25519::KeyPair::from_secret(s, 0)
        });
    });
}

pub fn gen_mlkem768_keypair(c: &mut Criterion) {
    c.bench_function("Generate MlKem768 Keypair", |b| {
        b.iter(|| {
            libcrux_kem::key_gen(libcrux_kem::Algorithm::MlKem768, &mut rand::rng()).unwrap()
        });
    });
}

pub fn kkt_benchmark(c: &mut Criterion) {
    // generate ed25519 keys
    let mut secret_initiator: [u8; 32] = [0u8; 32];
    rand::rng().fill_bytes(&mut secret_initiator);
    let initiator_ed25519_keypair = ed25519::KeyPair::from_secret(secret_initiator, 0);

    let mut secret_responder: [u8; 32] = [0u8; 32];
    rand::rng().fill_bytes(&mut secret_responder);
    let responder_ed25519_keypair = ed25519::KeyPair::from_secret(secret_responder, 1);

    // generate kem keypair
    let (_, responder_kem_public_key) =
        libcrux_kem::key_gen(libcrux_kem::Algorithm::MlKem768, &mut rand::rng()).unwrap();

    // initialize parties
    let initiator: KKTInitiator<MlKem768> =
        KKTInitiator::init(initiator_ed25519_keypair.private_key());

    c.bench_function("Initialize Initiator", |b| {
        b.iter(|| KKTInitiator::<MlKem768>::init(initiator_ed25519_keypair.private_key()));
    });

    let responder: KKTResponder<MlKem768> = KKTResponder::init(
        responder_ed25519_keypair.private_key(),
        &responder_kem_public_key,
    );

    c.bench_function("Initialize Responder", |b| {
        b.iter(|| {
            KKTResponder::init(
                responder_ed25519_keypair.private_key(),
                &responder_kem_public_key,
            )
        });
    });

    // create buffers
    let mut request_buffer: [u8; KKT_REQ_LEN] = [0u8; KKT_REQ_LEN];
    let mut response_buffer: [u8; KKT_RES_LEN_MLKEM768] = [0u8; KKT_RES_LEN_MLKEM768];
    let mut tag_buffer: [u8; KKT_TAG_LEN] = [0u8; KKT_TAG_LEN];

    c.bench_function("Initiator: Generate Request", |b| {
        b.iter(|| {
            initiator.request_kem_pk(&mut request_buffer, &mut tag_buffer);
        });
    });

    // generate request
    initiator.request_kem_pk(&mut request_buffer, &mut tag_buffer);

    c.bench_function("Responder: Ingest Request and Generate Response", |b| {
        b.iter(|| {
            responder
                .respond_kem_pk(
                    &mut response_buffer,
                    initiator_ed25519_keypair.public_key(),
                    &request_buffer,
                )
                .unwrap();
        });
    });

    // ingest request, generate response
    responder
        .respond_kem_pk(
            &mut response_buffer,
            initiator_ed25519_keypair.public_key(),
            &request_buffer,
        )
        .unwrap();

    c.bench_function("Initiator: Ingest Response and Store Key", |b| {
        b.iter(|| {
            let _ = initiator
                .ingest_response_kem_pk::<MlKem768>(
                    &response_buffer,
                    &tag_buffer,
                    responder_ed25519_keypair.public_key(),
                )
                .unwrap();
        });
    });

    // ingest response
    let received_responder_key = initiator
        .ingest_response_kem_pk::<MlKem768>(
            &response_buffer,
            &tag_buffer,
            responder_ed25519_keypair.public_key(),
        )
        .unwrap();

    // check if the public key received is the same one that we generated at the start
    assert_eq!(
        responder_kem_public_key.encode(),
        received_responder_key.encode()
    );
}

criterion_group!(
    benches,
    gen_ed25519_keypair,
    gen_mlkem768_keypair,
    kkt_benchmark
);
criterion_main!(benches);
