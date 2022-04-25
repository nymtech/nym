// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::utils::hash_to_scalar;
use bls12_381::{G1Projective, Scalar};
use ff::Field;
use group::GroupEncoding;
use rand_core::RngCore;
use zeroize::Zeroize;

// Domain tries to follow guidelines specified by:
// https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-hash-to-curve-11#section-3.1
const DISCRETE_LOG_DOMAIN: &[u8] =
    b"NYM_COCONUT_NIDKG_V01_CS01_WITH_BLS12381_XMD:SHA-256_SSWU_RO_PROOF_DISCRETE_LOG";

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct ProofOfDiscreteLog {
    pub(crate) rand_commitment: G1Projective,
    pub(crate) response: Scalar,
}

impl ProofOfDiscreteLog {
    pub fn construct(mut rng: impl RngCore, public: &G1Projective, witness: &Scalar) -> Self {
        let mut rand_x = Scalar::random(&mut rng);
        let rand_commitment = G1Projective::generator() * rand_x;
        let challenge = Self::compute_challenge(public, &rand_commitment);

        let response = rand_x + challenge * witness;
        rand_x.zeroize();

        ProofOfDiscreteLog {
            rand_commitment,
            response,
        }
    }

    // note: we don't have to explicitly check whether points are on correct curves / fields
    // as if they weren't, they'd fail to get deserialized
    pub fn verify(&self, public: &G1Projective) -> bool {
        let challenge = Self::compute_challenge(public, &self.rand_commitment);

        // y^c • a == g1^rand_x
        public * challenge + self.rand_commitment == G1Projective::generator() * self.response
    }

    pub(crate) fn compute_challenge(public: &G1Projective, rand_commit: &G1Projective) -> Scalar {
        let public_bytes = public.to_bytes();
        let rand_commit_bytes = rand_commit.to_bytes();

        let mut bytes = Vec::with_capacity(96);
        bytes.extend_from_slice(public_bytes.as_ref());
        bytes.extend_from_slice(rand_commit_bytes.as_ref());

        hash_to_scalar(bytes, DISCRETE_LOG_DOMAIN)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::SeedableRng;

    #[test]
    fn should_verify_a_valid_proof() {
        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let witness = Scalar::random(&mut rng);
        let public = G1Projective::generator() * witness;

        let proof = ProofOfDiscreteLog::construct(&mut rng, &public, &witness);

        assert!(proof.verify(&public))
    }

    #[test]
    fn should_fail_on_invalid_proof() {
        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let witness = Scalar::random(&mut rng);
        let public = G1Projective::generator() * witness;

        let other_witness = Scalar::random(&mut rng);
        let other_public = G1Projective::generator() * other_witness;

        let proof = ProofOfDiscreteLog::construct(&mut rng, &public, &witness);
        let other_proof = ProofOfDiscreteLog::construct(&mut rng, &other_public, &other_witness);

        assert!(!proof.verify(&other_public));
        assert!(!other_proof.verify(&public));
    }
}
