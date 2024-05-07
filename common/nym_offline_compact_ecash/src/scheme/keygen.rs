// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use core::borrow::Borrow;
use core::iter::Sum;
use core::ops::{Add, Mul};
use std::convert::TryFrom;
use std::convert::TryInto;

use bls12_381::{G1Projective, G2Projective, Scalar};
use group::{Curve, GroupEncoding};
use nym_pemstore::traits::{PemStorableKey, PemStorableKeyPair};
use serde::{Deserialize, Serialize};

use crate::error::{CompactEcashError, Result};
use crate::scheme::aggregation::aggregate_verification_keys;
use crate::scheme::SignerIndex;
use crate::traits::Bytable;
use crate::utils::{hash_to_scalar, Polynomial};
use crate::utils::{
    try_deserialize_g1_projective, try_deserialize_g2_projective, try_deserialize_scalar,
    try_deserialize_scalar_vec,
};
use crate::{ecash_group_parameters, Base58};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Debug, PartialEq, Clone, Zeroize, ZeroizeOnDrop)]
pub struct SecretKeyAuth {
    pub(crate) x: Scalar,
    pub(crate) ys: Vec<Scalar>,
}

impl PemStorableKey for SecretKeyAuth {
    type Error = CompactEcashError;

    fn pem_type() -> &'static str {
        "ECASH SECRET KEY"
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> std::result::Result<Self, Self::Error> {
        Self::from_bytes(bytes)
    }
}

impl TryFrom<&[u8]> for SecretKeyAuth {
    type Error = CompactEcashError;

    fn try_from(bytes: &[u8]) -> Result<SecretKeyAuth> {
        // There should be x and at least one y
        if bytes.len() < 32 * 2 + 8 || (bytes.len() - 8) % 32 != 0 {
            return Err(CompactEcashError::DeserializationInvalidLength {
                actual: bytes.len(),
                modulus_target: bytes.len() - 8,
                target: 32 * 2 + 8,
                modulus: 32,
                object: "secret key".to_string(),
            });
        }

        //SAFETY : slice to array conversion after a length check
        #[allow(clippy::unwrap_used)]
        let x_bytes: [u8; 32] = bytes[..32].try_into().unwrap();

        #[allow(clippy::unwrap_used)]
        let ys_len = u64::from_le_bytes(bytes[32..40].try_into().unwrap());
        let actual_ys_len = (bytes.len() - 40) / 32;

        if ys_len as usize != actual_ys_len {
            return Err(CompactEcashError::DeserializationLengthMismatch {
                type_name: "Secret_key ys".into(),
                expected: ys_len as usize,
                actual: actual_ys_len,
            });
        }

        let x = try_deserialize_scalar(&x_bytes)?;
        let ys = try_deserialize_scalar_vec(ys_len, &bytes[40..])?;

        Ok(SecretKeyAuth { x, ys })
    }
}

impl SecretKeyAuth {
    /// Following a (distributed) key generation process, scalar values can be obtained
    /// outside of the normal key generation process.
    pub fn create_from_raw(x: Scalar, ys: Vec<Scalar>) -> Self {
        Self { x, ys }
    }

    /// Extract the Scalar copy of the underlying secrets.
    /// The caller of this function must exercise extreme care to not misuse the data and ensuring it gets zeroized
    pub fn hazmat_to_raw(&self) -> (Scalar, Vec<Scalar>) {
        (self.x, self.ys.clone())
    }

    pub fn size(&self) -> usize {
        self.ys.len()
    }

    pub(crate) fn get_y_by_idx(&self, i: usize) -> Option<&Scalar> {
        self.ys.get(i)
    }

    pub fn verification_key(&self) -> VerificationKeyAuth {
        let params = ecash_group_parameters();
        let g1 = params.gen1();
        let g2 = params.gen2();
        VerificationKeyAuth {
            alpha: g2 * self.x,
            beta_g1: self.ys.iter().map(|y| g1 * y).collect(),
            beta_g2: self.ys.iter().map(|y| g2 * y).collect(),
        }
    }

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

