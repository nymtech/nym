// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod ciphersuite;
pub mod context;
pub mod encryption;
pub mod error;
pub mod frame;
pub mod key_utils;
pub mod kkt;
pub mod session;

// This must be less than 4 bits
pub const KKT_VERSION: u8 = 1;
const _: () = assert!(KKT_VERSION < 1 << 4);

#[cfg(test)]
mod test {
    use crate::kkt::KKT_RESPONSE_AAD;
    use crate::{
        ciphersuite::{Ciphersuite, EncapsulationKey, HashFunction, KEM},
        encryption::{
            decrypt_initial_kkt_frame, decrypt_kkt_frame, encrypt_initial_kkt_frame,
            encrypt_kkt_frame,
        },
        frame::KKTFrame,
        key_utils::{
            generate_keypair_ed25519, generate_keypair_libcrux, generate_keypair_mceliece,
            generate_keypair_x25519, hash_encapsulation_key,
        },
        session::{
            anonymous_initiator_process, initiator_ingest_response, initiator_process,
            responder_ingest_message, responder_process,
        },
    };

    #[test]
    fn test_kkt_psq_e2e_clear() {
        let mut rng = rand::rng();

        // generate ed25519 keys
        let initiator_ed25519_keypair = generate_keypair_ed25519(&mut rng, Some(0));
        let responder_ed25519_keypair = generate_keypair_ed25519(&mut rng, Some(1));

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
                        i_frame_r.session_id(),
                        responder_ed25519_keypair.private_key(),
                        &responder_kem_public_key,
                    )
                    .unwrap();

                    let r_bytes = r_frame.to_bytes();

                    let (i_frame_r, i_context_r) = KKTFrame::from_bytes(&r_bytes).unwrap();

                    let i_obtained_key = initiator_ingest_response(
                        &mut i_context,
                        &i_frame_r,
                        &i_context_r,
                        responder_ed25519_keypair.public_key(),
                        &r_dir_hash,
                    )
                    .unwrap();

