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
    use nym_kkt_ciphersuite::{Ciphersuite, HashFunction, HashLength, KEM, SignatureScheme};
    use rand09::RngCore;

    use crate::keys::KEMKeys;
    use crate::{
        initiator::KKTInitiator,
        key_utils::{
            generate_keypair_mceliece, generate_keypair_mlkem, generate_lp_keypair_x25519,
            hash_encapsulation_key,
        },
        responder::KKTResponder,
    };

    #[test]
    fn test_kkt_psq_e2e_encrypted_carrier() {
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
            let initiator_mlkem_keypair = generate_keypair_mlkem(&mut rng);
            let initiator_mceliece_keypair = generate_keypair_mceliece(&mut rng);

            let _i_dir_hash_mlkem = hash_encapsulation_key(
                hash_function,
                HashLength::Default.value(),
                initiator_mlkem_keypair.public_key().as_slice(),
            );

            let _i_dir_hash_mceliece = hash_encapsulation_key(
                hash_function,
                HashLength::Default.value(),
                initiator_mceliece_keypair.pk.as_ref(),
            );

            let responder = KKTResponder::new(
                &responder_x25519_keypair,
                &responder_kem,
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

                let processed_request = responder
                    .process_request(request.request, payload.len())
                    .unwrap();

                assert_eq!(processed_request.request_payload, payload);

                let result = initiator
                    .process_response(processed_request.response, 0)
                    .unwrap();

                assert_eq!(
                    result.encapsulation_key.as_bytes(),
                    responder_kem.ml_kem768_encapsulation_key().as_slice(),
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
                let (mut initiator, request) = KKTInitiator::generate_one_way_request(
                    &mut rng,
                    ciphersuite,
                    &responder_x25519_keypair.pk,
                    &r_dir_hash_mlkem,
                    1u8,
                    Some(payload.clone()),
                )
                .unwrap();

                let processed_request = responder
                    .process_request(request.request, payload.len())
                    .unwrap();

                assert_eq!(processed_request.request_payload, payload);

                // if we keep unverified keys, this should change
                assert!(processed_request.remote_encapsulation_key.is_none());

                let processed_response = initiator
                    .process_response(processed_request.response, 0)
                    .unwrap();

                assert_eq!(
                    processed_response.encapsulation_key.as_bytes(),
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

                let processed_request = responder
                    .process_request(request.request, payload.len())
                    .unwrap();
                assert_eq!(processed_request.request_payload, payload);

                let processed_response = initiator
                    .process_response(processed_request.response, 0)
                    .unwrap();

                assert_eq!(
                    processed_response.encapsulation_key.as_bytes(),
                    responder_kem.mc_eliece_encapsulation_key().as_ref()
                )
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
                let (mut initiator, request) = KKTInitiator::generate_one_way_request(
                    &mut rng,
                    ciphersuite,
                    &responder_x25519_keypair.pk,
                    &r_dir_hash_mceliece,
                    1u8,
                    Some(payload.clone()),
                )
                .unwrap();

                let processed_request = responder
                    .process_request(request.request, payload.len())
                    .unwrap();

                assert_eq!(processed_request.request_payload, payload);

                // if we keep unverified keys, this should change
                assert!(processed_request.remote_encapsulation_key.is_none());

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
