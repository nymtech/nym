use libcrux_chacha20poly1305::TAG_LEN;
use libcrux_psq::handshake::types::{DHKeyPair, DHPublicKey};
use nym_crypto::hkdf::blake3::derive_key_blake3;
use rand09::{CryptoRng, RngCore};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use crate::error::KKTError;

// This is arbitrary
pub const MAX_PAYLOAD_LEN: usize = 1_000_000;
const CARRIER_KDF_INFO_TX: &str = "CARRIER_V1_KDF_TX";
const CARRIER_KDF_INFO_RX: &str = "CARRIER_V1_KDF_RX";
const CARRIER_KKT_AAD: &[u8] = b"kkt-carrier-v1";

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
            tx_key,
            rx_key,
            tx_counter: 1,
            rx_counter: 1,
        }
    }

    pub fn new<R>(
        rng: &mut R,
        remote_public_key: &DHPublicKey,
        context: &[u8],
        is_initiator: bool,
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
            Self::from_secret_slice(shared_secret.as_ref(), context, is_initiator),
            ephemeral_keypair.pk,
        ))
    }

    pub(crate) fn from_secret_slice(secret: &[u8], context: &[u8], is_initiator: bool) -> Self {
        let (tx_key, rx_key) = if is_initiator {
            (
                derive_key_blake3(CARRIER_KDF_INFO_TX, secret, context),
                derive_key_blake3(CARRIER_KDF_INFO_RX, secret, context),
            )
        } else {
            (
                derive_key_blake3(CARRIER_KDF_INFO_RX, secret, context),
                derive_key_blake3(CARRIER_KDF_INFO_TX, secret, context),
            )
        };

        Self::init(tx_key, rx_key)
    }

    pub fn from_secret(secret: [u8; 32], context: &[u8], is_initiator: bool) -> Self {
        Self::from_secret_slice(Zeroizing::new(secret).as_slice(), context, is_initiator)
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
            CARRIER_KKT_AAD,
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
            CARRIER_KKT_AAD,
            &as_nonce_bytes(self.rx_counter),
        )?;

        increment_nonce(&mut self.rx_counter)?;

        Ok(output_buffer)
    }
}

#[cfg(test)]
mod tests {
    use crate::{carrier::Carrier, key_utils::generate_lp_keypair_x25519};
    use rand09::RngCore;

    #[test]
    fn test_e2e() {
        let mut rng = rand09::rng();

        // generate responder x25519 keys
        let r_x25519 = generate_lp_keypair_x25519(&mut rng);

        let mut context: [u8; 32] = [0u8; 32];
        rng.fill_bytes(&mut context);

        let ephemeral_keypair = generate_lp_keypair_x25519(&mut rng);

        let i_shared_secret = ephemeral_keypair.sk().diffie_hellman(&r_x25519.pk).unwrap();

        let r_shared_secret = r_x25519.sk().diffie_hellman(&ephemeral_keypair.pk).unwrap();

        let mut i_carrier = Carrier::from_secret_slice(i_shared_secret.as_ref(), &context, true);
        let mut r_carrier = Carrier::from_secret_slice(r_shared_secret.as_ref(), &context, false);

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
