// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod carrier;
pub mod ciphersuite;
pub mod context;
pub mod error;
pub mod frame;
pub mod initiator;
pub mod key_utils;
pub mod masked_byte;
pub mod rekey;
pub mod responder;

// This must be less than 4 bits
pub const KKT_VERSION: u8 = 1;
const _: () = assert!(KKT_VERSION < 1 << 4);

#[cfg(test)]
mod test {
    use nym_kkt_ciphersuite::SignatureScheme;

    use crate::{
        ciphersuite::{Ciphersuite, EncapsulationKey, HashFunction, KEM},
        initiator::KKTInitiator,
        key_utils::{
            generate_keypair_mceliece, generate_keypair_mlkem, generate_keypair_x25519,
            hash_encapsulation_key,
        },
        responder::KKTResponder,
    };

    #[test]
    fn test_kkt_psq_e2e_encrypted_carrier() {
        let mut rng = rand09::rng();

        // generate responder x25519 keys
        let responder_x25519_keypair = generate_keypair_x25519(&mut rng);

        for kem in [KEM::MlKem768, KEM::McEliece] {
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
                    KEM::McEliece => (
                        EncapsulationKey::McEliece(generate_keypair_mceliece(&mut rng).1),
                        EncapsulationKey::McEliece(generate_keypair_mceliece(&mut rng).1),
                    ),
                    _ => unreachable!(), // KEM::XWing => (
                                         //     EncapsulationKey::XWing(generate_keypair_libcrux(&mut rng, kem).unwrap().1),
                                         //     EncapsulationKey::XWing(generate_keypair_libcrux(&mut rng, kem).unwrap().1),
                                         // ),
                                         // KEM::X25519 => (
                                         //     EncapsulationKey::X25519(
                                         //         generate_keypair_libcrux(&mut rng, kem).unwrap().1,
                                         //     ),
                                         //     EncapsulationKey::X25519(
                                         //         generate_keypair_libcrux(&mut rng, kem).unwrap().1,
                                         //     ),
                                         // ),
                };

                let i_kem_key_bytes = initiator_kem_public_key.encode();

                let r_kem_key_bytes = responder_kem_public_key.encode();

                let _i_dir_hash = hash_encapsulation_key(
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
                    let (mut initiator, request_bytes) = KKTInitiator::generate_one_way_request(
                        &mut rng,
                        &ciphersuite,
                        &responder_x25519_keypair.pk,
                        &r_dir_hash,
                        1u8,
                    )
                    .unwrap();
                    let responder = if kem == KEM::McEliece {
                        KKTResponder::new(
                            &responder_x25519_keypair,
                            None,
                            Some(&responder_kem_public_key),
                            &[
                                HashFunction::Blake3,
                                HashFunction::SHA256,
                                HashFunction::Shake128,
                                HashFunction::Shake256,
                            ],
                            &[1],
                            &[SignatureScheme::Ed25519],
                        )
                        .unwrap()
                    } else if kem == KEM::MlKem768 {
                        KKTResponder::new(
                            &responder_x25519_keypair,
                            Some(&responder_kem_public_key),
                            None,
                            &[
                                HashFunction::Blake3,
                                HashFunction::SHA256,
                                HashFunction::Shake128,
                                HashFunction::Shake256,
                            ],
                            &[1],
                            &[SignatureScheme::Ed25519],
                        )
                        .unwrap()
                    } else {
                        unreachable!();
                    };

                    let (response_bytes, _) = responder.process_request(&request_bytes).unwrap();

                    let (i_obtained_key, _) = initiator.process_response(&response_bytes).unwrap();

                    assert_eq!(i_obtained_key.encode(), r_kem_key_bytes)
                }
                // Mutual
                {
                    let (mut initiator, request_bytes) = KKTInitiator::generate_mutual_request(
                        &mut rng,
                        &ciphersuite,
                        &initiator_kem_public_key,
                        &responder_x25519_keypair.pk,
                        &r_dir_hash,
                        1u8,
                    )
                    .unwrap();
                    let responder = if kem == KEM::McEliece {
                        KKTResponder::new(
                            &responder_x25519_keypair,
                            None,
                            Some(&responder_kem_public_key),
                            &[
                                HashFunction::Blake3,
                                HashFunction::SHA256,
                                HashFunction::Shake128,
                                HashFunction::Shake256,
                            ],
                            &[1],
                            &[SignatureScheme::Ed25519],
                        )
                        .unwrap()
                    } else if kem == KEM::MlKem768 {
                        KKTResponder::new(
                            &responder_x25519_keypair,
                            Some(&responder_kem_public_key),
                            None,
                            &[
                                HashFunction::Blake3,
                                HashFunction::SHA256,
                                HashFunction::Shake128,
                                HashFunction::Shake256,
                            ],
                            &[1],
                            &[SignatureScheme::Ed25519],
                        )
                        .unwrap()
                    } else {
                        unreachable!();
                    };

                    let (response_bytes, r_obtained_key) =
                        responder.process_request(&request_bytes).unwrap();

                    // if we keep unverified keys, this should change
                    assert!(r_obtained_key.is_none());

                    let (i_obtained_key, _) = initiator.process_response(&response_bytes).unwrap();

                    assert_eq!(i_obtained_key.encode(), r_kem_key_bytes)
                }
            }
        }
    }
}