    pub fn from_bytes(bytes: &[u8]) -> Result<SecretKeyAuth> {
        SecretKeyAuth::try_from(bytes)
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct VerificationKeyAuth {
    pub(crate) alpha: G2Projective,
    pub(crate) beta_g1: Vec<G1Projective>,
    pub(crate) beta_g2: Vec<G2Projective>,
}

impl PemStorableKey for VerificationKeyAuth {
    type Error = CompactEcashError;

    fn pem_type() -> &'static str {
        "ECASH VERIFICATION KEY"
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> std::result::Result<Self, Self::Error> {
        Self::from_bytes(bytes)
    }
}

impl TryFrom<&[u8]> for VerificationKeyAuth {
    type Error = CompactEcashError;

    fn try_from(bytes: &[u8]) -> Result<VerificationKeyAuth> {
        // There should be at least alpha, one betaG1 and one betaG2 and their length
        if bytes.len() < 96 * 2 + 48 + 8 || (bytes.len() - 8 - 96) % (96 + 48) != 0 {
            return Err(CompactEcashError::DeserializationInvalidLength {
                actual: bytes.len(),
                modulus_target: bytes.len() - 8 - 96,
                target: 96 * 2 + 48 + 8,
                modulus: 96 + 48,
                object: "verification key".to_string(),
            });
        }

        //SAFETY : slice to array conversion after a length check
        #[allow(clippy::unwrap_used)]
        let alpha_bytes: [u8; 96] = bytes[..96].try_into().unwrap();
        #[allow(clippy::unwrap_used)]
        let betas_len = u64::from_le_bytes(bytes[96..104].try_into().unwrap());

        let actual_betas_len = (bytes.len() - 104) / (96 + 48);

        if betas_len as usize != actual_betas_len {
            return Err(CompactEcashError::DeserializationLengthMismatch {
                type_name: "Verification_key betas".into(),
                expected: betas_len as usize,
                actual: actual_betas_len,
            });
        }

        let alpha = try_deserialize_g2_projective(&alpha_bytes)?;

        let mut beta_g1 = Vec::with_capacity(betas_len as usize);
        let mut beta_g1_end: u64 = 0;
        for i in 0..betas_len {
            let start = (104 + i * 48) as usize;
            let end = start + 48;
            //SAFETY : slice to array conversion after a length check
            #[allow(clippy::unwrap_used)]
            let beta_i_bytes = bytes[start..end].try_into().unwrap();
            let beta_i = try_deserialize_g1_projective(&beta_i_bytes)?;

            beta_g1_end = end as u64;
            beta_g1.push(beta_i)
        }

        let mut beta_g2 = Vec::with_capacity(betas_len as usize);
        for i in 0..betas_len {
            let start = (beta_g1_end + i * 96) as usize;
            let end = start + 96;
            //SAFETY : slice to array conversion after a length check
            #[allow(clippy::unwrap_used)]
            let beta_i_bytes = bytes[start..end].try_into().unwrap();
            let beta_i = try_deserialize_g2_projective(&beta_i_bytes)?;

            beta_g2.push(beta_i)
        }

        Ok(VerificationKeyAuth {
            alpha,
            beta_g1,
            beta_g2,
        })
    }
}

impl<'b> Add<&'b VerificationKeyAuth> for VerificationKeyAuth {
    type Output = VerificationKeyAuth;

    #[inline]
    fn add(self, rhs: &'b VerificationKeyAuth) -> VerificationKeyAuth {
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

        VerificationKeyAuth {
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

impl<'a> Mul<Scalar> for &'a VerificationKeyAuth {
    type Output = VerificationKeyAuth;

    #[inline]
    fn mul(self, rhs: Scalar) -> Self::Output {
        VerificationKeyAuth {
            alpha: self.alpha * rhs,
            beta_g1: self.beta_g1.iter().map(|b_i| b_i * rhs).collect(),
            beta_g2: self.beta_g2.iter().map(|b_i| b_i * rhs).collect(),
        }
    }
}

impl<T> Sum<T> for VerificationKeyAuth
where
    T: Borrow<VerificationKeyAuth>,
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
                return VerificationKeyAuth::identity(0);
            }
        };

        peekable.fold(
            VerificationKeyAuth::identity(head_attributes),
            |acc, item| acc + item.borrow(),
        )
    }
}

impl VerificationKeyAuth {
    /// Create a (kinda) identity verification key using specified
    /// number of 'beta' elements
    pub(crate) fn identity(beta_size: usize) -> Self {
        VerificationKeyAuth {
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

    pub fn from_bytes(bytes: &[u8]) -> Result<VerificationKeyAuth> {
        VerificationKeyAuth::try_from(bytes)
    }
}

impl Bytable for VerificationKeyAuth {
    fn to_byte_vec(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }

    fn try_from_byte_slice(slice: &[u8]) -> std::result::Result<Self, CompactEcashError> {
        Self::from_bytes(slice)
    }
}

impl Base58 for VerificationKeyAuth {}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct SecretKeyUser {
    pub(crate) sk: Scalar,
}

impl SecretKeyUser {
    pub fn public_key(&self) -> PublicKeyUser {
        PublicKeyUser {
            pk: ecash_group_parameters().gen1() * self.sk,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.sk.to_bytes().to_vec()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let sk = Scalar::try_from_byte_slice(bytes)?;
        Ok(SecretKeyUser { sk })
    }
}

impl Bytable for SecretKeyUser {
    fn to_byte_vec(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }

    fn try_from_byte_slice(slice: &[u8]) -> std::result::Result<Self, CompactEcashError> {
        Self::from_bytes(slice)
    }
}

impl Base58 for SecretKeyUser {}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct PublicKeyUser {
    pub(crate) pk: G1Projective,
}

impl PublicKeyUser {
    pub fn to_base58_string(&self) -> String {
        bs58::encode(&self.pk.to_bytes()).into_string()
    }

    pub fn from_base58_string<I: AsRef<[u8]>>(val: I) -> Result<Self> {
        let bytes = bs58::decode(val).into_vec()?;
        Self::from_bytes(&bytes)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.pk.to_affine().to_compressed().to_vec()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 48 {
            return Err(CompactEcashError::DeserializationLengthMismatch {
                type_name: "PublicKeyUser".into(),
                expected: 48,
                actual: bytes.len(),
            });
        }
        //SAFETY : slice to array conversion after a length check
        #[allow(clippy::unwrap_used)]
        let pk_bytes: &[u8; 48] = bytes[..48].try_into().unwrap();
        let pk = try_deserialize_g1_projective(pk_bytes)?;
        Ok(PublicKeyUser { pk })
    }
}

impl Bytable for PublicKeyUser {
    fn to_byte_vec(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }

    fn try_from_byte_slice(slice: &[u8]) -> Result<Self> {
        Self::from_bytes(slice)
    }
}

impl Base58 for PublicKeyUser {}
#[derive(Debug, Zeroize, ZeroizeOnDrop)]
pub struct KeyPairAuth {
    secret_key: SecretKeyAuth,
    #[zeroize(skip)]
    verification_key: VerificationKeyAuth,
    /// Optional index value specifying polynomial point used during threshold key generation.
    pub index: Option<SignerIndex>,
}

impl PemStorableKeyPair for KeyPairAuth {
    type PrivatePemKey = SecretKeyAuth;
    type PublicPemKey = VerificationKeyAuth;

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

impl KeyPairAuth {
    pub fn new(
        sk: SecretKeyAuth,
        vk: VerificationKeyAuth,
        index: Option<SignerIndex>,
    ) -> KeyPairAuth {
        KeyPairAuth {
            secret_key: sk,
            verification_key: vk,
            index,
        }
    }

    pub fn from_keys(secret_key: SecretKeyAuth, verification_key: VerificationKeyAuth) -> Self {
        Self {
            secret_key,
            verification_key,
            index: None,
        }
    }

    pub fn secret_key(&self) -> &SecretKeyAuth {
        &self.secret_key
    }

    pub fn verification_key(&self) -> VerificationKeyAuth {
        self.verification_key.clone()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyPairUser {
    secret_key: SecretKeyUser,
    public_key: PublicKeyUser,
}

impl KeyPairUser {
    pub fn secret_key(&self) -> &SecretKeyUser {
        &self.secret_key
    }

    pub fn public_key(&self) -> PublicKeyUser {
        self.public_key.clone()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        [self.secret_key.to_bytes(), self.public_key.to_bytes()].concat()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 32 + 48 {
            return Err(CompactEcashError::DeserializationLengthMismatch {
                type_name: "KeyPairUser".into(),
                expected: 80,
                actual: bytes.len(),
            });
        }
        let sk = SecretKeyUser::from_bytes(&bytes[..32])?;
        let pk = PublicKeyUser::from_bytes(&bytes[32..32 + 48])?;
        Ok(KeyPairUser {
            secret_key: sk,
            public_key: pk,
        })
    }
}

pub fn generate_keypair_user() -> KeyPairUser {
    let params = ecash_group_parameters();
    let sk_user = SecretKeyUser {
        sk: params.random_scalar(),
    };
    let pk_user = PublicKeyUser {
        pk: params.gen1() * sk_user.sk,
    };

    KeyPairUser {
        secret_key: sk_user,
        public_key: pk_user,
    }
}

pub fn generate_keypair_user_from_seed(seed: &[u8]) -> KeyPairUser {
    let params = ecash_group_parameters();
    let sk_user = SecretKeyUser {
        sk: hash_to_scalar(seed),
    };
    let pk_user = PublicKeyUser {
        pk: params.gen1() * sk_user.sk,
    };

    KeyPairUser {
        secret_key: sk_user,
        public_key: pk_user,
    }
}

pub fn ttp_keygen(threshold: u64, num_authorities: u64) -> Result<Vec<KeyPairAuth>> {
    let params = ecash_group_parameters();
    if threshold == 0 {
        return Err(CompactEcashError::KeygenParameters);
    }

    if threshold > num_authorities {
        return Err(CompactEcashError::KeygenParameters);
    }

    let attributes = params.gammas().len();

    // generate polynomials
    let v = Polynomial::new_random(params, threshold - 1);
    let ws = (0..attributes + 1)
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
    let secret_keys = x.zip(ys).map(|(x, ys)| SecretKeyAuth { x, ys });

    let keypairs = secret_keys
        .zip(polynomial_indices.iter())
        .map(|(secret_key, index)| {
            let verification_key = secret_key.verification_key();
            KeyPairAuth {
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
    use super::*;

    fn assert_zeroize_on_drop<T: ZeroizeOnDrop>() {}

    fn assert_zeroize<T: Zeroize>() {}

    #[test]
    fn secret_key_is_zeroized() {
        assert_zeroize::<SecretKeyAuth>();
        assert_zeroize_on_drop::<SecretKeyAuth>();

        assert_zeroize::<SecretKeyUser>();
        assert_zeroize_on_drop::<SecretKeyUser>();
    }
}
