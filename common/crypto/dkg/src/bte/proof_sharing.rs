// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::bte::PublicKey;
use crate::error::DkgError;
use crate::utils::hash_to_scalar;
use bls12_381::{G1Projective, G2Projective, Scalar};
use ff::Field;
use group::GroupEncoding;
use rand_core::RngCore;

// Domain tries to follow guidelines specified by:
// https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-hash-to-curve-11#section-3.1
const INSTANCE_DOMAIN: &[u8] =
    b"NYM_COCONUT_NIDKG_V01_CS01_WITH_BLS12381_XMD:SHA-256_SSWU_RO_PROOF_SECRET_SHARING_INSTANCE";

const CHALLENGE_DOMAIN: &[u8] =
    b"NYM_COCONUT_NIDKG_V01_CS01_WITH_BLS12381_XMD:SHA-256_SSWU_RO_PROOF_SECRET_SHARING_CHALLENGE";

// TODO: perhaps break it down into separate arguments after all
#[cfg_attr(test, derive(Clone))]
pub(crate) struct Instance<'a> {
    public_keys: &'a [PublicKey],
    public_coefficients: &'a [G2Projective],
    combined_randomizer: &'a G1Projective,
    combined_ciphertexts: &'a [G1Projective],
}

impl<'a> Instance<'a> {
    fn hash_to_scalar(&self) -> Scalar {
        let g1s = self.public_keys.len() + 1 + self.combined_ciphertexts.len();
        let g2s = self.public_coefficients.len();
        let mut bytes = Vec::with_capacity(g1s * 48 + g2s * 96);

        for pk in self.public_keys {
            bytes.extend_from_slice(pk.0.to_bytes().as_ref())
        }
        for coeff in self.public_coefficients {
            bytes.extend_from_slice(coeff.to_bytes().as_ref())
        }
        bytes.extend_from_slice(self.combined_randomizer.to_bytes().as_ref());

        for ciphertext in self.combined_ciphertexts {
            bytes.extend_from_slice(ciphertext.to_bytes().as_ref())
        }

        hash_to_scalar(&bytes, INSTANCE_DOMAIN)
    }

    fn validate(&self) -> bool {
        if self.public_keys.is_empty() || self.public_coefficients.is_empty() {
            return false;
        }

        if self.public_keys.len() != self.combined_ciphertexts.len() {
            return false;
        }

        true
    }
}

#[cfg_attr(test, derive(Clone))]
pub struct ProofOfSecretSharing {
    // TODO: ask @AP for better names for those
    f: G1Projective,
    a: G2Projective,
    y: G1Projective,
    response_r: Scalar,
    response_alpha: Scalar,
}

impl ProofOfSecretSharing {
    pub(crate) fn construct(
        mut rng: impl RngCore,
        instance: Instance,
        witness_r: &Scalar,
        // TODO: are those just shares?
        witnesses_s: &[Scalar],
    ) -> Result<Self, DkgError> {
        if !instance.validate() {
            return Err(DkgError::MalformedProofOfSharingInstance);
        }

        let g1 = G1Projective::generator();
        let g2 = G2Projective::generator();

        let x = instance.hash_to_scalar();

        // alpha, rho ← random_scalars
        let alpha = Scalar::random(&mut rng);
        let rho = Scalar::random(&mut rng);

        // F = g1^rho
        let f = g1 * rho;
        // A = g2^alpha
        let a = g2 * alpha;

        // Y = (y_1^{x^1} • ...  y_n^{x^n})^rho • g1^alpha
        // produce intermediate product (y_1^{x^1} • ...  y_n^{x^n})
        let product =
            instance
                .public_keys
                .iter()
                .rev()
                .fold(G1Projective::identity(), |mut acc, pk| {
                    acc += pk.0;
                    acc *= x;
                    acc
                });
        let y = product * rho + g1 * alpha;

        let challenge = Self::compute_challenge(&x, &f, &a, &y);

        // response_r = r • challenge + rho
        let response_r = witness_r * challenge + rho;

        // response_alpha = (share_1 • x^1 + ... share_n • x^n) • challenge + alpha
        // produce intermediate sum (share_1 • x^1 + ... share_n • x^n)
        let sum = witnesses_s
            .iter()
            .rev()
            .fold(Scalar::zero(), |mut acc, witness| {
                acc += witness;
                acc *= x;
                acc
            });
        let response_alpha = sum * challenge + alpha;

        Ok(ProofOfSecretSharing {
            f,
            a,
            y,
            response_r,
            response_alpha,
        })
    }

