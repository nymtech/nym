// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use core::ops::Neg;
use std::convert::TryFrom;
use std::convert::TryInto;

use bls12_381::{multi_miller_loop, G1Affine, G2Prepared, G2Projective, Scalar};
use group::{Curve, Group};

use crate::error::{CoconutError, Result};
use crate::proofs::ProofKappaZeta;
use crate::scheme::setup::Parameters;
use crate::scheme::Signature;
use crate::scheme::VerificationKey;
use crate::traits::{Base58, Bytable};
use crate::utils::try_deserialize_g2_projective;
use crate::Attribute;

// TODO NAMING: this whole thing
// Theta
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Theta {
    // blinded_message (kappa)
    pub blinded_message: G2Projective,
    // blinded serial number (zeta)
    pub blinded_serial_number: G2Projective,
    // sigma
    pub credential: Signature,
    // pi_v
    pub pi_v: ProofKappaZeta,
}

impl TryFrom<&[u8]> for Theta {
    type Error = CoconutError;

    fn try_from(bytes: &[u8]) -> Result<Theta> {
        if bytes.len() < 288 {
            return Err(
                CoconutError::Deserialization(
                    format!("Tried to deserialize theta with insufficient number of bytes, expected >= 288, got {}", bytes.len()),
                ));
        }

        let blinded_message_bytes = bytes[..96].try_into().unwrap();
        let blinded_message = try_deserialize_g2_projective(
            &blinded_message_bytes,
            CoconutError::Deserialization(
                "failed to deserialize the blinded message (kappa)".to_string(),
            ),
        )?;

        let blinded_serial_number_bytes = bytes[96..192].try_into().unwrap();
        let blinded_serial_number = try_deserialize_g2_projective(
            &blinded_serial_number_bytes,
            CoconutError::Deserialization(
                "failed to deserialize the blinded serial number (zeta)".to_string(),
            ),
        )?;
        let credential = Signature::try_from(&bytes[192..288])?;

        let pi_v = ProofKappaZeta::from_bytes(&bytes[288..])?;

        Ok(Theta {
            blinded_message,
            blinded_serial_number,
            credential,
            pi_v,
        })
    }
}

impl Theta {
    fn verify_proof(&self, params: &Parameters, verification_key: &VerificationKey) -> bool {
        self.pi_v.verify(
            params,
            verification_key,
            &self.blinded_message,
            &self.blinded_serial_number,
        )
    }

    // blinded message (kappa)  || blinded serial number (zeta) || credential || pi_v
    pub fn to_bytes(&self) -> Vec<u8> {
        let blinded_message_bytes = self.blinded_message.to_affine().to_compressed();
        let blinded_serial_number_bytes = self.blinded_serial_number.to_affine().to_compressed();
        let credential_bytes = self.credential.to_bytes();
        let proof_bytes = self.pi_v.to_bytes();

        let mut bytes = Vec::with_capacity(288 + proof_bytes.len());
        bytes.extend_from_slice(&blinded_message_bytes);
        bytes.extend_from_slice(&blinded_serial_number_bytes);
        bytes.extend_from_slice(&credential_bytes);
        bytes.extend_from_slice(&proof_bytes);

        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Theta> {
        Theta::try_from(bytes)
    }
}

impl Bytable for Theta {
    fn to_byte_vec(&self) -> Vec<u8> {
        self.to_bytes()
    }

