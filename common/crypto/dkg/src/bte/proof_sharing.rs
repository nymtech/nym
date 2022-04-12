// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::bte::PublicKey;
use crate::error::DkgError;
use crate::interpolation::polynomial::PublicCoefficients;
use crate::utils::{deserialize_g1, deserialize_g2, deserialize_scalar, hash_to_scalar};
use crate::{NodeIndex, Share};
use bls12_381::{G1Projective, G2Projective, Scalar};
use ff::Field;
use group::GroupEncoding;
use rand_core::RngCore;
use std::collections::BTreeMap;

// Domain tries to follow guidelines specified by:
// https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-hash-to-curve-11#section-3.1
const INSTANCE_DOMAIN: &[u8] =
    b"NYM_COCONUT_NIDKG_V01_CS01_WITH_BLS12381_XMD:SHA-256_SSWU_RO_PROOF_SECRET_SHARING_INSTANCE";

const CHALLENGE_DOMAIN: &[u8] =
    b"NYM_COCONUT_NIDKG_V01_CS01_WITH_BLS12381_XMD:SHA-256_SSWU_RO_PROOF_SECRET_SHARING_CHALLENGE";

#[cfg_attr(test, derive(Clone))]
pub struct Instance<'a> {
    public_keys: &'a BTreeMap<NodeIndex, PublicKey>,
    public_coefficients: &'a PublicCoefficients,
    combined_randomizer: &'a G1Projective,
    combined_ciphertexts: &'a [G1Projective],
}

