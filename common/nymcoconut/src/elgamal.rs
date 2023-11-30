// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use core::ops::{Deref, Mul};
use std::convert::TryFrom;
use std::convert::TryInto;

use bls12_381::{G1Projective, Scalar};
use group::Curve;
use serde_derive::{Deserialize, Serialize};

use crate::error::{CoconutError, Result};
use crate::scheme::setup::Parameters;
use crate::traits::{Base58, Bytable};
use crate::utils::{try_deserialize_g1_projective, try_deserialize_scalar};
use crate::Attribute;

/// Type alias for the ephemeral key generated during ElGamal encryption
pub type EphemeralKey = Scalar;

/// Two G1 points representing ElGamal ciphertext
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct Ciphertext(pub(crate) G1Projective, pub(crate) G1Projective);

impl TryFrom<&[u8]> for Ciphertext {
    type Error = CoconutError;

    fn try_from(bytes: &[u8]) -> Result<Ciphertext> {
        if bytes.len() != 96 {
            return Err(CoconutError::Deserialization(format!(
                "Ciphertext must be exactly 96 bytes, got {}",
                bytes.len()
            )));
        }

        // safety: we just checked for the length so the unwraps are fine
        #[allow(clippy::unwrap_used)]
        let c1_bytes: &[u8; 48] = &bytes[..48].try_into().unwrap();
        #[allow(clippy::unwrap_used)]
        let c2_bytes: &[u8; 48] = &bytes[48..].try_into().unwrap();

        let c1 = try_deserialize_g1_projective(
            c1_bytes,
            CoconutError::Deserialization("Failed to deserialize compressed c1".to_string()),
        )?;
        let c2 = try_deserialize_g1_projective(
            c2_bytes,
            CoconutError::Deserialization("Failed to deserialize compressed c2".to_string()),
        )?;

        Ok(Ciphertext(c1, c2))
    }
}

impl Ciphertext {
    pub fn c1(&self) -> &G1Projective {
        &self.0
    }

    pub fn c2(&self) -> &G1Projective {
        &self.1
    }

    pub fn to_bytes(&self) -> [u8; 96] {
        let mut bytes = [0u8; 96];
        bytes[..48].copy_from_slice(&self.0.to_affine().to_compressed());
        bytes[48..].copy_from_slice(&self.1.to_affine().to_compressed());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Ciphertext> {
        Ciphertext::try_from(bytes)
    }
}

/// PrivateKey used in the ElGamal encryption scheme to recover the plaintext
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct PrivateKey(pub(crate) Scalar);

impl PrivateKey {
    /// Decrypt takes the ElGamal encryption of a message and returns a point on the G1 curve
    /// that represents original h^m.
    pub fn decrypt(&self, ciphertext: &Ciphertext) -> G1Projective {
        let (c1, c2) = &(ciphertext.0, ciphertext.1);

        // (gamma^k * h^m) / (g1^{d * k})   |   note: gamma = g1^d
        c2 - c1 * self.0
    }

    pub fn public_key(&self, params: &Parameters) -> PublicKey {
        PublicKey(params.gen1() * self.0)
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_bytes()
    }

    pub fn from_bytes(bytes: &[u8; 32]) -> Result<PrivateKey> {
        try_deserialize_scalar(
            bytes,
            CoconutError::Deserialization(
                "Failed to deserialize ElGamal private key - it was not in the canonical form"
                    .to_string(),
            ),
        )
        .map(PrivateKey)
    }
}

impl Bytable for PrivateKey {
    fn to_byte_vec(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }

    fn try_from_byte_slice(slice: &[u8]) -> Result<Self> {
        let received = slice.len();
        let Ok(arr) = slice.try_into() else {
            return Err(CoconutError::UnexpectedArrayLength {
                typ: "elgamal::PrivateKey".to_string(),
                received,
                expected: 32,
            });
        };

        PrivateKey::from_bytes(arr)
    }
}

impl Base58 for PrivateKey {}

// TODO: perhaps be more explicit and apart from gamma also store generator and group order?
/// PublicKey used in the ElGamal encryption scheme to produce the ciphertext
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct PublicKey(G1Projective);

impl PublicKey {
    /// Encrypt encrypts the given message in the form of h^m,
    /// where h is a point on the G1 curve using the given public key.
    /// The random k is returned alongside the encryption
    /// as it is required by the Coconut Scheme to create proofs of knowledge.
    pub fn encrypt(
        &self,
        params: &Parameters,
        h: &G1Projective,
        msg: &Scalar,
    ) -> (Ciphertext, EphemeralKey) {
        let k = params.random_scalar();
        // c1 = g1^k
        let c1 = params.gen1() * k;
        // c2 = gamma^k * h^m
        let c2 = self.0 * k + h * msg;

        (Ciphertext(c1, c2), k)
    }

    pub fn to_bytes(&self) -> [u8; 48] {
        self.0.to_affine().to_compressed()
    }

    pub fn from_bytes(bytes: &[u8; 48]) -> Result<PublicKey> {
        try_deserialize_g1_projective(
            bytes,
            CoconutError::Deserialization(
                "Failed to deserialize compressed ElGamal public key".to_string(),
            ),
        )
        .map(PublicKey)
    }
}

impl Bytable for PublicKey {
    fn to_byte_vec(&self) -> Vec<u8> {
        self.to_bytes().into()
    }