    fn try_from_byte_slice(slice: &[u8]) -> Result<Self> {
        Theta::try_from(slice)
    }
}

impl Base58 for Theta {}

pub fn compute_kappa(
    params: &Parameters,
    verification_key: &VerificationKey,
    private_attributes: &[Attribute],
    blinding_factor: Scalar,
) -> G2Projective {
    params.gen2() * blinding_factor
        + verification_key.alpha
        + private_attributes
            .iter()
            .zip(verification_key.beta_g2.iter())
            .map(|(priv_attr, beta_i)| beta_i * priv_attr)
            .sum::<G2Projective>()
}

pub fn compute_zeta(params: &Parameters, serial_number: Attribute) -> G2Projective {
    params.gen2() * serial_number
}

pub fn prove_bandwidth_credential(
    params: &Parameters,
    verification_key: &VerificationKey,
    signature: &Signature,
    serial_number: Attribute,
    binding_number: Attribute,
) -> Result<Theta> {
    if verification_key.beta_g2.len() < 2 {
        return Err(
            CoconutError::Verification(
                format!("Tried to prove a credential for higher than supported by the provided verification key number of attributes (max: {}, requested: 2)",
                        verification_key.beta_g2.len()
                )));
    }

    // Randomize the signature
    let (signature_prime, sign_blinding_factor) = signature.randomise(params);

    // blinded_message : kappa in the paper.
    // Value kappa is needed since we want to show a signature sigma'.
    // In order to verify sigma' we need both the verification key vk and the message m.
    // However, we do not want to reveal m to whomever we are showing the signature.
    // Thus, we need kappa which allows us to verify sigma'. In particular,
    // kappa is computed on m as input, but thanks to the use or random value r,
    // it does not reveal any information about m.
    let private_attributes = vec![serial_number, binding_number];
    let blinded_message = compute_kappa(
        params,
        verification_key,
        &private_attributes,
        sign_blinding_factor,
    );

    // zeta is a commitment to the serial number (i.e., a public value associated with the serial number)
    let blinded_serial_number = compute_zeta(params, serial_number);

    let pi_v = ProofKappaZeta::construct(
        params,
        verification_key,
        &serial_number,
        &binding_number,
        &sign_blinding_factor,
        &blinded_message,
        &blinded_serial_number,
    );

    Ok(Theta {
        blinded_message,
        blinded_serial_number,
        credential: signature_prime,
        pi_v,
    })
}

/// Checks whether e(P, Q) * e(-R, S) == id
pub fn check_bilinear_pairing(p: &G1Affine, q: &G2Prepared, r: &G1Affine, s: &G2Prepared) -> bool {
    // checking e(P, Q) * e(-R, S) == id
    // is equivalent to checking e(P, Q) == e(R, S)
    // but requires only a single final exponentiation rather than two of them
    // and therefore, as seen via benchmarks.rs, is almost 50% faster
    // (1.47ms vs 2.45ms, tested on R9 5900X)

    let multi_miller = multi_miller_loop(&[(p, q), (&r.neg(), s)]);
    multi_miller.final_exponentiation().is_identity().into()
}

pub fn verify_credential(
    params: &Parameters,
    verification_key: &VerificationKey,
    theta: &Theta,
    public_attributes: &[Attribute],
) -> bool {
    if public_attributes.len() + theta.pi_v.private_attributes_len()
        > verification_key.beta_g2.len()
    {
        return false;
    }

    if !theta.verify_proof(params, verification_key) {
        return false;
    }

    let kappa = if public_attributes.is_empty() {
        theta.blinded_message
    } else {
        let signed_public_attributes = public_attributes
            .iter()
            .zip(
                verification_key
                    .beta_g2
                    .iter()
                    .skip(theta.pi_v.private_attributes_len()),
            )
            .map(|(pub_attr, beta_i)| beta_i * pub_attr)
            .sum::<G2Projective>();

        theta.blinded_message + signed_public_attributes
    };

    check_bilinear_pairing(
        &theta.credential.0.to_affine(),
        &G2Prepared::from(kappa.to_affine()),
        &(theta.credential.1).to_affine(),
        params.prepared_miller_g2(),
    ) && !bool::from(theta.credential.0.is_identity())
}

// Used in tests only
#[cfg(test)]
pub fn verify(
    params: &Parameters,
    verification_key: &VerificationKey,
    public_attributes: &[Attribute],
    sig: &Signature,
) -> bool {
    let kappa = (verification_key.alpha
        + public_attributes
            .iter()
            .zip(verification_key.beta_g2.iter())
            .map(|(m_i, b_i)| b_i * m_i)
            .sum::<G2Projective>())
    .to_affine();

    check_bilinear_pairing(
        &sig.0.to_affine(),
        &G2Prepared::from(kappa),
        &sig.1.to_affine(),
        params.prepared_miller_g2(),
    ) && !bool::from(sig.0.is_identity())
}

#[cfg(test)]
mod tests {
    use crate::scheme::keygen::keygen;
    use crate::scheme::setup::setup;

    use super::*;

    #[test]
    fn theta_bytes_roundtrip() {
        let params = setup(2).unwrap();

        let keypair = keygen(&params);
        let r = params.random_scalar();
        let s = params.random_scalar();

        let signature = Signature(params.gen1() * r, params.gen1() * s);
        let serial_number = params.random_scalar();
        let binding_number = params.random_scalar();

        let theta = prove_bandwidth_credential(
            &params,
            &keypair.verification_key(),
            &signature,
            serial_number,
            binding_number,
        )
        .unwrap();

        let bytes = theta.to_bytes();
        assert_eq!(Theta::try_from(bytes.as_slice()).unwrap(), theta);
    }
}
