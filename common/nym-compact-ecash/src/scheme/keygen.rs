use std::convert::TryFrom;
use std::convert::TryInto;

use bls12_381::{G1Projective, G2Projective, Scalar};

use crate::error::{CompactEcashError, Result};
use crate::scheme::setup::Parameters;
use crate::utils::{try_deserialize_g1_projective, try_deserialize_g2_projective, try_deserialize_scalar, try_deserialize_scalar_vec};

pub struct SecretKeyAuth {
    pub(crate) x: Scalar,
    pub(crate) ys: Vec<Scalar>,
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

        // this conversion will not fail as we are taking the same length of data
        let x_bytes: [u8; 32] = bytes[..32].try_into().unwrap();
        let ys_len = u64::from_le_bytes(bytes[32..40].try_into().unwrap());
        let actual_ys_len = (bytes.len() - 40) / 32;

        if ys_len as usize != actual_ys_len {
            return Err(CompactEcashError::Deserialization(format!(
                "Tried to deserialize secret key with inconsistent ys len (expected {}, got {})",
                ys_len, actual_ys_len
            )));
        }

        let x = try_deserialize_scalar(
            &x_bytes,
            CompactEcashError::Deserialization("Failed to deserialize secret key scalar".to_string()),
        )?;
        let ys = try_deserialize_scalar_vec(
            ys_len,
            &bytes[40..],
            CompactEcashError::Deserialization("Failed to deserialize secret key scalars".to_string()),
        )?;

        Ok(SecretKeyAuth { x, ys })
    }
}

impl SecretKeyAuth {
    pub fn verification_key(&self, params: &Parameters) -> VerificationKeyAuth {
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
        let mut bytes = Vec::with_capacity(8 + (ys_len + 1) as usize * 32);
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

pub struct VerificationKeyAuth {
    pub(crate) alpha: G2Projective,
    pub(crate) beta_g1: Vec<G1Projective>,
    pub(crate) beta_g2: Vec<G2Projective>,
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

        // this conversion will not fail as we are taking the same length of data
        let alpha_bytes: [u8; 96] = bytes[..96].try_into().unwrap();
        let betas_len = u64::from_le_bytes(bytes[96..104].try_into().unwrap());

        let actual_betas_len = (bytes.len() - 104) / (96 + 48);

        if betas_len as usize != actual_betas_len {
            return Err(
                CompactEcashError::Deserialization(
                    format!("Tried to deserialize verification key with inconsistent betas len (expected {}, got {})",
                            betas_len, actual_betas_len
                    )));
        }

        let alpha = try_deserialize_g2_projective(
            &alpha_bytes,
            CompactEcashError::Deserialization(
                "Failed to deserialize verification key G2 point (alpha)".to_string(),
            ),
        )?;

        let mut beta_g1 = Vec::with_capacity(betas_len as usize);
        let mut beta_g1_end: u64 = 0;
        for i in 0..betas_len {
            let start = (104 + i * 48) as usize;
            let end = (start + 48) as usize;
            let beta_i_bytes = bytes[start..end].try_into().unwrap();
            let beta_i = try_deserialize_g1_projective(
                &beta_i_bytes,
                CompactEcashError::Deserialization(
                    "Failed to deserialize verification key G1 point (beta)".to_string(),
                ),
            )?;

            beta_g1_end = end as u64;
            beta_g1.push(beta_i)
        }

        let mut beta_g2 = Vec::with_capacity(betas_len as usize);
        for i in 0..betas_len {
            let start = (beta_g1_end + i * 96) as usize;
            let end = (start + 96) as usize;
            let beta_i_bytes = bytes[start..end].try_into().unwrap();
            let beta_i = try_deserialize_g2_projective(
                &beta_i_bytes,
                CompactEcashError::Deserialization(
                    "Failed to deserialize verification key G2 point (beta)".to_string(),
                ),
            )?;

            beta_g2.push(beta_i)
        }

        Ok(VerificationKeyAuth {
            alpha,
            beta_g1,
            beta_g2,
        })
    }
}

pub struct SecretKeyUser {
    pub(crate) sk: Scalar,
}

pub struct PublicKeyUser {
    pub(crate) pk: G1Projective,
}
