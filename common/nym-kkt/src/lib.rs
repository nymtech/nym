// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod carrier;
pub mod context;
pub mod error;
pub mod frame;
pub mod initiator;
pub mod key_utils;
pub mod keys;
pub mod masked_byte;
pub mod rekey;
pub mod responder;

// This must be less than 4 bits
pub const KKT_VERSION: u8 = 1;
const _: () = assert!(KKT_VERSION < 1 << 4);

#[cfg(test)]
mod test {
    use nym_kkt_ciphersuite::{Ciphersuite, HashFunction, HashLength, KEM, SignatureScheme};

    use crate::{
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

        for hash_function in [
            HashFunction::Blake3,
            HashFunction::SHA256,
            HashFunction::Shake128,
            HashFunction::Shake256,
        ] {
            // generate kem public keys

            let responder_mlkem_keypair = generate_keypair_mlkem(&mut rng);

            let responder_mceliece_keypair = generate_keypair_mceliece(&mut rng);

            let r_dir_hash_mlkem = hash_encapsulation_key(
                // &ciphersuite.hash_function(),
                &hash_function,
                // ciphersuite.hash_len(),
                HashLength::Default.value(),
                responder_mlkem_keypair.1.as_slice().as_slice(),
            );

            let r_dir_hash_mceliece = hash_encapsulation_key(
                // &ciphersuite.hash_function(),
                &hash_function,
                // ciphersuite.hash_len(),
                HashLength::Default.value(),
                responder_mceliece_keypair.1.as_ref(),
            );
            let initiator_mlkem_keypair = generate_keypair_mlkem(&mut rng);
            let initiator_mceliece_keypair = generate_keypair_mceliece(&mut rng);

            let _i_dir_hash_mlkem = hash_encapsulation_key(
                // &ciphersuite.hash_function(),
                &hash_function,
                // ciphersuite.hash_len(),
                HashLength::Default.value(),
                initiator_mlkem_keypair.1.as_slice().as_slice(),
            );

            let _i_dir_hash_mceliece = hash_encapsulation_key(
                // &ciphersuite.hash_function(),
                &hash_function,
                // ciphersuite.hash_len(),
                HashLength::Default.value(),
                initiator_mceliece_keypair.1.as_ref(),
            );

            let responder = KKTResponder::new(
                &responder_x25519_keypair,
                Some(&responder_mlkem_keypair.1),
                Some(&responder_mceliece_keypair.1),
                &[
                    HashFunction::Blake3,
                    HashFunction::SHA256,
                    HashFunction::Shake128,
                    HashFunction::Shake256,
                ],
                &[1],
                &[SignatureScheme::Ed25519],
            )
            .unwrap();

            // OneWay - MlKem
            {
                let ciphersuite = Ciphersuite::resolve_ciphersuite(
                    KEM::MlKem768,
                    hash_function,
                    SignatureScheme::Ed25519,
                    None,
                )
                .unwrap();
                let (mut initiator, request_bytes) = KKTInitiator::generate_one_way_request(
                    &mut rng,
                    &ciphersuite,
                    &responder_x25519_keypair.pk,
                    &r_dir_hash_mlkem,
                    1u8,
                )
                .unwrap();

                let (response_bytes, _) = responder.process_request(&request_bytes).unwrap();

                let (i_obtained_key, _) = initiator.process_response(&response_bytes).unwrap();

                assert_eq!(
                    i_obtained_key,
                    responder_mlkem_keypair.1.as_slice().as_slice(),
                )
            }
            // Mutual - MlKem
            {
                let ciphersuite = Ciphersuite::resolve_ciphersuite(
                    KEM::MlKem768,
                    hash_function,
                    SignatureScheme::Ed25519,
                    None,
                )
                .unwrap();
                let (mut initiator, request_bytes) = KKTInitiator::generate_one_way_request(
                    &mut rng,
                    &ciphersuite,
                    &responder_x25519_keypair.pk,
                    &r_dir_hash_mlkem,
                    1u8,
                )
                .unwrap();

                let (response_bytes, r_obtained_key) =
                    responder.process_request(&request_bytes).unwrap();

                // if we keep unverified keys, this should change
                assert!(r_obtained_key.is_none());

                let (i_obtained_key, _) = initiator.process_response(&response_bytes).unwrap();

                assert_eq!(
                    i_obtained_key,
                    responder_mlkem_keypair.1.as_slice().as_slice(),
                )
            }

            // OneWay - McEliece
            {
                let ciphersuite = Ciphersuite::resolve_ciphersuite(
                    KEM::McEliece,
                    hash_function,
                    SignatureScheme::Ed25519,
                    None,
                )
                .unwrap();
                let (mut initiator, request_bytes) = KKTInitiator::generate_one_way_request(
                    &mut rng,
                    &ciphersuite,
                    &responder_x25519_keypair.pk,
                    &r_dir_hash_mceliece,
                    1u8,
                )
                .unwrap();

                let (response_bytes, _) = responder.process_request(&request_bytes).unwrap();

                let (i_obtained_key, _) = initiator.process_response(&response_bytes).unwrap();

                assert_eq!(i_obtained_key, responder_mceliece_keypair.1.as_ref(),)
            }
            // Mutual - MlKem
            {
                let ciphersuite = Ciphersuite::resolve_ciphersuite(
                    KEM::McEliece,
                    hash_function,
                    SignatureScheme::Ed25519,
                    None,
                )
                .unwrap();
                let (mut initiator, request_bytes) = KKTInitiator::generate_one_way_request(
                    &mut rng,
                    &ciphersuite,
                    &responder_x25519_keypair.pk,
                    &r_dir_hash_mceliece,
                    1u8,
                )
                .unwrap();

                let (response_bytes, r_obtained_key) =
                    responder.process_request(&request_bytes).unwrap();

                // if we keep unverified keys, this should change
                assert!(r_obtained_key.is_none());

                let (i_obtained_key, _) = initiator.process_response(&response_bytes).unwrap();

                assert_eq!(i_obtained_key, responder_mceliece_keypair.1.as_ref(),)
            }
        }
    }
}