impl<'a> Instance<'a> {
    pub fn new(
        public_keys: &'a BTreeMap<NodeIndex, PublicKey>,
        public_coefficients: &'a PublicCoefficients,
        combined_randomizer: &'a G1Projective,
        combined_ciphertexts: &'a [G1Projective],
    ) -> Instance<'a> {
        Instance {
            public_keys,
            public_coefficients,
            combined_randomizer,
            combined_ciphertexts,
        }
    }

    fn hash_to_scalar(&self) -> Scalar {
        let g1s = self.public_keys.len() + 1 + self.combined_ciphertexts.len();
        let g2s = self.public_coefficients.size();
        let mut bytes = Vec::with_capacity(g1s * 48 + g2s * 96);

        for pk in self.public_keys.values() {
            bytes.extend_from_slice(pk.0.to_bytes().as_ref())
        }
        for coeff in self.public_coefficients.inner() {
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

#[derive(Debug)]
#[cfg_attr(test, derive(Clone, PartialEq))]
pub struct ProofOfSecretSharing {
    ff: G1Projective,
    aa: G2Projective,
    yy: G1Projective,
    response_r: Scalar,
    response_alpha: Scalar,
}

impl ProofOfSecretSharing {
    pub fn construct(
        mut rng: impl RngCore,
        instance: Instance,
        witness_r: &Scalar,
        witnesses_s: &[Share],
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
        let ff = g1 * rho;
        // A = g2^alpha
        let aa = g2 * alpha;

        // Y = (y_1^{x^1} • ...  y_n^{x^n})^rho • g1^alpha
        // produce intermediate product (y_1^{x^1} • ...  y_n^{x^n})
        let product =
            instance
                .public_keys
                .values()
                .rev()
                .fold(G1Projective::identity(), |mut acc, pk| {
                    acc += pk.0;
                    acc *= x;
                    acc
                });
        let yy = product * rho + g1 * alpha;

        let challenge = Self::compute_challenge(&x, &ff, &aa, &yy);

        // response_r = r • challenge + rho
        let response_r = witness_r * challenge + rho;

        // response_alpha = (share_1 • x^1 + ... share_n • x^n) • challenge + alpha
        // produce intermediate sum (share_1 • x^1 + ... share_n • x^n)
        let sum = witnesses_s
            .iter()
            .rev()
            .fold(Scalar::zero(), |mut acc, witness| {
                acc += witness.inner();
                acc *= x;
                acc
            });
        let response_alpha = sum * challenge + alpha;

        Ok(ProofOfSecretSharing {
            ff,
            aa,
            yy,
            response_r,
            response_alpha,
        })
    }

    pub fn verify(&self, instance: Instance) -> bool {
        if !instance.validate() {
            return false;
        }

        let g1 = G1Projective::generator();
        let g2 = G2Projective::generator();

        let x = instance.hash_to_scalar();
        let challenge = Self::compute_challenge(&x, &self.ff, &self.aa, &self.yy);

        // check if R^challenge * F == g1^response_r
        if instance.combined_randomizer * challenge + self.ff != g1 * self.response_r {
            return false;
        }

        // check if
        // (A_0 ^ (id1^0 • x^1 + ... idn^0 • x^n) • ... A_{t-1} ^ (id1^{t-1} • x^{t-1} + ... idn^{t-1} • x^n))^challenge * A
        // ==
        // g2^response_alpha
        let product = instance
            .public_coefficients
            .inner()
            .iter()
            .enumerate()
            .fold(G2Projective::identity(), |mut acc, (k, coeff)| {
                // intermediate (id1^k • x^1 + ... + idn^k • x^n) sum
                let sum: Scalar = instance
                    .public_keys
                    .keys()
                    .enumerate()
                    .map(|(i, node_id)| {
                        let id_scalar = Scalar::from(*node_id);
                        id_scalar.pow(&[k as u64, 0, 0, 0]) * x.pow(&[(i + 1) as u64, 0, 0, 0])
                    })
                    .sum();

                acc += coeff * sum;
                acc
            });

        if product * challenge + self.aa != g2 * self.response_alpha {
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
                .values()
                .rev()
                .fold(G1Projective::identity(), |mut acc, pk| {
                    acc += pk.0;
                    acc *= x;
                    acc
                });

        if product_1 * challenge + self.yy != product_2 * self.response_r + g1 * self.response_alpha
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

    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        // we have 2 G1 elements, single G2 element and 2 scalars
        let mut bytes = Vec::with_capacity(2 * 48 + 96 + 2 * 32);
        bytes.extend_from_slice(self.ff.to_bytes().as_ref());
        bytes.extend_from_slice(self.aa.to_bytes().as_ref());
        bytes.extend_from_slice(self.yy.to_bytes().as_ref());
        bytes.extend_from_slice(self.response_r.to_bytes().as_ref());
        bytes.extend_from_slice(self.response_alpha.to_bytes().as_ref());
        bytes
    }

    pub(crate) fn try_from_bytes(bytes: &[u8]) -> Result<Self, DkgError> {
        if bytes.len() != 2 * 48 + 96 + 2 * 32 {
            return Err(DkgError::new_deserialization_failure(
                "ProofOfSecretSharing",
                "invalid number of bytes provided",
            ));
        }

        let mut i = 0;
        let f = deserialize_g1(&bytes[i..i + 48]).ok_or_else(|| {
            DkgError::new_deserialization_failure("ProofOfSecretSharing.f", "invalid curve point")
        })?;
        i += 48;

        let a = deserialize_g2(&bytes[i..i + 96]).ok_or_else(|| {
            DkgError::new_deserialization_failure("ProofOfSecretSharing.a", "invalid curve point")
        })?;
        i += 96;

        let y = deserialize_g1(&bytes[i..i + 48]).ok_or_else(|| {
            DkgError::new_deserialization_failure("ProofOfSecretSharing.y", "invalid curve point")
        })?;
        i += 48;

        let response_r = deserialize_scalar(&bytes[i..i + 32]).ok_or_else(|| {
            DkgError::new_deserialization_failure(
                "ProofOfSecretSharing.response_r",
                "invalid scalar",
            )
        })?;
        i += 32;

        let response_alpha = deserialize_scalar(&bytes[i..]).ok_or_else(|| {
            DkgError::new_deserialization_failure(
                "ProofOfSecretSharing.response_alpha",
                "invalid scalar",
            )
        })?;

        Ok(ProofOfSecretSharing {
            ff: f,
            aa: a,
            yy: y,
            response_r,
            response_alpha,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpolation::polynomial::Polynomial;
    use group::Group;
    use rand_core::SeedableRng;

    const NODES: u64 = 50;
    const THRESHOLD: u64 = 40;

    fn setup(
        mut rng: impl RngCore,
    ) -> (
        BTreeMap<NodeIndex, PublicKey>,
        PublicCoefficients,
        G1Projective,
        Vec<G1Projective>,
        Scalar,
        Vec<Share>,
    ) {
        let g1 = G1Projective::generator();

        let mut pks = BTreeMap::new();
        let polynomial = Polynomial::new_random(&mut rng, THRESHOLD - 1);
        let public_coefficients = polynomial.public_coefficients();

        let mut shares: Vec<Share> = Vec::new();
        let mut node_indices = (0..NODES).map(|_| rng.next_u64()).collect::<Vec<_>>();
        node_indices.sort_unstable();

        for node_index in node_indices {
            let share = polynomial.evaluate_at(&Scalar::from(node_index));
            shares.push(share.into());
            pks.insert(node_index, PublicKey(g1 * Scalar::random(&mut rng)));
        }

        let r = Scalar::random(&mut rng);
        let rr = g1 * r;

        let ciphertexts = pks
            .values()
            .zip(&shares)
            .map(|(pk, share)| pk.0 * r + g1 * share.inner())
            .collect();
        (pks, public_coefficients, rr, ciphertexts, r, shares)
    }

    #[test]
    fn should_fail_to_create_proof_with_invalid_instance() {
        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let g1 = G1Projective::generator();

        let mut pks = BTreeMap::new();
        let polynomial = Polynomial::new_random(&mut rng, THRESHOLD - 1);
        let public_coefficients = polynomial.public_coefficients();

        let mut shares: Vec<Share> = Vec::new();
        for _ in 0..NODES {
            let node_index = rng.next_u64();
            let share = polynomial.evaluate_at(&Scalar::from(node_index));
            shares.push(share.into());
            pks.insert(node_index, PublicKey(g1 * Scalar::random(&mut rng)));
        }

        let r = Scalar::random(&mut rng);
        let rr = g1 * r;

        let mut shares = Vec::new();
        for node_id in 1..NODES + 1 {
            let share = polynomial.evaluate_at(&Scalar::from(node_id));
            shares.push(share);
        }

        let ciphertexts = pks
            .values()
            .zip(&shares)
            .map(|(pk, share)| pk.0 * r + g1 * share)
            .collect::<Vec<_>>();

        // no public keys
        let bad_instance1 = Instance {
            public_keys: &BTreeMap::new(),
            public_coefficients: &public_coefficients,
            combined_randomizer: &rr,
            combined_ciphertexts: &ciphertexts,
        };
        assert!(!bad_instance1.validate());

        // no public coefficients
        let bad_instance2 = Instance {
            public_keys: &pks,
            public_coefficients: &PublicCoefficients {
                coefficients: Vec::new(),
            },
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

        // changed index of one of the keys
        let mut bad_pks = pks.clone();
        let first_id = bad_pks.keys().copied().take(1).collect::<Vec<_>>();
        let first_val = bad_pks.remove(&first_id[0]).unwrap();
        bad_pks.insert(rng.next_u64(), first_val);

        let bad_instance5 = Instance {
            public_keys: &bad_pks,
            public_coefficients: &public_coefficients,
            combined_randomizer: &rr,
            combined_ciphertexts: &bad_ciphertexts,
        };
        assert!(!bad_instance5.validate());

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
            public_keys: &BTreeMap::new(),
            public_coefficients: &public_coefficients,
            combined_randomizer: &rr,
            combined_ciphertexts: &ciphertexts,
        };
        assert!(!sharing_proof.verify(bad_instance1));

        // no public coefficients
        let bad_instance2 = Instance {
            public_keys: &public_keys,
            public_coefficients: &PublicCoefficients {
                coefficients: Vec::new(),
            },
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
        bad_proof.ff = G1Projective::generator();
        assert!(!bad_proof.verify(instance.clone()));

        let mut bad_proof = good_proof.clone();
        bad_proof.aa = G2Projective::generator();
        assert!(!bad_proof.verify(instance.clone()));

        let mut bad_proof = good_proof.clone();
        bad_proof.yy = G1Projective::generator();
        assert!(!bad_proof.verify(instance.clone()));

        let mut bad_proof = good_proof.clone();
        bad_proof.response_r = Scalar::from(42);
        assert!(!bad_proof.verify(instance.clone()));

        let mut bad_proof = good_proof;
        bad_proof.response_alpha = Scalar::from(42);
        assert!(!bad_proof.verify(instance));
    }

    #[test]
    fn proof_of_secret_sharing_roundtrip() {
        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let proof_fixture = ProofOfSecretSharing {
            ff: G1Projective::random(&mut rng),
            aa: G2Projective::random(&mut rng),
            yy: G1Projective::random(&mut rng),
            response_r: Scalar::random(&mut rng),
            response_alpha: Scalar::random(&mut rng),
        };

        let bytes = proof_fixture.to_bytes();
        let recovered = ProofOfSecretSharing::try_from_bytes(&bytes).unwrap();
        assert_eq!(proof_fixture, recovered);

        assert!(ProofOfSecretSharing::try_from_bytes(&bytes[1..]).is_err())
    }
}
