// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod ciphersuite;
pub mod context;
// pub mod encryption;
pub mod error;
pub mod frame;
pub mod key_utils;
// pub mod kkt;
pub mod carrier;
pub mod masked_byte;
pub mod rekey;
pub mod session;

// This must be less than 4 bits
pub const KKT_VERSION: u8 = 1;
const _: () = assert!(KKT_VERSION < 1 << 4);
pub const KKT_RESPONSE_AAD: &[u8] = b"KKT_Response";
pub(crate) const KKT_INITIAL_FRAME_AAD: &[u8] = b"KKT_INITIAL_FRAME";

#[cfg(test)]
mod test {
    use crate::{
        KKT_RESPONSE_AAD,
        carrier::Carrier,
        ciphersuite::{Ciphersuite, EncapsulationKey, HashFunction, KEM},
        context::KKTMode,
        frame::KKTFrame,
        key_utils::{
            generate_keypair_libcrux, generate_keypair_mceliece, generate_keypair_mlkem,
            generate_keypair_x25519, hash_encapsulation_key,
        },
        session::{
            initiator_ingest_response, initiator_process, responder_ingest_message,
            responder_process,
        },
    };

    #[test]
    fn test_kkt_psq_e2e() {
        let mut rng = rand09::rng();

        for encryption in [false, true] {
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
                        crate::ciphersuite::SignatureScheme::Ed25519,
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
                            EncapsulationKey::XWing(
                                generate_keypair_libcrux(&mut rng, kem).unwrap().1,
                            ),
                            EncapsulationKey::XWing(
                                generate_keypair_libcrux(&mut rng, kem).unwrap().1,
                            ),
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

                    // OneWay
                    {
                        let (mut i_context, i_frame) =
                            initiator_process(&mut rng, KKTMode::OneWay, ciphersuite, None)
                                .unwrap();

                        let i_frame_bytes = i_frame.to_bytes();

                        let (i_frame_r, r_context) = KKTFrame::from_bytes(&i_frame_bytes).unwrap();

                        let (mut r_context, r_obtained_key) =
                            responder_ingest_message(&r_context, None, &i_frame_r).unwrap();

                        assert!(r_obtained_key.is_none());

                        let r_frame = responder_process(
                            &mut r_context,
                            i_frame_r.session_id(),
                            &responder_kem_public_key,
                        )
                        .unwrap();

                        let r_bytes = r_frame.to_bytes();

                        let (i_frame_r, i_context_r) = KKTFrame::from_bytes(&r_bytes).unwrap();

                        let i_obtained_key = initiator_ingest_response(
                            &mut i_context,
                            &i_frame_r,
                            &i_context_r,
                            &r_dir_hash,
                        )
                        .unwrap();

                        assert_eq!(i_obtained_key.encode(), r_kem_key_bytes)
                    }

                    // Mutual
                    {
                        let (mut i_context, i_frame) = initiator_process(
                            &mut rng,
                            KKTMode::Mutual,
                            ciphersuite,
                            Some(&initiator_kem_public_key),
                        )
                        .unwrap();

                        let i_frame_bytes = i_frame.to_bytes();

                        let (i_frame_r, r_context) = KKTFrame::from_bytes(&i_frame_bytes).unwrap();

                        let (mut r_context, r_obtained_key) =
                            responder_ingest_message(&r_context, Some(&i_dir_hash), &i_frame_r)
                                .unwrap();

                        assert_eq!(r_obtained_key.unwrap().encode(), i_kem_key_bytes);

                        let r_frame = responder_process(
                            &mut r_context,
                            i_frame_r.session_id(),
                            &responder_kem_public_key,
                        )
                        .unwrap();

                        let r_bytes = r_frame.to_bytes();

                        let (i_frame_r, i_context_r) = KKTFrame::from_bytes(&r_bytes).unwrap();

                        let i_obtained_key = initiator_ingest_response(
                            &mut i_context,
                            &i_frame_r,
                            &i_context_r,
                            &r_dir_hash,
                        )
                        .unwrap();

                        assert_eq!(i_obtained_key.encode(), r_kem_key_bytes)
                    }
                }
            }
        }
    }

    #[test]
    fn test_kkt_psq_e2e_encrypted_carrier() {
        let mut rng = rand09::rng();

        // generate responder x25519 keys
        let responder_x25519_keypair = generate_keypair_x25519(&mut rng);

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
                    crate::ciphersuite::SignatureScheme::Ed25519,
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

                // OneWay
                {
                    let (mut i_context, i_frame) =
                        initiator_process(&mut rng, KKTMode::OneWay, ciphersuite, None).unwrap();

                    // encryption - initiator frame
                    let (mut i_carrier, i_bytes) = Carrier::new_kkt_initiator(
                        &mut rng,
                        &responder_x25519_keypair.pk,
                        1u8,
                        &i_frame,
                    )
                    .unwrap();

                    // decryption - initiator frame

                    let (mut r_carrier, i_frame_r, i_context_r) =
                        Carrier::new_kkt_responder(&responder_x25519_keypair, &i_bytes, &[1])
                            .unwrap();

                    let (mut r_context, _) =
                        responder_ingest_message(&i_context_r, None, &i_frame_r).unwrap();

                    let r_frame = responder_process(
                        &mut r_context,
                        i_frame_r.session_id(),
                        &responder_kem_public_key,
                    )
                    .unwrap();

                    // encryption - responder frame
                    let r_bytes = r_carrier.encrypt(&r_frame.to_bytes()).unwrap();

                    // decryption - responder frame

                    let (i_frame_r, i_context_r) =
                        KKTFrame::from_bytes(&i_carrier.decrypt(&r_bytes).unwrap()).unwrap();

                    let i_obtained_key = initiator_ingest_response(
                        &mut i_context,
                        &i_frame_r,
                        &i_context_r,
                        &r_dir_hash,
                    )
                    .unwrap();

                    assert_eq!(i_obtained_key.encode(), r_kem_key_bytes)
                }
                // Mutual
                {
                    let (mut i_context, i_frame) = initiator_process(
                        &mut rng,
                        KKTMode::Mutual,
                        ciphersuite,
                        Some(&initiator_kem_public_key),
                    )
                    .unwrap();

                    // encryption - initiator frame
                    let (mut i_carrier, i_bytes) = Carrier::new_kkt_initiator(
                        &mut rng,
                        &responder_x25519_keypair.pk,
                        1u8,
                        &i_frame,
                    )
                    .unwrap();

                    // decryption - initiator frame

                    let (mut r_carrier, i_frame_r, i_context_r) =
                        Carrier::new_kkt_responder(&responder_x25519_keypair, &i_bytes, &[1])
                            .unwrap();

                    let (mut r_context, _) = responder_ingest_message(
                        &i_context_r,
                        Some(i_dir_hash.as_slice()),
                        &i_frame_r,
                    )
                    .unwrap();

                    let r_frame = responder_process(
                        &mut r_context,
                        i_frame_r.session_id(),
                        &responder_kem_public_key,
                    )
                    .unwrap();

                    // encryption - responder frame
                    let r_bytes = r_carrier.encrypt(&r_frame.to_bytes()).unwrap();

                    // decryption - responder frame

                    let (i_frame_r, i_context_r) =
                        KKTFrame::from_bytes(&i_carrier.decrypt(&r_bytes).unwrap()).unwrap();

                    let i_obtained_key = initiator_ingest_response(
                        &mut i_context,
                        &i_frame_r,
                        &i_context_r,
                        &r_dir_hash,
                    )
                    .unwrap();

                    assert_eq!(i_obtained_key.encode(), r_kem_key_bytes)
                }
            }
        }
    }
}
