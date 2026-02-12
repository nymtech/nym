use libcrux_chacha20poly1305::TAG_LEN;
use libcrux_psq::handshake::types::{DHKeyPair, DHPrivateKey, DHPublicKey};
use nym_crypto::hkdf::blake3::{derive_key_blake3, derive_key_blake3_multi_input};
use nym_kkt_ciphersuite::x25519::PUBLIC_KEY_LENGTH;
use rand09::{CryptoRng, RngCore};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::{
    ciphersuite::EncapsulationKey,
    context::{KKTContext, KKTRole},
    error::KKTError,
    frame::{KKT_SESSION_ID_LEN, KKTFrame},
    masked_byte::{MASKED_BYTE_LEN, MaskedByte},
};

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct KKTCarrier(Carrier);
impl KKTCarrier {
    fn read_message(&mut self) {
        // if we are reading a message and we have not read or written a message before
        // this means that we are a responder who is
        // reading the initiator's first handshake message
        if self.0.rx_counter() == 1 && self.0.tx_counter() == 1 {
            // in this case, we parse out the eph public key, the nonce, and the version hash
            // we decode to see if the hash is
        }
        // if we are reading a message and we have written a message before
        // this means that we are an initiator
        // who is reading the responder's handshake message
        if self.0.rx_counter() == 1 && self.0.tx_counter() > 1 {}
    }
    fn write_message(&mut self) {}

    // Generate carrier and encrypt first message

    // the first message would look like this
    // (outer_header || eph_pk ||

    // fn init<R>(
    //     rng: &mut R,
    //     responder_public_key: &DHPublicKey,
    //     payload: &[u8],
    //     header: Option<&[u8]>,
    // ) -> Result<(Self, Vec<u8>), KKTError>
    // where
    //     R: RngCore + CryptoRng,
    // {
    //     let ephemeral_keypair = DHKeyPair::new(rng);
    //     let shared_secret = ephemeral_keypair
    //         .sk()
    //         .diffie_hellman(responder_public_key)
    //         .map_err(|_| KKTError::X25519Error {
    //             info: "Key Derivation Error",
    //         })?;

    //     let carrier = Carrier::from_secret(&shared_secret.as_ref(), context);

    //     match header {
    //         Some(header) => {}
    //     }
    // }

    fn respond(responder_private_key: &DHPrivateKey, expects_header: bool) -> Self {
        todo!()
    }
}
// This is arbitrary
pub const MAX_PAYLOAD_LEN: usize = 1_000_000;
const CARRIER_KDF_INFO_TX: &str = "CARRIER_V1_KDF_RX";
const CARRIER_KDF_INFO_RX: &str = "CARRIER_V1_KDF_TX";
const KKT_CARRIER_CONTEXT: &[u8] = b"CARRIER_V1_KKT_V1_KDF";

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct Carrier {
    tx_key: [u8; 32],
    rx_key: [u8; 32],
    tx_counter: u64,
    rx_counter: u64,
}

pub enum CarrierRole {
    Initiator,
    Responder,
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
    fn init(tx_key: [u8; 32], rx_key: [u8; 32]) -> Self {
        Self {
            tx_key: tx_key,
            rx_key: rx_key,
            tx_counter: 1,
            rx_counter: 1,
        }
    }
    pub fn new<R>(
        rng: &mut R,
        remote_public_key: &DHPublicKey,
        context: &[u8],
    ) -> Result<(Self, DHPublicKey), KKTError>
    where
        R: RngCore + CryptoRng,
    {
        let ephemeral_keypair = DHKeyPair::new(rng);
        let shared_secret = ephemeral_keypair
            .sk()
            .diffie_hellman(remote_public_key)
            .map_err(|_| KKTError::X25519Error {
                info: "Key Derivation Error",
            })?;

        Ok((
            Self::from_secret_slice(shared_secret.as_ref(), context),
            ephemeral_keypair.pk,
        ))
    }

    pub(crate) fn tx_counter(&self) -> u64 {
        self.tx_counter
    }

    pub(crate) fn rx_counter(&self) -> u64 {
        self.rx_counter
    }

