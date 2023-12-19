// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use core::borrow::Borrow;
use core::iter::Sum;
use core::ops::{Add, Mul};
use std::convert::TryFrom;
use std::convert::TryInto;

use bls12_381::{G1Projective, G2Projective, Scalar};
use group::Curve;
use nym_pemstore::traits::{PemStorableKey, PemStorableKeyPair};
use serde_derive::{Deserialize, Serialize};

use crate::error::{CoconutError, Result};
use crate::scheme::aggregation::aggregate_verification_keys;
use crate::scheme::setup::Parameters;
use crate::scheme::SignerIndex;
use crate::traits::Bytable;
use crate::utils::{
    try_deserialize_g1_projective, try_deserialize_g2_projective, try_deserialize_scalar,
    try_deserialize_scalar_vec, Polynomial,
};
use crate::Base58;

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq, Eq, Clone))]
#[cfg_attr(
    feature = "key-zeroize",
    derive(zeroize::Zeroize, zeroize::ZeroizeOnDrop)
)]
pub struct SecretKey {
    pub(crate) x: Scalar,
    pub(crate) ys: Vec<Scalar>,
}

impl PemStorableKey for SecretKey {
    type Error = CoconutError;

    fn pem_type() -> &'static str {
        "COCONUT SECRET KEY"
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> std::result::Result<Self, Self::Error> {
        Self::from_bytes(bytes)
    }
}

impl TryFrom<&[u8]> for SecretKey {
    type Error = CoconutError;

    fn try_from(bytes: &[u8]) -> Result<SecretKey> {
        // There should be x and at least one y
        if bytes.len() < 32 * 2 + 8 || (bytes.len() - 8) % 32 != 0 {
            return Err(CoconutError::DeserializationInvalidLength {
                actual: bytes.len(),
                modulus_target: bytes.len() - 8,
                target: 32 * 2 + 8,
                modulus: 32,
                object: "secret key".to_string(),
            });
        }

        // this conversion will not fail as we are taking the same length of data
        #[allow(clippy::unwrap_used)]
        let x_bytes: [u8; 32] = bytes[..32].try_into().unwrap();
        #[allow(clippy::unwrap_used)]
        let ys_len = u64::from_le_bytes(bytes[32..40].try_into().unwrap());
        let actual_ys_len = (bytes.len() - 40) / 32;

        if ys_len as usize != actual_ys_len {
            return Err(CoconutError::Deserialization(format!(
                "Tried to deserialize secret key with inconsistent ys len (expected {ys_len}, got {actual_ys_len})"
            )));
        }

        let x = try_deserialize_scalar(
            &x_bytes,
            CoconutError::Deserialization("Failed to deserialize secret key scalar".to_string()),
        )?;
        let ys = try_deserialize_scalar_vec(
            ys_len,
            &bytes[40..],
            CoconutError::Deserialization("Failed to deserialize secret key scalars".to_string()),
        )?;

        Ok(SecretKey { x, ys })
    }
}

impl SecretKey {
    /// Following a (distributed) key generation process, scalar values can be obtained
    /// outside of the normal key generation process.
    pub fn create_from_raw(x: Scalar, ys: Vec<Scalar>) -> Self {
        Self { x, ys }
    }

    pub fn into_raw(&self) -> (Scalar, Vec<Scalar>) {
        (self.x, self.ys.clone())
    }

    pub fn size(&self) -> usize {
        self.ys.len()
    }

    /// Derive verification key using this secret key.
    pub fn verification_key(&self, params: &Parameters) -> VerificationKey {
        let g1 = params.gen1();
        let g2 = params.gen2();
        VerificationKey {
            alpha: g2 * self.x,
            beta_g1: self.ys.iter().map(|y| g1 * y).collect(),
            beta_g2: self.ys.iter().map(|y| g2 * y).collect(),
        }
    }

    // x || ys.len() || ys
    pub fn to_bytes(&self) -> Vec<u8> {
        let ys_len = self.ys.len();
        let mut bytes = Vec::with_capacity(8 + (ys_len + 1) * 32);

        bytes.extend_from_slice(&self.x.to_bytes());
        bytes.extend_from_slice(&ys_len.to_le_bytes());
        for y in self.ys.iter() {
            bytes.extend_from_slice(&y.to_bytes())
        }
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<SecretKey> {
        SecretKey::try_from(bytes)
    }
}

impl Bytable for SecretKey {
    fn to_byte_vec(&self) -> Vec<u8> {
        self.to_bytes()
    }