    fn try_from_byte_slice(slice: &[u8]) -> Result<Self> {
        let received = slice.len();
        let Ok(arr) = slice.try_into() else {
            return Err(CoconutError::UnexpectedArrayLength {
                typ: "elgamal::PublicKey".to_string(),
                received,
                expected: 48,
            });
        };

        PublicKey::from_bytes(arr)
    }
}

impl TryFrom<&[u8]> for PublicKey {
    type Error = CoconutError;

    fn try_from(slice: &[u8]) -> Result<PublicKey> {
        PublicKey::try_from_byte_slice(slice)
    }
}

impl Base58 for PublicKey {}

impl Deref for PublicKey {
    type Target = G1Projective;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, 'b> Mul<&'b Scalar> for &'a PublicKey {
    type Output = G1Projective;

    fn mul(self, rhs: &'b Scalar) -> Self::Output {
        self.0 * rhs
    }
}

#[derive(Serialize, Deserialize)]
/// A convenient wrapper for both keys of the ElGamal keypair
pub struct ElGamalKeyPair {
    private_key: PrivateKey,
    public_key: PublicKey,
}

impl ElGamalKeyPair {
    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }

    pub fn private_key(&self) -> &PrivateKey {
        &self.private_key
    }
}

/// Generate a fresh ElGamal keypair using the group generator specified by the provided [Parameters]
pub fn elgamal_keygen(params: &Parameters) -> ElGamalKeyPair {
    let private_key = params.random_scalar();
    let gamma = params.gen1() * private_key;

    ElGamalKeyPair {
        private_key: PrivateKey(private_key),
        public_key: PublicKey(gamma),
    }
}

pub fn compute_attribute_encryption(
    params: &Parameters,
    private_attributes: &[&Attribute],
    pub_key: &PublicKey,
    commitment_hash: G1Projective,
) -> (Vec<Ciphertext>, Vec<EphemeralKey>) {
    private_attributes
        .iter()
        .map(|m| pub_key.encrypt(params, &commitment_hash, m))
        .unzip()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keygen() {
        let params = Parameters::default();
        let keypair = super::elgamal_keygen(&params);

        let expected = params.gen1() * keypair.private_key.0;
        let gamma = keypair.public_key.0;
        assert_eq!(
            expected, gamma,
            "Public key, gamma, should be equal to g1^d, where d is the private key"
        );
    }

    #[test]
    fn encryption() {
        let params = Parameters::default();
        let keypair = super::elgamal_keygen(&params);

        let r = params.random_scalar();
        let h = params.gen1() * r;
        let m = params.random_scalar();

        let (ciphertext, ephemeral_key) = keypair.public_key.encrypt(&params, &h, &m);

        let expected_c1 = params.gen1() * ephemeral_key;
        assert_eq!(expected_c1, ciphertext.0, "c1 should be equal to g1^k");

        let expected_c2 = keypair.public_key.0 * ephemeral_key + h * m;
        assert_eq!(
            expected_c2, ciphertext.1,
            "c2 should be equal to gamma^k * h^m"
        );
    }

    #[test]
    fn decryption() {
        let params = Parameters::default();
        let keypair = super::elgamal_keygen(&params);

        let r = params.random_scalar();
        let h = params.gen1() * r;
        let m = params.random_scalar();

        let (ciphertext, _) = keypair.public_key.encrypt(&params, &h, &m);
        let dec = keypair.private_key.decrypt(&ciphertext);

        let expected = h * m;
        assert_eq!(
            expected, dec,
            "after ElGamal decryption, original h^m should be obtained"
        );
    }

    #[test]
    fn private_key_bytes_roundtrip() {
        let params = Parameters::default();
        let private_key = PrivateKey(params.random_scalar());
        let bytes = private_key.to_bytes();

        // also make sure it is equivalent to the internal scalar's bytes
        assert_eq!(private_key.0.to_bytes(), bytes);
        assert_eq!(private_key, PrivateKey::from_bytes(&bytes).unwrap())
    }

    #[test]
    fn public_key_bytes_roundtrip() {
        let params = Parameters::default();
        let r = params.random_scalar();
        let public_key = PublicKey(params.gen1() * r);
        let bytes = public_key.to_bytes();

        // also make sure it is equivalent to the internal g1 compressed bytes
        assert_eq!(public_key.0.to_affine().to_compressed(), bytes);
        assert_eq!(public_key, PublicKey::from_bytes(&bytes).unwrap())
    }

    #[test]
    fn ciphertext_bytes_roundtrip() {
        let params = Parameters::default();
        let r = params.random_scalar();
        let s = params.random_scalar();
        let ciphertext = Ciphertext(params.gen1() * r, params.gen1() * s);
        let bytes = ciphertext.to_bytes();

        // also make sure it is equivalent to the internal g1 compressed bytes concatenated
        let expected_bytes = [
            ciphertext.0.to_affine().to_compressed(),
            ciphertext.1.to_affine().to_compressed(),
        ]
        .concat();
        assert_eq!(expected_bytes, bytes);
        assert_eq!(ciphertext, Ciphertext::try_from(&bytes[..]).unwrap())
    }
}