    pub fn new_kkt_responder(
        responder_keypair: &DHKeyPair,
        message: &[u8],
        supported_versions: &[u8],
    ) -> Result<(Carrier, KKTFrame, KKTContext), KKTError> {
        let mut initiator_public_key_bytes: [u8; PUBLIC_KEY_LENGTH] = [0; PUBLIC_KEY_LENGTH];
        initiator_public_key_bytes.clone_from_slice(&message[0..PUBLIC_KEY_LENGTH]);

        // check mask

        // todo: deal with this
        let masked_byte =
            MaskedByte::try_from(&message[PUBLIC_KEY_LENGTH..PUBLIC_KEY_LENGTH + MASKED_BYTE_LEN])
                .unwrap();

        let mut mask = Vec::from(&initiator_public_key_bytes);
        mask.extend_from_slice(responder_keypair.pk.as_ref());

        // todo: deal with this
        let byte = masked_byte.unmask(&mask).unwrap();

        // todo: derive version from byte

        if supported_versions.iter().find(|x| *x == &byte).is_some() {
            // now that the version is ok, we can try dh

            let initiator_public_key = DHPublicKey::from_bytes(&initiator_public_key_bytes);

            let shared_secret = responder_keypair
                .sk()
                .diffie_hellman(&initiator_public_key)
                .map_err(|_| KKTError::X25519Error {
                    info: "Key Derivation Error",
                })?;

            let mut context = Vec::from(masked_byte.as_slice());
            context.extend_from_slice(&KKT_CARRIER_CONTEXT);
            context.extend_from_slice(&initiator_public_key.as_ref());
            context.extend_from_slice(&responder_keypair.pk.as_ref());

            let mut carrier = Self::from_secret_slice(shared_secret.as_ref(), &context).flip_keys();

            let decrypted_message =
                carrier.decrypt(&message[PUBLIC_KEY_LENGTH + MASKED_BYTE_LEN..])?;
            let (frame, context) = KKTFrame::from_bytes(&decrypted_message)?;

            Ok((carrier, frame, context))
        } else {
            panic!("unsupported protocol version");
        }
    }
    pub fn new_kkt_initiator<R>(
        rng: &mut R,
        responder_public_key: &DHPublicKey,
        version_byte: u8,
        kkt_frame: &KKTFrame,
    ) -> Result<(Self, Vec<u8>), KKTError>
    where
        R: CryptoRng + RngCore,
    {
        let ephemeral_keypair = DHKeyPair::new(rng);
        let shared_secret = ephemeral_keypair
            .sk()
            .diffie_hellman(responder_public_key)
            .map_err(|_| KKTError::X25519Error {
                info: "Key Derivation Error",
            })?;

        let mut mask = Vec::from(ephemeral_keypair.pk.as_ref());
        mask.extend_from_slice(responder_public_key.as_ref());

        let masked_byte = MaskedByte::new(version_byte, &mask);

        let mut context = Vec::from(masked_byte.as_slice());
        context.extend_from_slice(&KKT_CARRIER_CONTEXT);
        context.extend_from_slice(&ephemeral_keypair.pk.as_ref());
        context.extend_from_slice(&responder_public_key.as_ref());

        let mut carrier = Self::from_secret_slice(shared_secret.as_ref(), &context);

        let mut full_kkt_message = Vec::from(ephemeral_keypair.pk.as_ref());
        full_kkt_message.extend_from_slice(masked_byte.as_slice());
        let encrypted_kkt_frame = carrier.encrypt(&kkt_frame.to_bytes())?;
        full_kkt_message.extend_from_slice(&encrypted_kkt_frame);

        Ok((carrier, full_kkt_message))
    }

    pub fn from_secret_slice(secret: &[u8], context: &[u8]) -> Self {
        let tx_key = derive_key_blake3(CARRIER_KDF_INFO_TX, secret, &context);
        let rx_key = derive_key_blake3(CARRIER_KDF_INFO_RX, secret, &context);
        Self::init(tx_key, rx_key)
    }

    pub fn from_secret(mut secret: [u8; 32], context: &[u8]) -> Self {
        let tx_key = derive_key_blake3(CARRIER_KDF_INFO_TX, secret.as_ref(), &context);
        let rx_key = derive_key_blake3(CARRIER_KDF_INFO_RX, secret.as_ref(), &context);
        secret.zeroize();
        Self::init(tx_key, rx_key)
    }

    fn flip_keys(self) -> Self {
        Self {
            tx_key: self.rx_key,
            rx_key: self.tx_key,
            tx_counter: self.rx_counter,
            rx_counter: self.tx_counter,
        }
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
        context::{KKTMode, KKTRole},
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
    use nym_kkt_ciphersuite::{Ciphersuite, HashFunction, KEM};

    #[test]
    fn test_e2e() {
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
                    Carrier::new_kkt_responder(&responder_x25519_keypair, &i_bytes, &[1]).unwrap();

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

                assert_eq!(i_obtained_key.encode(), r_kem_key_bytes);

                let test1 = b"test1: i>r #1";
                let ct1 = i_carrier.encrypt(test1).unwrap();
                let pt1 = r_carrier.decrypt(&ct1).unwrap();
                assert_eq!(pt1, test1);

                let test2 = b"test2: r>i #1";
                let ct2 = i_carrier.encrypt(test2).unwrap();
                let pt2 = r_carrier.decrypt(&ct2).unwrap();
                assert_eq!(pt2, test2);
                let test3 = b"test3: i>r #2";

                let ct3 = i_carrier.encrypt(test3).unwrap();
                let pt3 = r_carrier.decrypt(&ct3).unwrap();
                assert_eq!(pt3, test3);

                let test4 = b"test4: i>r #3";
                let ct4 = i_carrier.encrypt(test4).unwrap();
                let pt4 = r_carrier.decrypt(&ct4).unwrap();
                assert_eq!(pt4, test4);

                let test5 = b"test5: r>i #2";
                let ct5 = i_carrier.encrypt(test5).unwrap();
                let pt5 = r_carrier.decrypt(&ct5).unwrap();
                assert_eq!(pt5, test5);
            }
        }
    }
}