    fn try_from_byte_slice(slice: &[u8]) -> Result<Self> {
        SecretKey::try_from(slice)
    }
}

impl Base58 for SecretKey {}

// TODO: perhaps change points to affine representation
// to make verification slightly more efficient?
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(
    feature = "key-zeroize",
    derive(zeroize::Zeroize, zeroize::ZeroizeOnDrop)
)]
pub struct VerificationKey {
    // TODO add gen2 as per the paper or imply it from the fact library is using bls381?
    pub(crate) alpha: G2Projective,
    pub(crate) beta_g1: Vec<G1Projective>,
    pub(crate) beta_g2: Vec<G2Projective>,
}

impl PemStorableKey for VerificationKey {
    type Error = CoconutError;

    fn pem_type() -> &'static str {
        "COCONUT VERIFICATION KEY"
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> std::result::Result<Self, Self::Error> {
        Self::from_bytes(bytes)
    }
}

impl TryFrom<&[u8]> for VerificationKey {
    type Error = CoconutError;

    fn try_from(bytes: &[u8]) -> Result<VerificationKey> {
        // There should be at least alpha, one betaG1 and one betaG2 and their length
        if bytes.len() < 96 * 2 + 48 + 8 || (bytes.len() - 8 - 96) % (96 + 48) != 0 {
            return Err(CoconutError::DeserializationInvalidLength {
                actual: bytes.len(),
                modulus_target: bytes.len() - 8 - 96,
                target: 96 * 2 + 48 + 8,
                modulus: 96 + 48,
                object: "verification key".to_string(),
            });
        }

        // this conversion will not fail as we are taking the same length of data
        #[allow(clippy::unwrap_used)]
        let alpha_bytes: [u8; 96] = bytes[..96].try_into().unwrap();
        #[allow(clippy::unwrap_used)]
        let betas_len = u64::from_le_bytes(bytes[96..104].try_into().unwrap());

        let actual_betas_len = (bytes.len() - 104) / (96 + 48);

        if betas_len as usize != actual_betas_len {
            return Err(
                CoconutError::Deserialization(
                    format!("Tried to deserialize verification key with inconsistent betas len (expected {betas_len}, got {actual_betas_len})"
                    )));
        }

        let alpha = try_deserialize_g2_projective(
            &alpha_bytes,
            CoconutError::Deserialization(
                "Failed to deserialize verification key G2 point (alpha)".to_string(),
            ),
        )?;

        let mut beta_g1 = Vec::with_capacity(betas_len as usize);
        let mut beta_g1_end: u64 = 0;
        for i in 0..betas_len {
            let start = (104 + i * 48) as usize;
            let end = start + 48;
            // we're using a constant 48 byte offset (which is the size of G1 compressed) so unwrap is fine
            #[allow(clippy::unwrap_used)]
            let beta_i_bytes = bytes[start..end].try_into().unwrap();
            let beta_i = try_deserialize_g1_projective(
                &beta_i_bytes,
                CoconutError::Deserialization(
                    "Failed to deserialize verification key G1 point (beta)".to_string(),
                ),
            )?;

            beta_g1_end = end as u64;
            beta_g1.push(beta_i)
        }

        let mut beta_g2 = Vec::with_capacity(betas_len as usize);
        for i in 0..betas_len {
            let start = (beta_g1_end + i * 96) as usize;
            let end = start + 96;
            // we're using a constant 96 byte offset (which is the size of G2 compressed) so unwrap is fine
            #[allow(clippy::unwrap_used)]
            let beta_i_bytes = bytes[start..end].try_into().unwrap();
            let beta_i = try_deserialize_g2_projective(
                &beta_i_bytes,
                CoconutError::Deserialization(
                    "Failed to deserialize verification key G2 point (beta)".to_string(),
                ),
            )?;

            beta_g2.push(beta_i)
        }

        Ok(VerificationKey {
            alpha,
            beta_g1,
            beta_g2,
        })
    }
}

impl<'b> Add<&'b VerificationKey> for VerificationKey {
    type Output = VerificationKey;

