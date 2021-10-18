// Copyright 2021 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use core::ops::Neg;
use std::convert::TryFrom;
use std::convert::TryInto;

use bls12_381::{G1Affine, G1Projective, G2Prepared, G2Projective, multi_miller_loop, Scalar};
use group::{Curve, Group};

use crate::Attribute;
use crate::error::{CoconutError, Result};
use crate::proofs::ProofKappaNu;
use crate::scheme::setup::Parameters;
use crate::scheme::Signature;
use crate::scheme::VerificationKey;
use crate::traits::{Base58, Bytable};
use crate::utils::{try_deserialize_g1_projective, try_deserialize_g2_projective};

// TODO NAMING: this whole thing
// Theta
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Theta {
    // blinded_message (kappa)
    pub blinded_message: G2Projective,
    // sigma
    pub credential: Signature,
    // pi_v
    pub pi_v: ProofKappaNu,
}

impl TryFrom<&[u8]> for Theta {
    type Error = CoconutError;

    fn try_from(bytes: &[u8]) -> Result<Theta> {
        if bytes.len() < 192 {
            return Err(
                CoconutError::Deserialization(
                    format!("Tried to deserialize theta with insufficient number of bytes, expected >= 240, got {}", bytes.len()),
                ));
        }

        let blinded_message_bytes = bytes[..96].try_into().unwrap();
        let blinded_message = try_deserialize_g2_projective(
            &blinded_message_bytes,
            CoconutError::Deserialization("failed to deserialize kappa".to_string()),
        )?;

        let credential = Signature::try_from(&bytes[96..192])?;

        let pi_v = ProofKappaNu::from_bytes(&bytes[192..])?;

        Ok(Theta {
            blinded_message,
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
        )
    }

    // TODO: perhaps also include pi_v.len()?
    // to be determined once we implement serde to make sure its 1:1 compatible
    // with bincode
    // kappa || nu || credential || pi_v
    pub fn to_bytes(&self) -> Vec<u8> {
        let blinded_message_bytes = self.blinded_message.to_affine().to_compressed();
        let credential_bytes = self.credential.to_bytes();
        let proof_bytes = self.pi_v.to_bytes();

        let mut bytes = Vec::with_capacity(192 + proof_bytes.len());
        bytes.extend_from_slice(&blinded_message_bytes);
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
        .zip(verification_key.beta.iter())
        .map(|(priv_attr, beta_i)| beta_i * priv_attr)
        .sum::<G2Projective>()
}

pub fn prove_credential(
    params: &Parameters,
    verification_key: &VerificationKey,
    signature: &Signature,
    private_attributes: &[Attribute],
) -> Result<Theta> {
    if private_attributes.is_empty() {
        return Err(CoconutError::Verification(
            "Tried to prove a credential with an empty set of private attributes".to_string(),
        ));
    }

    if private_attributes.len() > verification_key.beta.len() {
        return Err(
            CoconutError::Verification(
                format!("Tried to prove a credential for higher than supported by the provided verification key number of attributes (max: {}, requested: {})",
                        verification_key.beta.len(),
                        private_attributes.len()
                )));
    }

    // Randomize the signature
    let (signature_prime, sign_blinding_factor) = signature.randomise(params);

    // blinded_message : kappa in the paper.
    // Value kappa is needed since we want to show a signature sigma'.
    // In order to verify sigma' we need both the varification key vk and the message m.
    // However, we do not want to reveal m to whomever we are showing the signature.
    // Thus, we need kappa which allows us to verify sigma'. In particular,
    // kappa is computed on m as input, but thanks to the use or random value r,
    // it does not reveal any information about m.
    let blinded_message = compute_kappa(params, verification_key, private_attributes, sign_blinding_factor);


    let pi_v = ProofKappaNu::construct(
        params,
        verification_key,
        private_attributes,
        &sign_blinding_factor,
        &blinded_message,
    );

    Ok(Theta {
        blinded_message,
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
    if public_attributes.len() + theta.pi_v.private_attributes() > verification_key.beta.len() {
        return false;
    }

    if !theta.verify_proof(params, verification_key) {
        return false;
    }
    // theta.verify_proof(params, verification_key)

    let kappa = if public_attributes.is_empty() {
        theta.blinded_message
    } else {
        let signed_public_attributes = public_attributes
            .iter()
            .zip(
                verification_key
                    .beta
                    .iter()
                    .skip(theta.pi_v.private_attributes()),
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
        .zip(verification_key.beta.iter())
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
        let mut params = setup(1).unwrap();

        let keypair = keygen(&mut params);
        let r = params.random_scalar();
        let s = params.random_scalar();

        let signature = Signature(params.gen1() * r, params.gen1() * s);
        let private_attributes = params.n_random_scalars(1);

        let theta = prove_credential(
            &mut params,
            &keypair.verification_key(),
            &signature,
            &private_attributes,
        )
            .unwrap();

        let bytes = theta.to_bytes();
        assert_eq!(Theta::try_from(bytes.as_slice()).unwrap(), theta);

        let mut params = setup(4).unwrap();

        let keypair = keygen(&mut params);
        let private_attributes = params.n_random_scalars(2);

        let theta = prove_credential(
            &mut params,
            &keypair.verification_key(),
            &signature,
            &private_attributes,
        )
            .unwrap();

        let bytes = theta.to_bytes();
        assert_eq!(Theta::try_from(bytes.as_slice()).unwrap(), theta);
    }
}