    pub(crate) fn verify(&self, instance: Instance) -> bool {
        if !instance.validate() {
            return false;
        }

        let g1 = G1Projective::generator();
        let g2 = G2Projective::generator();

        let x = instance.hash_to_scalar();
        let challenge = Self::compute_challenge(&x, &self.f, &self.a, &self.y);

        // check if R^challenge * F == g1^response_r
        if instance.combined_randomizer * challenge + self.f != g1 * self.response_r {
            return false;
        }

        // check if
        // (A_0 ^ (1^0 • x^1 + ... i^0 • x^n) • ... A_{t-1} ^ (1^{t-1} • x^{t-1} + ... i^{t-1} • x^n))^challenge * A
        // ==
        // g2^response_alpha
        let n = instance.public_keys.len();

        let product = instance.public_coefficients.iter().enumerate().fold(
            G2Projective::identity(),
            |mut acc, (k, coeff)| {
                // intermediate (1^k • x^1 + ... + n^k • x^n) sum
                let sum: Scalar = (1..=n)
                    .map(|i| {
                        let i_scalar = Scalar::from(i as u64);
                        i_scalar.pow(&[k as u64, 0, 0, 0]) * x.pow(&[i as u64, 0, 0, 0])
                    })
                    .sum();

                acc += coeff * sum;
                acc
            },
        );

        if product * challenge + self.a != g2 * self.response_alpha {
            return false;
        }

        // check if
        // (ciphertext_1 ^ (x^1) • ... ciphertext_n ^ (x^n)) ^ challenge • Y
        // ==
        // (pk_1 ^ (x^1) • ... pk_n ^ (x^n)) ^ response_r • g1^response_alpha

        let product_1 = instance.combined_ciphertexts.iter().rev().fold(
            G1Projective::identity(),
            |mut acc, ciphertext| {
                acc += ciphertext;
                acc *= x;
                acc
            },
        );

        let product_2 =
            instance
                .public_keys
                .iter()
                .rev()
                .fold(G1Projective::identity(), |mut acc, pk| {
                    acc += pk.0;
                    acc *= x;
                    acc
                });

        if product_1 * challenge + self.y != product_2 * self.response_r + g1 * self.response_alpha
        {
            return false;
        }

        true
    }