    #[inline]
    fn add(self, rhs: &'b VerificationKey) -> VerificationKey {
        // If you're trying to add two keys together that were created
        // for different number of attributes, just panic as it's a
        // nonsense operation.
        assert_eq!(
            self.beta_g1.len(),
            rhs.beta_g1.len(),
            "trying to add verification keys generated for different number of attributes [G1]"
        );

        assert_eq!(
            self.beta_g2.len(),
            rhs.beta_g2.len(),
            "trying to add verification keys generated for different number of attributes [G2]"
        );

        assert_eq!(
            self.beta_g1.len(),
            self.beta_g2.len(),
            "this key is incorrect - the number of elements G1 and G2 does not match"
        );

        assert_eq!(
            rhs.beta_g1.len(),
            rhs.beta_g2.len(),
            "they key you want to add is incorrect - the number of elements G1 and G2 does not match"
        );

        VerificationKey {
            alpha: self.alpha + rhs.alpha,
            beta_g1: self
                .beta_g1
                .iter()
                .zip(rhs.beta_g1.iter())
                .map(|(self_beta_g1, rhs_beta_g1)| self_beta_g1 + rhs_beta_g1)
                .collect(),
            beta_g2: self
                .beta_g2
                .iter()
                .zip(rhs.beta_g2.iter())
                .map(|(self_beta_g2, rhs_beta_g2)| self_beta_g2 + rhs_beta_g2)
                .collect(),
        }
    }
}

impl<'a> Mul<Scalar> for &'a VerificationKey {
    type Output = VerificationKey;

    #[inline]
    fn mul(self, rhs: Scalar) -> Self::Output {
        VerificationKey {
            alpha: self.alpha * rhs,
            beta_g1: self.beta_g1.iter().map(|b_i| b_i * rhs).collect(),
            beta_g2: self.beta_g2.iter().map(|b_i| b_i * rhs).collect(),
        }
    }
}

impl<T> Sum<T> for VerificationKey
where
    T: Borrow<VerificationKey>,
{
    #[inline]
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = T>,
    {
        let mut peekable = iter.peekable();
        let head_attributes = match peekable.peek() {
            Some(head) => head.borrow().beta_g2.len(),
            None => {
                // TODO: this is a really weird edge case. You're trying to sum an EMPTY iterator
                // of VerificationKey. So should it panic here or just return some nonsense value?
                return VerificationKey::identity(0);
            }
        };

        peekable.fold(VerificationKey::identity(head_attributes), |acc, item| {
            acc + item.borrow()
        })
    }
}

impl VerificationKey {
    /// Create a (kinda) identity verification key using specified
    /// number of 'beta' elements
    pub(crate) fn identity(beta_size: usize) -> Self {
        VerificationKey {
            alpha: G2Projective::identity(),
            beta_g1: vec![G1Projective::identity(); beta_size],
            beta_g2: vec![G2Projective::identity(); beta_size],
        }
    }

    pub fn aggregate(sigs: &[Self], indices: Option<&[SignerIndex]>) -> Result<Self> {
        aggregate_verification_keys(sigs, indices)
    }

    pub fn alpha(&self) -> &G2Projective {
        &self.alpha
    }

    pub fn beta_g1(&self) -> &Vec<G1Projective> {
        &self.beta_g1
    }

    pub fn beta_g2(&self) -> &Vec<G2Projective> {
        &self.beta_g2
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let beta_g1_len = self.beta_g1.len();
        let beta_g2_len = self.beta_g2.len();
        let mut bytes = Vec::with_capacity(96 + 8 + beta_g1_len * 48 + beta_g2_len * 96);

        bytes.extend_from_slice(&self.alpha.to_affine().to_compressed());

        bytes.extend_from_slice(&beta_g1_len.to_le_bytes());

        for beta_g1 in self.beta_g1.iter() {
            bytes.extend_from_slice(&beta_g1.to_affine().to_compressed())
        }

        for beta_g2 in self.beta_g2.iter() {
            bytes.extend_from_slice(&beta_g2.to_affine().to_compressed())
        }

        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<VerificationKey> {
        VerificationKey::try_from(bytes)
    }
}

impl Bytable for VerificationKey {
    fn to_byte_vec(&self) -> Vec<u8> {
        self.to_bytes()
    }

    fn try_from_byte_slice(slice: &[u8]) -> Result<Self> {
        VerificationKey::try_from(slice)
    }
}

impl Base58 for VerificationKey {}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq, Eq, Clone))]
#[cfg_attr(
    feature = "key-zeroize",
    derive(zeroize::Zeroize, zeroize::ZeroizeOnDrop)
)]
pub struct KeyPair {
    secret_key: SecretKey,
    verification_key: VerificationKey,

    /// Optional index value specifying polynomial point used during threshold key generation.
    pub index: Option<SignerIndex>,
}