                    assert_eq!(i_obtained_key.encode(), r_kem_key_bytes)
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
                        i_frame_r.session_id(),
                        responder_ed25519_keypair.private_key(),
                        &responder_kem_public_key,
                    )
                    .unwrap();

                    let r_bytes = r_frame.to_bytes();

                    let (i_frame_r, i_context_r) = KKTFrame::from_bytes(&r_bytes).unwrap();

                    let i_obtained_key = initiator_ingest_response(
                        &mut i_context,
                        &i_frame_r,
                        &i_context_r,
                        responder_ed25519_keypair.public_key(),
                        &r_dir_hash,
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
                        i_frame_r.session_id(),
                        responder_ed25519_keypair.private_key(),
                        &responder_kem_public_key,
                    )
                    .unwrap();

                    let r_bytes = r_frame.to_bytes();

                    let (i_frame_r, i_context_r) = KKTFrame::from_bytes(&r_bytes).unwrap();

                    let i_obtained_key = initiator_ingest_response(
                        &mut i_context,
                        &i_frame_r,
                        &i_context_r,
                        responder_ed25519_keypair.public_key(),
                        &r_dir_hash,
                    )
                    .unwrap();

                    assert_eq!(i_obtained_key.encode(), r_kem_key_bytes)
                }
            }
        }
    }
    #[test]
    fn test_kkt_psq_e2e_encrypted() {
        let mut rng = rand::rng();

        // generate ed25519 keys
        let initiator_ed25519_keypair = generate_keypair_ed25519(&mut rng, Some(0));
        let responder_ed25519_keypair = generate_keypair_ed25519(&mut rng, Some(1));

        // generate responder x25519 keys
        let responder_x25519_keypair = generate_keypair_x25519(&mut rng);

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

                    // encryption - initiator frame

                    let (i_session_secret, i_bytes) = encrypt_initial_kkt_frame(
                        &mut rng,
                        responder_x25519_keypair.public_key(),
                        &i_frame,
                    )
                    .unwrap();

                    // decryption - initiator frame

                    let (r_session_secret, i_frame_r, i_context_r) =
                        decrypt_initial_kkt_frame(responder_x25519_keypair.private_key(), &i_bytes)
                            .unwrap();

                    let (mut r_context, _) =
                        responder_ingest_message(&i_context_r, None, None, &i_frame_r).unwrap();

                    let r_frame = responder_process(
                        &mut r_context,
                        i_frame_r.session_id(),
                        responder_ed25519_keypair.private_key(),
                        &responder_kem_public_key,
                    )
                    .unwrap();

                    // encryption - responder frame
                    let r_bytes =
                        encrypt_kkt_frame(&mut rng, &r_session_secret, &r_frame, KKT_RESPONSE_AAD)
                            .unwrap();

                    // decryption - responder frame

                    let (i_frame_r, i_context_r) =
                        decrypt_kkt_frame(&i_session_secret, &r_bytes, KKT_RESPONSE_AAD).unwrap();

                    let i_obtained_key = initiator_ingest_response(
                        &mut i_context,
                        &i_frame_r,
                        &i_context_r,
                        responder_ed25519_keypair.public_key(),
                        &r_dir_hash,
                    )
                    .unwrap();

                    assert_eq!(i_obtained_key.encode(), r_kem_key_bytes)
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

                    // encryption - initiator frame

                    let (i_session_secret, i_bytes) = encrypt_initial_kkt_frame(
                        &mut rng,
                        responder_x25519_keypair.public_key(),
                        &i_frame,
                    )
                    .unwrap();

                    // decryption - initiator frame

                    let (r_session_secret, i_frame_r, r_context) =
                        decrypt_initial_kkt_frame(responder_x25519_keypair.private_key(), &i_bytes)
                            .unwrap();

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
                        i_frame_r.session_id(),
                        responder_ed25519_keypair.private_key(),
                        &responder_kem_public_key,
                    )
                    .unwrap();

                    // encryption - responder frame
                    let r_bytes =
                        encrypt_kkt_frame(&mut rng, &r_session_secret, &r_frame, KKT_RESPONSE_AAD)
                            .unwrap();

                    // decryption - responder frame

                    let (i_frame_r, i_context_r) =
                        decrypt_kkt_frame(&i_session_secret, &r_bytes, KKT_RESPONSE_AAD).unwrap();

                    let i_obtained_key = initiator_ingest_response(
                        &mut i_context,
                        &i_frame_r,
                        &i_context_r,
                        responder_ed25519_keypair.public_key(),
                        &r_dir_hash,
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

                    // encryption - initiator frame

                    let (i_session_secret, i_bytes) = encrypt_initial_kkt_frame(
                        &mut rng,
                        responder_x25519_keypair.public_key(),
                        &i_frame,
                    )
                    .unwrap();

                    // decryption - initiator frame

                    let (r_session_secret, i_frame_r, i_context_r) =
                        decrypt_initial_kkt_frame(responder_x25519_keypair.private_key(), &i_bytes)
                            .unwrap();

                    let (mut r_context, r_obtained_key) = responder_ingest_message(
                        &i_context_r,
                        Some(initiator_ed25519_keypair.public_key()),
                        Some(&i_dir_hash),
                        &i_frame_r,
                    )
                    .unwrap();

                    assert_eq!(r_obtained_key.unwrap().encode(), i_kem_key_bytes);

                    let r_frame = responder_process(
                        &mut r_context,
                        i_frame_r.session_id(),
                        responder_ed25519_keypair.private_key(),
                        &responder_kem_public_key,
                    )
                    .unwrap();

                    // encryption - responder frame
                    let r_bytes =
                        encrypt_kkt_frame(&mut rng, &r_session_secret, &r_frame, KKT_RESPONSE_AAD)
                            .unwrap();

                    // decryption - responder frame

                    let (i_frame_r, i_context_r) =
                        decrypt_kkt_frame(&i_session_secret, &r_bytes, KKT_RESPONSE_AAD).unwrap();

                    let i_obtained_key = initiator_ingest_response(
                        &mut i_context,
                        &i_frame_r,
                        &i_context_r,
                        responder_ed25519_keypair.public_key(),
                        &r_dir_hash,
                    )
                    .unwrap();

                    assert_eq!(i_obtained_key.encode(), r_kem_key_bytes)
                }
            }
        }
    }
}