    pub(crate) fn compute_challenge(
        commitment: &Scalar,
        blinder_g1: &G1Projective,
        blinder_g2: &G2Projective,
        blinded_instance: &G1Projective,
    ) -> Scalar {
        let mut bytes = Vec::with_capacity(224);

        bytes.extend_from_slice(commitment.to_bytes().as_ref());
        bytes.extend_from_slice(blinder_g1.to_bytes().as_ref());
        bytes.extend_from_slice(blinder_g2.to_bytes().as_ref());
        bytes.extend_from_slice(blinded_instance.to_bytes().as_ref());

        hash_to_scalar(&bytes, CHALLENGE_DOMAIN)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpolation::polynomial::Polynomial;
    use rand_core::SeedableRng;

    const NODES: u64 = 50;
    const THRESHOLD: u64 = 40;

    fn setup(
        mut rng: impl RngCore,
    ) -> (
        Vec<PublicKey>,
        Vec<G2Projective>,
        G1Projective,
        Vec<G1Projective>,
        Scalar,
        Vec<Scalar>,
    ) {
        let g1 = G1Projective::generator();
        let g2 = G2Projective::generator();

        let mut pks = Vec::new();
        let polynomial = Polynomial::new_random(&mut rng, THRESHOLD - 1);
        let public_coefficients = polynomial.public_coefficients();

        for _ in 0..NODES {
            pks.push(PublicKey(g1 * Scalar::random(&mut rng)));
        }

        let r = Scalar::random(&mut rng);
        let rr = g1 * r;

        let mut shares = Vec::new();
        for node_id in 1..NODES + 1 {
            let share = polynomial.evaluate(&Scalar::from(node_id));
            shares.push(share);
        }

        let ciphertexts = pks
            .iter()
            .zip(&shares)
            .map(|(pk, share)| pk.0 * r + g1 * share)
            .collect();
        (pks, public_coefficients, rr, ciphertexts, r, shares)
    }

    #[test]
    fn should_fail_to_create_proof_with_invalid_instance() {
        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let g1 = G1Projective::generator();
        let g2 = G2Projective::generator();

        let mut pks = Vec::new();
        let polynomial = Polynomial::new_random(&mut rng, THRESHOLD - 1);
        let public_coefficients = polynomial.public_coefficients();

        for _ in 0..NODES {
            pks.push(PublicKey(g1 * Scalar::random(&mut rng)));
        }

        let r = Scalar::random(&mut rng);
        let rr = g1 * r;

        let mut shares = Vec::new();
        for node_id in 1..NODES + 1 {
            let share = polynomial.evaluate(&Scalar::from(node_id));
            shares.push(share);
        }

        let ciphertexts = pks
            .iter()
            .zip(&shares)
            .map(|(pk, share)| pk.0 * r + g1 * share)
            .collect::<Vec<_>>();

        // no public keys
        let bad_instance1 = Instance {
            public_keys: &[],
            public_coefficients: &public_coefficients,
            combined_randomizer: &rr,
            combined_ciphertexts: &ciphertexts,
        };
        assert!(!bad_instance1.validate());

        // no public coefficients
        let bad_instance2 = Instance {
            public_keys: &pks,
            public_coefficients: &[],
            combined_randomizer: &rr,
            combined_ciphertexts: &ciphertexts,
        };
        assert!(!bad_instance2.validate());

        // no ciphertexts
        let bad_instance3 = Instance {
            public_keys: &pks,
            public_coefficients: &public_coefficients,
            combined_randomizer: &rr,
            combined_ciphertexts: &[],
        };
        assert!(!bad_instance3.validate());

        // public_keys.len() != combined_ciphertexts.len()
        let bad_ciphertexts = ciphertexts.iter().skip(1).cloned().collect::<Vec<_>>();

        let bad_instance4 = Instance {
            public_keys: &pks,
            public_coefficients: &public_coefficients,
            combined_randomizer: &rr,
            combined_ciphertexts: &bad_ciphertexts,
        };
        assert!(!bad_instance4.validate());

        let good_instance = Instance {
            public_keys: &pks,
            public_coefficients: &public_coefficients,
            combined_randomizer: &rr,
            combined_ciphertexts: &ciphertexts,
        };
        assert!(good_instance.validate())
    }

    #[test]
    fn should_verify_a_valid_proof() {
        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let (public_keys, public_coefficients, rr, ciphertexts, r, shares) = setup(&mut rng);

        let instance = Instance {
            public_keys: &public_keys,
            public_coefficients: &public_coefficients,
            combined_randomizer: &rr,
            combined_ciphertexts: &ciphertexts,
        };

        let sharing_proof =
            ProofOfSecretSharing::construct(&mut rng, instance.clone(), &r, &shares).unwrap();

        assert!(sharing_proof.verify(instance))
    }

    #[test]
    fn should_fail_to_verify_proof_with_invalid_instance() {
        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let (public_keys, public_coefficients, rr, ciphertexts, r, shares) = setup(&mut rng);

        let instance = Instance {
            public_keys: &public_keys,
            public_coefficients: &public_coefficients,
            combined_randomizer: &rr,
            combined_ciphertexts: &ciphertexts,
        };

        let sharing_proof =
            ProofOfSecretSharing::construct(&mut rng, instance.clone(), &r, &shares).unwrap();

        // no public keys
        let bad_instance1 = Instance {
            public_keys: &[],
            public_coefficients: &public_coefficients,
            combined_randomizer: &rr,
            combined_ciphertexts: &ciphertexts,
        };
        assert!(!sharing_proof.verify(bad_instance1));

        // no public coefficients
        let bad_instance2 = Instance {
            public_keys: &public_keys,
            public_coefficients: &[],
            combined_randomizer: &rr,
            combined_ciphertexts: &ciphertexts,
        };
        assert!(!sharing_proof.verify(bad_instance2));

        // no ciphertexts
        let bad_instance3 = Instance {
            public_keys: &public_keys,
            public_coefficients: &public_coefficients,
            combined_randomizer: &rr,
            combined_ciphertexts: &[],
        };
        assert!(!sharing_proof.verify(bad_instance3));

        // public_keys.len() != combined_ciphertexts.len()
        let bad_ciphertexts = ciphertexts.iter().skip(1).cloned().collect::<Vec<_>>();

        let bad_instance4 = Instance {
            public_keys: &public_keys,
            public_coefficients: &public_coefficients,
            combined_randomizer: &rr,
            combined_ciphertexts: &bad_ciphertexts,
        };
        assert!(!sharing_proof.verify(bad_instance4));
    }

    #[test]
    fn should_fail_to_verify_proof_with_wrong_instance() {
        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let (public_keys, public_coefficients, rr, ciphertexts, r, shares) = setup(&mut rng);

        let instance = Instance {
            public_keys: &public_keys,
            public_coefficients: &public_coefficients,
            combined_randomizer: &rr,
            combined_ciphertexts: &ciphertexts,
        };

        let sharing_proof =
            ProofOfSecretSharing::construct(&mut rng, instance.clone(), &r, &shares).unwrap();

        let (public_keys, public_coefficients, rr, ciphertexts, _, _) = setup(&mut rng);
        let bad_instance = Instance {
            public_keys: &public_keys,
            public_coefficients: &public_coefficients,
            combined_randomizer: &rr,
            combined_ciphertexts: &ciphertexts,
        };

        assert!(!sharing_proof.verify(bad_instance));
    }

    #[test]
    fn should_fail_to_verify_invalid_proof() {
        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let (public_keys, public_coefficients, rr, ciphertexts, r, shares) = setup(&mut rng);

        let instance = Instance {
            public_keys: &public_keys,
            public_coefficients: &public_coefficients,
            combined_randomizer: &rr,
            combined_ciphertexts: &ciphertexts,
        };

        let good_proof =
            ProofOfSecretSharing::construct(&mut rng, instance.clone(), &r, &shares).unwrap();

        let mut bad_proof = good_proof.clone();
        bad_proof.f = G1Projective::generator();
        assert!(!bad_proof.verify(instance.clone()));

        let mut bad_proof = good_proof.clone();
        bad_proof.a = G2Projective::generator();
        assert!(!bad_proof.verify(instance.clone()));

        let mut bad_proof = good_proof.clone();
        bad_proof.y = G1Projective::generator();
        assert!(!bad_proof.verify(instance.clone()));

        let mut bad_proof = good_proof.clone();
        bad_proof.response_r = Scalar::from(42);
        assert!(!bad_proof.verify(instance.clone()));

        let mut bad_proof = good_proof;
        bad_proof.response_alpha = Scalar::from(42);
        assert!(!bad_proof.verify(instance));
    }
}