impl PemStorableKeyPair for KeyPair {
    type PrivatePemKey = SecretKey;
    type PublicPemKey = VerificationKey;

    fn private_key(&self) -> &Self::PrivatePemKey {
        &self.secret_key
    }

    fn public_key(&self) -> &Self::PublicPemKey {
        &self.verification_key
    }

    fn from_keys(secret_key: Self::PrivatePemKey, verification_key: Self::PublicPemKey) -> Self {
        Self::from_keys(secret_key, verification_key)
    }
}

impl KeyPair {
    const MARKER_BYTES: &'static [u8] = b"coconutkeypair";

    pub fn from_keys(secret_key: SecretKey, verification_key: VerificationKey) -> Self {
        Self {
            secret_key,
            verification_key,
            index: None,
        }
    }

    pub fn secret_key(&self) -> &SecretKey {
        &self.secret_key
    }

    pub fn verification_key(&self) -> &VerificationKey {
        &self.verification_key
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        // Schema is coconutkeypair[14]|secret_key_len[8]|secret_key[secret_key_len]|verification_key_len[8]|verification_key[verification_key_len]|signer_index[8] - optional
        self.to_byte_vec()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        KeyPair::try_from_byte_slice(bytes)
    }
}

impl Bytable for KeyPair {
    fn to_byte_vec(&self) -> Vec<u8> {
        // Schema is coconutkeypair[14]|secret_key_len[8]|secret_key[secret_key_len]|verification_key_len[8]|verification_key[verification_key_len]|signer_index[8] - optional
        let mut byts = vec![];
        let secret_key_bytes = self.secret_key.to_bytes();
        let secret_key_len = (secret_key_bytes.len() as u64).to_le_bytes();
        let verification_key_bytes = self.verification_key.to_bytes();
        let verification_key_len = (verification_key_bytes.len() as u64).to_le_bytes();
        byts.extend_from_slice(Self::MARKER_BYTES);
        byts.extend_from_slice(&secret_key_len);
        byts.extend_from_slice(&secret_key_bytes);
        byts.extend_from_slice(&verification_key_len);
        byts.extend_from_slice(&verification_key_bytes);
        if let Some(index) = self.index {
            byts.extend_from_slice(&index.to_le_bytes())
        }
        byts
    }

    fn try_from_byte_slice(slice: &[u8]) -> Result<Self> {
        KeyPair::try_from(slice)
    }
}

impl Base58 for KeyPair {}

impl TryFrom<&[u8]> for KeyPair {
    type Error = CoconutError;

    fn try_from(bytes: &[u8]) -> Result<KeyPair> {
        let header_len = Self::MARKER_BYTES.len();

        // we must be able to at the very least read the length of secret key which is past the header
        // and is 8 bytes long
        if bytes.len() < header_len + 8 {
            return Err(CoconutError::DeserializationMinLength {
                min: header_len + 8,
                actual: bytes.len(),
            });
        }

        // safety: we made bound check and we're using constant offest
        #[allow(clippy::unwrap_used)]
        let secret_key_len =
            u64::from_le_bytes(bytes[header_len..header_len + 8].try_into().unwrap()) as usize;
        let secret_key_start = header_len + 8;

        let secret_key =
            SecretKey::try_from(&bytes[secret_key_start..secret_key_start + secret_key_len])?;

        // we must be able to read the length of verification key
        if bytes.len() < secret_key_start + secret_key_len + 8 {
            return Err(CoconutError::DeserializationMinLength {
                min: secret_key_start + secret_key_len + 8,
                actual: bytes.len(),
            });
        }

        // safety: we made bound check
        #[allow(clippy::unwrap_used)]
        let verification_key_len = u64::from_le_bytes(
            bytes[secret_key_start + secret_key_len..secret_key_start + secret_key_len + 8]
                .try_into()
                .unwrap(),
        ) as usize;
        let verification_key_start = secret_key_start + secret_key_len + 8;

        let verification_key = VerificationKey::try_from(
            &bytes[verification_key_start..verification_key_start + verification_key_len],
        )?;
        let consumed_bytes = verification_key_start + verification_key_len;
        let index = if consumed_bytes < bytes.len() && [consumed_bytes..].len() == 8 {
            #[allow(clippy::unwrap_used)]
            Some(u64::from_le_bytes(
                bytes[consumed_bytes..consumed_bytes + 8]
                    .try_into()
                    .unwrap(),
            ))
        } else {
            None
        };
        Ok(KeyPair {
            secret_key,
            verification_key,
            index,
        })
    }
}

