#[cfg(test)]
mod tests {

    use libcrux_psq::{
        Channel, IntoSession,
        handshake::{
            builders::{CiphersuiteBuilder, PrincipalBuilder},
            ciphersuites::CiphersuiteName,
            types::{Authenticator, PQEncapsulationKey},
        },
        session::{Session, SessionBinding},
    };
    use nym_kkt::{
        initiator::KKTInitiator,
        key_utils::{
            generate_keypair_mceliece, generate_keypair_mlkem, generate_keypair_x25519,
            hash_encapsulation_key,
        },
        responder::KKTResponder,
    };
    use nym_kkt_ciphersuite::{Ciphersuite, HashFunction, HashLength, KEM, SignatureScheme};

    #[test]
    fn test_e2e_client_node() {
        let mut rng = rand09::rng();

        // we should add these as consts
        let aad_initiator_outer = b"Test Data I Outer";
        let aad_initiator_inner = b"Test Data I Inner";
        let aad_responder = b"Test Data R";
        let ctx = b"Test Context";

        // generate responder x25519 keys
        let responder_x25519_keypair = generate_keypair_x25519(&mut rng);
        let hash_function = HashFunction::Blake3;
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

        let _r_dir_hash_mceliece = hash_encapsulation_key(
            // &ciphersuite.hash_function(),
            &hash_function,
            // ciphersuite.hash_len(),
            HashLength::Default.value(),
            responder_mceliece_keypair.1.as_ref(),
        );

        let kkt_responder = KKTResponder::new(
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
        let psq_ciphersuite = CiphersuiteName::X25519_MLKEM768_X25519_AESGCM128_HKDFSHA256;

        let responder_ciphersuite = CiphersuiteBuilder::new(psq_ciphersuite)
            .longterm_x25519_keys(&responder_x25519_keypair)
            .longterm_mlkem_encapsulation_key(&responder_mlkem_keypair.1)
            .longterm_mlkem_decapsulation_key(&responder_mlkem_keypair.0)
            .build_responder_ciphersuite()
            .unwrap();

        let mut responder = PrincipalBuilder::new(rand09::rng())
            .context(ctx)
            .outer_aad(aad_responder)
            .recent_keys_upper_bound(30)
            .build_responder(responder_ciphersuite)
            .unwrap();

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

        let (response_bytes, _) = kkt_responder.process_request(&request_bytes).unwrap();

        let (i_obtained_key, _) = initiator.process_response(&response_bytes).unwrap();

        assert_eq!(
            i_obtained_key,
            responder_mlkem_keypair.1.as_slice().as_slice(),
        );

        let mlkem_key =
            libcrux_kem::MlKem768PublicKey::try_from(i_obtained_key.as_slice()).unwrap();

        let initiator_psq_keys = generate_keypair_x25519(&mut rng);
        let initiator_cbuilder = CiphersuiteBuilder::new(psq_ciphersuite)
            .longterm_x25519_keys(&initiator_psq_keys)
            .peer_longterm_x25519_pk(&responder_x25519_keypair.pk)
            .peer_longterm_mlkem_pk(&mlkem_key);

        let initiator_ciphersuite = initiator_cbuilder.build_initiator_ciphersuite().unwrap();

        let mut msg_channel = vec![0u8; 8192];
        let mut payload_buf_responder = vec![0u8; 4096];
        let mut payload_buf_initiator = vec![0u8; 4096];

        let mut initiator = PrincipalBuilder::new(rand09::rng())
            .outer_aad(aad_initiator_outer)
            .inner_aad(aad_initiator_inner)
            .context(ctx)
            .build_registration_initiator(initiator_ciphersuite)
            .unwrap();

        // Send first message
        let registration_payload_initiator = b"Registration_init";
        let len_i = initiator
            .write_message(registration_payload_initiator, &mut msg_channel)
            .unwrap();

        // Read first message
        let (len_r_deserialized, len_r_payload) = responder
            .read_message(&msg_channel, &mut payload_buf_responder)
            .unwrap();

        // We read the same amount of data.
        assert_eq!(len_r_deserialized, len_i);
        assert_eq!(len_r_payload, registration_payload_initiator.len());
        assert_eq!(
            &payload_buf_responder[0..len_r_payload],
            registration_payload_initiator
        );

        // Get the authenticator out here, so we can deserialize the session later.
        let Some(initiator_authenticator) = responder.initiator_authenticator() else {
            panic!("No initiator authenticator found")
        };

        // Respond
        let registration_payload_responder = b"Registration_respond";
        let len_r = responder
            .write_message(registration_payload_responder, &mut msg_channel)
            .unwrap();

        // Finalize on registration initiator
        let (len_i_deserialized, len_i_payload) = initiator
            .read_message(&msg_channel, &mut payload_buf_initiator)
            .unwrap();

        // We read the same amount of data.
        assert_eq!(len_r, len_i_deserialized);
        assert_eq!(registration_payload_responder.len(), len_i_payload);
        assert_eq!(
            &payload_buf_initiator[0..len_i_payload],
            registration_payload_responder
        );

        // Ready for transport mode
        assert!(initiator.is_handshake_finished());
        assert!(responder.is_handshake_finished());

        let i_transport = initiator.into_session().unwrap();
        let r_transport = responder.into_session().unwrap();

        // test serialization, deserialization
        let mut session_storage = vec![0u8; 4096];
        i_transport
            .serialize(
                &mut session_storage,
                SessionBinding {
                    initiator_authenticator: &Authenticator::Dh(initiator_psq_keys.pk),
                    responder_ecdh_pk: &responder_x25519_keypair.pk,
                    responder_pq_pk: Some(PQEncapsulationKey::MlKem(&mlkem_key)),
                },
            )
            .unwrap();
        let mut i_transport = Session::deserialize(
            &session_storage,
            SessionBinding {
                initiator_authenticator: &Authenticator::Dh(initiator_psq_keys.pk),
                responder_ecdh_pk: &responder_x25519_keypair.pk,
                responder_pq_pk: Some(PQEncapsulationKey::MlKem(&mlkem_key)),
            },
        )
        .unwrap();

        r_transport
            .serialize(
                &mut session_storage,
                SessionBinding {
                    initiator_authenticator: &initiator_authenticator,
                    responder_ecdh_pk: &responder_x25519_keypair.pk,
                    responder_pq_pk: Some(PQEncapsulationKey::MlKem(&mlkem_key)),
                },
            )
            .unwrap();
        let mut r_transport = Session::deserialize(
            &session_storage,
            SessionBinding {
                initiator_authenticator: &initiator_authenticator,
                responder_ecdh_pk: &responder_x25519_keypair.pk,
                responder_pq_pk: Some(PQEncapsulationKey::MlKem(&mlkem_key)),
            },
        )
        .unwrap();

        let mut channel_i = i_transport.transport_channel().unwrap();
        let mut channel_r = r_transport.transport_channel().unwrap();

        assert_eq!(channel_i.identifier(), channel_r.identifier());

        let app_data_i = b"Derived session hey".as_slice();
        let app_data_r = b"Derived session ho".as_slice();

        let len_i = channel_i
            .write_message(app_data_i, &mut msg_channel)
            .unwrap();

        let (len_r_deserialized, len_r_payload) = channel_r
            .read_message(&msg_channel, &mut payload_buf_responder)
            .unwrap();

        // We read the same amount of data.
        assert_eq!(len_r_deserialized, len_i);
        assert_eq!(len_r_payload, app_data_i.len());
        assert_eq!(&payload_buf_responder[0..len_r_payload], app_data_i);

        let len_r = channel_r
            .write_message(app_data_r, &mut msg_channel)
            .unwrap();

        let (len_i_deserialized, len_i_payload) = channel_i
            .read_message(&msg_channel, &mut payload_buf_initiator)
            .unwrap();

        assert_eq!(len_r, len_i_deserialized);
        assert_eq!(app_data_r.len(), len_i_payload);
        assert_eq!(&payload_buf_initiator[0..len_i_payload], app_data_r);
    }
}
