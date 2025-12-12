// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod ciphersuite;
pub mod context;
// pub mod encryption;
pub mod error;
pub mod frame;
pub mod key_utils;
pub mod kkt;
pub mod session;

// pub mod psq;

// This must be less than 4 bits
pub const KKT_VERSION: u8 = 1;
const _: () = assert!(KKT_VERSION < 1 << 4);

#[cfg(test)]
mod test {
    use nym_crypto::asymmetric::ed25519;
    use rand::prelude::*;

    use crate::{
        ciphersuite::{Ciphersuite, EncapsulationKey, HashFunction, KEM},
        frame::KKTFrame,
        key_utils::{generate_keypair_libcrux, generate_keypair_mceliece, hash_encapsulation_key},
        session::{
            anonymous_initiator_process, initiator_ingest_response, initiator_process,
            responder_ingest_message, responder_process,
        },
    };

    #[test]
    fn test_kkt_psq_e2e_clear() {
        let mut rng = rand::rng();

        // generate ed25519 keys
        let mut secret_initiator: [u8; 32] = [0u8; 32];
        rng.fill_bytes(&mut secret_initiator);
        let initiator_ed25519_keypair = ed25519::KeyPair::from_secret(secret_initiator, 0);

        let mut secret_responder: [u8; 32] = [0u8; 32];
        rng.fill_bytes(&mut secret_responder);
        let responder_ed25519_keypair = ed25519::KeyPair::from_secret(secret_responder, 1);
        for kem in [KEM::MlKem768, KEM::XWing, KEM::X25519, KEM::McEliece] {
            for hash_function in [
                HashFunction::Blake3,
                HashFunction::SHA256,
                HashFunction::SHAKE128,
                HashFunction::SHAKE256,
            ] {
                let ciphersuite = Ciphersuite::resolve_ciphersuite(
                    kem,
                    hash_function,
                    crate::ciphersuite::SignatureScheme::Ed25519,
                    None,
                )
                .unwrap();

                // generate kem public keys

                let (responder_kem_public_key, initiator_kem_public_key) = match kem {
                    KEM::MlKem768 => (
                        EncapsulationKey::MlKem768(
                            generate_keypair_libcrux(&mut rng, kem).unwrap().1,
                        ),
                        EncapsulationKey::MlKem768(
                            generate_keypair_libcrux(&mut rng, kem).unwrap().1,
                        ),
                    ),
                    KEM::XWing => (
                        EncapsulationKey::XWing(generate_keypair_libcrux(&mut rng, kem).unwrap().1),
                        EncapsulationKey::XWing(generate_keypair_libcrux(&mut rng, kem).unwrap().1),
                    ),
                    KEM::X25519 => (
                        EncapsulationKey::X25519(
                            generate_keypair_libcrux(&mut rng, kem).unwrap().1,
                        ),
                        EncapsulationKey::X25519(
                            generate_keypair_libcrux(&mut rng, kem).unwrap().1,
                        ),
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
                    let (mut i_context, i_frame) =
                        anonymous_initiator_process(&mut rng, ciphersuite).unwrap();

                    let i_frame_bytes = i_frame.to_bytes();

                    let (i_frame_r, r_context) = KKTFrame::from_bytes(&i_frame_bytes).unwrap();

                    let (mut r_context, _) =
                        responder_ingest_message(&r_context, None, None, &i_frame_r).unwrap();

                    let r_frame = responder_process(
                        &mut r_context,
                        i_frame_r.session_id_ref(),
                        responder_ed25519_keypair.private_key(),
                        &responder_kem_public_key,
                    )
                    .unwrap();

                    let r_bytes = r_frame.to_bytes();

                    let obtained_key = initiator_ingest_response(
                        &mut i_context,
                        responder_ed25519_keypair.public_key(),
                        &r_dir_hash,
                        &r_bytes,
                    )
                    .unwrap();

                    assert_eq!(obtained_key.encode(), r_kem_key_bytes)
                }
                // Initiator, OneWay
                {
                    let (mut i_context, i_frame) = initiator_process(
                        &mut rng,
                        crate::context::KKTMode::OneWay,
                        ciphersuite,
                        initiator_ed25519_keypair.private_key(),
                        None,
                    )
                    .unwrap();

                    let i_frame_bytes = i_frame.to_bytes();

                    let (i_frame_r, r_context) = KKTFrame::from_bytes(&i_frame_bytes).unwrap();

                    let (mut r_context, r_obtained_key) = responder_ingest_message(
                        &r_context,
                        Some(initiator_ed25519_keypair.public_key()),
                        None,
                        &i_frame_r,
                    )
                    .unwrap();

                    assert!(r_obtained_key.is_none());

                    let r_frame = responder_process(
                        &mut r_context,
                        i_frame_r.session_id_ref(),
                        responder_ed25519_keypair.private_key(),
                        &responder_kem_public_key,
                    )
                    .unwrap();

                    let r_bytes = r_frame.to_bytes();

                    let i_obtained_key = initiator_ingest_response(
                        &mut i_context,
                        responder_ed25519_keypair.public_key(),
                        &r_dir_hash,
                        &r_bytes,
                    )
                    .unwrap();

                    assert_eq!(i_obtained_key.encode(), r_kem_key_bytes)
                }

                // Initiator, Mutual
                {
                    let (mut i_context, i_frame) = initiator_process(
                        &mut rng,
                        crate::context::KKTMode::Mutual,
                        ciphersuite,
                        initiator_ed25519_keypair.private_key(),
                        Some(&initiator_kem_public_key),
                    )
                    .unwrap();

                    let i_frame_bytes = i_frame.to_bytes();

                    let (i_frame_r, r_context) = KKTFrame::from_bytes(&i_frame_bytes).unwrap();

                    let (mut r_context, r_obtained_key) = responder_ingest_message(
                        &r_context,
                        Some(initiator_ed25519_keypair.public_key()),
                        Some(&i_dir_hash),
                        &i_frame_r,
                    )
                    .unwrap();

                    assert_eq!(r_obtained_key.unwrap().encode(), i_kem_key_bytes);

                    let r_frame = responder_process(
                        &mut r_context,
                        i_frame_r.session_id_ref(),
                        responder_ed25519_keypair.private_key(),
                        &responder_kem_public_key,
                    )
                    .unwrap();

                    let r_bytes = r_frame.to_bytes();

                    let obtained_key = initiator_ingest_response(
                        &mut i_context,
                        responder_ed25519_keypair.public_key(),
                        &r_dir_hash,
                        &r_bytes,
                    )
                    .unwrap();

                    assert_eq!(obtained_key.encode(), r_kem_key_bytes)
                }
            }
        }
    }
}
