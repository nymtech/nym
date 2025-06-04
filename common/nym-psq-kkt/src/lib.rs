// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod error;
pub mod kkt;
pub mod psq;

#[cfg(test)]
mod test {
    use libcrux_psq::impls::MlKem768;
    use nym_crypto::asymmetric::ed25519;
    use rand::prelude::*;

    use crate::{
        kkt::{KKTInitiator, KKTResponder, KKT_REQ_LEN, KKT_RES_LEN_MLKEM768, KKT_TAG_LEN},
        psq::{PSQInitiator, PSQResponder},
    };

    #[test]
    fn test_kkt_psq_e2e() {
        let mut rng = rand::rng();

        // generate ed25519 keys
        let mut secret_initiator: [u8; 32] = [0u8; 32];
        rng.fill_bytes(&mut secret_initiator);
        let initiator_ed25519_keypair = ed25519::KeyPair::from_secret(secret_initiator, 0);

        let mut secret_responder: [u8; 32] = [0u8; 32];
        rng.fill_bytes(&mut secret_responder);
        let responder_ed25519_keypair = ed25519::KeyPair::from_secret(secret_responder, 1);

        // generate kem keypair
        let (responder_kem_private_key, responder_kem_public_key) =
            libcrux_kem::key_gen(libcrux_kem::Algorithm::MlKem768, &mut rng).unwrap();

        // initialize parties
        let kkt_initiator: KKTInitiator<MlKem768> =
            KKTInitiator::init(initiator_ed25519_keypair.private_key());

        let kkt_responder: KKTResponder<MlKem768> = KKTResponder::init(
            responder_ed25519_keypair.private_key(),
            &responder_kem_public_key,
        );

        // create buffers
        let mut request_buffer: [u8; KKT_REQ_LEN] = [0u8; KKT_REQ_LEN];
        let mut response_buffer: [u8; KKT_RES_LEN_MLKEM768] = [0u8; KKT_RES_LEN_MLKEM768];
        let mut tag_buffer: [u8; KKT_TAG_LEN] = [0u8; KKT_TAG_LEN];

        // generate request
        kkt_initiator.request_kem_pk(&mut request_buffer, &mut tag_buffer);

        // ingest request, generate response
        kkt_responder
            .respond_kem_pk(
                &mut response_buffer,
                initiator_ed25519_keypair.public_key(),
                &request_buffer,
            )
            .unwrap();

        // ingest response
        let received_responder_key = kkt_initiator
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

        let mut psq_initiator: PSQInitiator<MlKem768> =
            PSQInitiator::init(&initiator_ed25519_keypair);
        let psq_responder: PSQResponder<MlKem768> =
            PSQResponder::init(&responder_kem_private_key, &responder_kem_public_key);

        let initiator_psq_msg = psq_initiator
            .initiator_message(&mut rng, &received_responder_key)
            .unwrap();

        let (responder_psk, responder_psq_msg) = psq_responder
            .responder_msg(initiator_ed25519_keypair.public_key(), &initiator_psq_msg)
            .unwrap();

        let initiator_psk = psq_initiator.finalize(&responder_psq_msg).unwrap();

        assert_eq!(initiator_psk, responder_psk);
    }
}
