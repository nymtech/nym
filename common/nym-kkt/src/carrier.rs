use libcrux_chacha20poly1305::TAG_LEN;
use libcrux_psq::handshake::types::DHPublicKey;
use nym_crypto::hkdf::blake3::{derive_key_blake3, derive_key_blake3_multi_input};
use zeroize::Zeroize;

use crate::{
    ciphersuite::EncapsulationKey, context::KKTRole, encryption::KKTSessionSecret, error::KKTError,
    frame::KKT_SESSION_ID_LEN,
};

const MAX_PAYLOAD_LEN: usize = 65535;

pub struct Carrier {
    tx_key: [u8; 32],
    rx_key: [u8; 32],
    tx_counter: u64,
    rx_counter: u64,
}

fn increment_nonce(nonce: &mut u64) -> Result<(), KKTError> {
    match nonce.checked_add(1) {
        Some(incremented_nonce) => {
            *nonce = incremented_nonce;
            Ok(())
        }
        None => Err(KKTError::AEADError {
            info: "Nonce maxed out.",
        }),
    }
}

fn as_nonce_bytes(nonce: u64) -> [u8; 12] {
    let mut bytes = [0u8; 12];
    let nonce_bytes = nonce.to_le_bytes();
    bytes[4..].clone_from_slice(&nonce_bytes);
    bytes
}

impl Carrier {
    pub fn from_session_secret(
        mut session_secret: KKTSessionSecret,
        role: KKTRole,
        kkt_session_id: &[u8; KKT_SESSION_ID_LEN],
        responder_public_key: &DHPublicKey,
        responder_encapsulation_key: &EncapsulationKey,
    ) -> Result<Self, KKTError> {
        let mut salt = derive_key_blake3_multi_input(
            "nym-kkt-carrier-kdf-main",
            &[
                responder_encapsulation_key.encode().as_ref(),
                responder_public_key.as_ref(),
            ],
            kkt_session_id,
        );

        let k1 = derive_key_blake3(
            "nym-kkt-carrier-kdf-initiator",
            session_secret.as_bytes().as_ref(),
            &salt,
        );
        let k2 = derive_key_blake3(
            "nym-kkt-carrier-kdf-responder",
            session_secret.as_bytes().as_ref(),
            &salt,
        );

        salt.zeroize();
        session_secret.zeroize();

        Ok(match role {
            KKTRole::Initiator | KKTRole::AnonymousInitiator => Self {
                tx_key: k1,
                rx_key: k2,
                tx_counter: 1,
                rx_counter: 1,
            },
            KKTRole::Responder => Self {
                tx_key: k2,
                rx_key: k1,
                tx_counter: 1,
                rx_counter: 1,
            },
        })
    }
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<Vec<u8>, KKTError> {
        if plaintext.len() > MAX_PAYLOAD_LEN {
            return Err(KKTError::AEADError {
                info: "Plaintext too large",
            });
        }
        let mut output_buffer = vec![0; plaintext.len() + TAG_LEN];
        libcrux_chacha20poly1305::encrypt(
            &self.tx_key,
            plaintext,
            &mut output_buffer,
            b"kkt-carrier-v1",
            &as_nonce_bytes(self.tx_counter),
        )?;

        increment_nonce(&mut self.tx_counter)?;

        Ok(output_buffer)
    }
    pub fn decrypt(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>, KKTError> {
        if ciphertext.len() > MAX_PAYLOAD_LEN + TAG_LEN {
            return Err(KKTError::AEADError {
                info: "Ciphertext too large",
            });
        }
        let mut output_buffer = vec![0; ciphertext.len() - TAG_LEN];
        libcrux_chacha20poly1305::decrypt(
            &self.rx_key,
            &mut output_buffer,
            ciphertext,
            b"kkt-carrier-v1",
            &as_nonce_bytes(self.rx_counter),
        )?;

        increment_nonce(&mut self.rx_counter)?;

        Ok(output_buffer)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        KKT_RESPONSE_AAD,
        carrier::Carrier,
        ciphersuite::EncapsulationKey,
        encryption::*,
        key_utils::{
            generate_keypair_ed25519, generate_keypair_libcrux, generate_keypair_mceliece,
            generate_keypair_mlkem, generate_keypair_x25519, hash_encapsulation_key,
        },
        session::{
            anonymous_initiator_process, initiator_ingest_response, responder_ingest_message,
            responder_process,
        },
    };
    use nym_kkt_ciphersuite::{Ciphersuite, HashFunction, KEM};

    #[test]
    fn test_e2e() {
        let mut rng = rand09::rng();
        // generate ed25519 keys

        let _initiator_ed25519_keypair = generate_keypair_ed25519(&mut rng, Some(0));
        let responder_ed25519_keypair = generate_keypair_ed25519(&mut rng, Some(1));

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

                let (mut i_context, i_frame) =
                    anonymous_initiator_process(&mut rng, ciphersuite).unwrap();

                // encryption - initiator frame

                let (i_session_secret, i_bytes) =
                    encrypt_initial_kkt_frame(&mut rng, &responder_x25519_keypair.pk, &i_frame)
                        .unwrap();

                // decryption - initiator frame

                let (r_session_secret, i_frame_r, i_context_r) =
                    decrypt_initial_kkt_frame(responder_x25519_keypair.sk(), &i_bytes).unwrap();

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

                assert_eq!(i_obtained_key.encode(), r_kem_key_bytes);

                let mut initiator_carrier = Carrier::from_session_secret(
                    i_session_secret,
                    i_context.role(),
                    &i_frame.session_id(),
                    &responder_x25519_keypair.pk,
                    &i_obtained_key,
                )
                .unwrap();

                let mut responder_carrier = Carrier::from_session_secret(
                    r_session_secret,
                    r_context.role(),
                    &r_frame.session_id(),
                    &responder_x25519_keypair.pk,
                    &responder_kem_public_key,
                )
                .unwrap();

                let test1 = b"test1: i>r #1";
                let ct1 = initiator_carrier.encrypt(test1).unwrap();
                let pt1 = responder_carrier.decrypt(&ct1).unwrap();
                assert_eq!(pt1, test1);

                let test2 = b"test2: r>i #1";
                let ct2 = initiator_carrier.encrypt(test2).unwrap();
                let pt2 = responder_carrier.decrypt(&ct2).unwrap();
                assert_eq!(pt2, test2);
                let test3 = b"test3: i>r #2";

                let ct3 = initiator_carrier.encrypt(test3).unwrap();
                let pt3 = responder_carrier.decrypt(&ct3).unwrap();
                assert_eq!(pt3, test3);

                let test4 = b"test4: i>r #3";
                let ct4 = initiator_carrier.encrypt(test4).unwrap();
                let pt4 = responder_carrier.decrypt(&ct4).unwrap();
                assert_eq!(pt4, test4);

                let test5 = b"test5: r>i #2";
                let ct5 = initiator_carrier.encrypt(test5).unwrap();
                let pt5 = responder_carrier.decrypt(&ct5).unwrap();
                assert_eq!(pt5, test5);
            }
        }
    }
}