/// Generate a single Coconut keypair ((x, y0, y1...), (g2^x, g2^y0, ...)).
/// It is not suitable for threshold credentials as all subsequent calls to `keygen` generate keys
/// that are independent of each other.
#[cfg(test)]
pub fn keygen(params: &Parameters) -> KeyPair {
    let attributes = params.gen_hs().len();

    let x = params.random_scalar();
    let ys = params.n_random_scalars(attributes);

    let secret_key = SecretKey { x, ys };
    let verification_key = secret_key.verification_key(params);

    KeyPair {
        secret_key,
        verification_key,
        index: None,
    }
}

/// Generate a set of n Coconut keypairs [((x, y0, y1...), (g2^x, g2^y0, ...)), ...],
/// such that they support threshold aggregation by `threshold` number of parties.
/// It is expected that this procedure is executed by a Trusted Third Party.
pub fn ttp_keygen(
    params: &Parameters,
    threshold: u64,
    num_authorities: u64,
) -> Result<Vec<KeyPair>> {
    if threshold == 0 {
        return Err(CoconutError::Setup(
            "Tried to generate threshold keys with a 0 threshold value".to_string(),
        ));
    }

    if threshold > num_authorities {
        return Err(
            CoconutError::Setup(
                "Tried to generate threshold keys for threshold value being higher than number of the signing authorities".to_string(),
            ));
    }

    let attributes = params.gen_hs().len();

    // generate polynomials
    let v = Polynomial::new_random(params, threshold - 1);
    let ws = (0..attributes)
        .map(|_| Polynomial::new_random(params, threshold - 1))
        .collect::<Vec<_>>();

    // TODO: potentially if we had some known authority identifier we could use that instead
    // of the increasing (1,2,3,...) sequence
    let polynomial_indices = (1..=num_authorities).collect::<Vec<_>>();

    // generate polynomial shares
    let x = polynomial_indices
        .iter()
        .map(|&id| v.evaluate(&Scalar::from(id)));
    let ys = polynomial_indices.iter().map(|&id| {
        ws.iter()
            .map(|w| w.evaluate(&Scalar::from(id)))
            .collect::<Vec<_>>()
    });

    // finally set the keys
    let secret_keys = x.zip(ys).map(|(x, ys)| SecretKey { x, ys });

    let keypairs = secret_keys
        .zip(polynomial_indices.iter())
        .map(|(secret_key, index)| {
            let verification_key = secret_key.verification_key(params);
            KeyPair {
                secret_key,
                verification_key,
                index: Some(*index),
            }
        })
        .collect();

    Ok(keypairs)
}

#[cfg(test)]
mod tests {
    use crate::scheme::setup::setup;

    use super::*;

    #[test]
    fn keypair_bytes_roundtrip() {
        let params1 = setup(1).unwrap();
        let params5 = setup(5).unwrap();

        let keypair1 = keygen(&params1);
        let keypair5 = keygen(&params5);

        let bytes1 = keypair1.to_bytes();
        let bytes5 = keypair5.to_bytes();

        assert_eq!(KeyPair::from_bytes(&bytes1).unwrap(), keypair1);
        assert_eq!(KeyPair::from_bytes(&bytes5).unwrap(), keypair5);
    }

    #[test]
    fn secret_key_bytes_roundtrip() {
        let params1 = setup(1).unwrap();
        let params5 = setup(5).unwrap();

        let keypair1 = keygen(&params1);
        let keypair5 = keygen(&params5);

        let bytes1 = keypair1.secret_key.to_bytes();
        let bytes5 = keypair5.secret_key.to_bytes();

        assert_eq!(SecretKey::from_bytes(&bytes1).unwrap(), keypair1.secret_key);
        assert_eq!(SecretKey::from_bytes(&bytes5).unwrap(), keypair5.secret_key);
    }

    #[test]
    fn verification_key_bytes_roundtrip() {
        let params1 = setup(1).unwrap();
        let params5 = setup(5).unwrap();

        let keypair1 = &keygen(&params1);
        let keypair5 = &keygen(&params5);

        let bytes1: Vec<u8> = keypair1.verification_key.to_bytes();
        let bytes5: Vec<u8> = keypair5.verification_key.to_bytes();

        assert_eq!(
            VerificationKey::try_from(bytes1.as_slice()).unwrap(),
            keypair1.verification_key
        );
        assert_eq!(
            VerificationKey::try_from(bytes5.as_slice()).unwrap(),
            keypair5.verification_key
        );
    }
}
