// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod carrier;
pub mod error;
pub mod frame;
pub mod initiator;
pub mod key_utils;
pub mod keys;
pub mod masked_byte;
pub mod message;
pub mod rekey;
pub mod responder;

pub use nym_kkt_context as context;

#[cfg(test)]
mod test {
    use crate::keys::KEMKeys;
    use crate::{
        initiator::KKTInitiator,
        key_utils::{
            generate_keypair_mceliece, generate_keypair_mlkem, generate_lp_keypair_x25519,
            hash_encapsulation_key,
        },
        responder::KKTResponder,
    };
    use nym_kkt_ciphersuite::{Ciphersuite, HashFunction, HashLength, KEM, SignatureScheme};
    use nym_test_utils::helpers::deterministic_rng_09;
    use rand09::RngCore;
    use std::collections::BTreeMap;

    #[test]
    fn test_kkt_psq_e2e_one_way_encrypted_carrier() {
        let mut rng = rand09::rng();

        let mut payload: Vec<u8> = vec![0u8; 900_000];
        rng.fill_bytes(&mut payload);

        // generate responder x25519 keys
        let responder_x25519_keypair = generate_lp_keypair_x25519(&mut rng);

        for hash_function in [
            HashFunction::Blake3,
            HashFunction::SHA256,
            HashFunction::Shake128,
            HashFunction::Shake256,
        ] {
            // generate kem public keys
            let responder_mlkem_keypair = generate_keypair_mlkem(&mut rng);
            let responder_mceliece_keypair = generate_keypair_mceliece(&mut rng);

            let responder_kem = KEMKeys::new(responder_mceliece_keypair, responder_mlkem_keypair);

            let r_dir_hash_mlkem = hash_encapsulation_key(
                hash_function,
                HashLength::Default.value(),
                responder_kem.ml_kem768_encapsulation_key().as_slice(),
            );

            let r_dir_hash_mceliece = hash_encapsulation_key(
                hash_function,
                HashLength::Default.value(),
                responder_kem.mc_eliece_encapsulation_key().as_ref(),
            );

            let init_hashes = BTreeMap::new();

            let responder = KKTResponder::new(
                &responder_x25519_keypair,
                &responder_kem,
                &init_hashes,
                &[
                    HashFunction::Blake3,
                    HashFunction::SHA256,
                    HashFunction::Shake128,
                    HashFunction::Shake256,
                ],
                &[SignatureScheme::Ed25519],
                &[1],
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
                let (mut initiator, request) = KKTInitiator::generate_one_way_request(
                    &mut rng,
                    ciphersuite,
                    &responder_x25519_keypair.pk,
                    &r_dir_hash_mlkem,
                    1u8,
                    Some(payload.clone()),
                )
                .unwrap();

                let processed_request = responder.process_request(request, payload.len()).unwrap();

                assert_eq!(processed_request.request_payload, payload);

                let result = initiator
                    .process_response(processed_request.response, 0)
                    .unwrap();

                assert_eq!(
                    result.encapsulation_key.as_bytes(),
                    responder_kem.ml_kem768_encapsulation_key().as_slice(),
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
                let (mut initiator, request) = KKTInitiator::generate_one_way_request(
                    &mut rng,
                    ciphersuite,
                    &responder_x25519_keypair.pk,
                    &r_dir_hash_mceliece,
                    1u8,
                    Some(payload.clone()),
                )
                .unwrap();

                let processed_request = responder.process_request(request, payload.len()).unwrap();
                assert_eq!(processed_request.request_payload, payload);

                let processed_response = initiator
                    .process_response(processed_request.response, 0)
                    .unwrap();

                assert_eq!(
                    processed_response.encapsulation_key.as_bytes(),
                    responder_kem.mc_eliece_encapsulation_key().as_ref()
                )
            }
        }
    }

    #[test]
    fn test_kkt_psq_e2e_mutual_encrypted_carrier() {
        let mut rng = deterministic_rng_09();

        let mut payload: Vec<u8> = vec![0u8; 50000];
        rng.fill_bytes(&mut payload);

        // generate kem public keys
        let initiator_mlkem_keypair = generate_keypair_mlkem(&mut rng);
        let initiator_mceliece_keypair = generate_keypair_mceliece(&mut rng);

        let responder_mlkem_keypair = generate_keypair_mlkem(&mut rng);
        let responder_mceliece_keypair = generate_keypair_mceliece(&mut rng);

        let responder_x25519_keypair = generate_lp_keypair_x25519(&mut rng);

        let initiator_kem = KEMKeys::new(initiator_mceliece_keypair, initiator_mlkem_keypair);
        let responder_kem = KEMKeys::new(responder_mceliece_keypair, responder_mlkem_keypair);

        let init_hashes = initiator_kem.encapsulation_keys_digests();

        let responder = KKTResponder::new(
            &responder_x25519_keypair,
            &responder_kem,
            &init_hashes,
            &[
                HashFunction::Blake3,
                HashFunction::SHA256,
                HashFunction::Shake128,
                HashFunction::Shake256,
            ],
            &[SignatureScheme::Ed25519],
            &[1],
        )
        .unwrap();

        for hash_function in [
            HashFunction::Blake3,
            HashFunction::SHA256,
            HashFunction::Shake128,
            HashFunction::Shake256,
        ] {
            let r_dir_hash_mlkem = hash_encapsulation_key(
                hash_function,
                HashLength::Default.value(),
                responder_kem.ml_kem768_encapsulation_key().as_slice(),
            );

            let r_dir_hash_mceliece = hash_encapsulation_key(
                hash_function,
                HashLength::Default.value(),
                responder_kem.mc_eliece_encapsulation_key().as_ref(),
            );

            // Mutual - MlKem
            {
                let ciphersuite = Ciphersuite::resolve_ciphersuite(
                    KEM::MlKem768,
                    hash_function,
                    SignatureScheme::Ed25519,
                    None,
                )
                .unwrap();
                let (mut initiator, request) = KKTInitiator::generate_mutual_request(
                    &mut rng,
                    ciphersuite,
                    initiator_kem
                        .encoded_encapsulation_key(KEM::MlKem768)
                        .unwrap(),
                    &responder_x25519_keypair.pk,
                    &r_dir_hash_mlkem,
                    1u8,
                    Some(payload.clone()),
                )
                .unwrap();

                let processed_request = responder.process_request(request, payload.len()).unwrap();

                assert_eq!(processed_request.request_payload, payload);
                assert_eq!(
                    processed_request
                        .remote_encapsulation_key
                        .unwrap()
                        .as_bytes(),
                    initiator_kem
                        .encapsulation_key(KEM::MlKem768)
                        .unwrap()
                        .as_bytes()
                );

                let processed_response = initiator
                    .process_response(processed_request.response, 0)
                    .unwrap();

                assert_eq!(
                    processed_response.encapsulation_key.as_bytes(),
                    responder_kem.ml_kem768_encapsulation_key().as_slice(),
                )
            }

            // Mutual - McEliece is not supported due to the key being too large
            {
                let ciphersuite = Ciphersuite::resolve_ciphersuite(
                    KEM::McEliece,
                    hash_function,
                    SignatureScheme::Ed25519,
                    None,
                )
                .unwrap();
                let (mut initiator, request) = KKTInitiator::generate_mutual_request(
                    &mut rng,
                    ciphersuite,
                    initiator_kem
                        .encoded_encapsulation_key(KEM::McEliece)
                        .unwrap(),
                    &responder_x25519_keypair.pk,
                    &r_dir_hash_mceliece,
                    1u8,
                    Some(payload.clone()),
                )
                .unwrap();

                let processed_request = responder.process_request(request, payload.len()).unwrap();

                assert_eq!(processed_request.request_payload, payload);
                assert_eq!(
                    processed_request
                        .remote_encapsulation_key
                        .unwrap()
                        .as_bytes(),
                    initiator_kem
                        .encapsulation_key(KEM::McEliece)
                        .unwrap()
                        .as_bytes()
                );

                let processed_response = initiator
                    .process_response(processed_request.response, 0)
                    .unwrap();

                assert_eq!(
                    processed_response.encapsulation_key.as_bytes(),
                    responder_kem.mc_eliece_encapsulation_key().as_ref()
                )
            }
        }
    }
}
