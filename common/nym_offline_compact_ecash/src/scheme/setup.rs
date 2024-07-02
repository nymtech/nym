// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ecash_group_parameters;
use crate::utils::hash_g1;
use bls12_381::{G1Affine, G1Projective, G2Affine, G2Prepared, Scalar};
use ff::Field;
use group::GroupEncoding;
use rand::thread_rng;

#[derive(Debug)]
pub struct GroupParameters {
    /// Generator of the G1 group
    g1: G1Affine,
    /// Generator of the G2 group
    g2: G2Affine,
    /// Additional generators of the G1 group
    gammas: Vec<G1Projective>,
    // Additional generator of the G1 group
    delta: G1Projective,
    /// Precomputed G2 generator used for the miller loop
    _g2_prepared_miller: G2Prepared,
}

impl GroupParameters {
    pub fn new(attributes: usize) -> GroupParameters {
        assert!(attributes > 0);
        let gammas = (1..=attributes)
            .map(|i| hash_g1(format!("gamma{}", i)))
            .collect();

        let delta = hash_g1("delta");

        GroupParameters {
            g1: G1Affine::generator(),
            g2: G2Affine::generator(),
            gammas,
            delta,
            _g2_prepared_miller: G2Prepared::from(G2Affine::generator()),
        }
    }

    pub(crate) fn gen1(&self) -> &G1Affine {
        &self.g1
    }

    pub(crate) fn gen2(&self) -> &G2Affine {
        &self.g2
    }

    pub(crate) fn gammas(&self) -> &Vec<G1Projective> {
        &self.gammas
    }

    pub(crate) fn gammas_to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.gammas.len() * 48);
        for g in &self.gammas {
            bytes.extend_from_slice(g.to_bytes().as_ref());
        }
        bytes
    }

    pub(crate) fn gamma_idx(&self, i: usize) -> Option<&G1Projective> {
        self.gammas.get(i)
    }

    pub(crate) fn delta(&self) -> &G1Projective {
        &self.delta
    }

    pub fn random_scalar(&self) -> Scalar {
        // lazily-initialized thread-local random number generator, seeded by the system
        let mut rng = thread_rng();
        Scalar::random(&mut rng)
    }

    pub fn n_random_scalars(&self, n: usize) -> Vec<Scalar> {
        (0..n).map(|_| self.random_scalar()).collect()
    }

    pub(crate) fn prepared_miller_g2(&self) -> &G2Prepared {
        &self._g2_prepared_miller
    }
}

#[derive(Debug)]
pub struct Parameters {
    /// Number of coins of fixed denomination in the credential wallet; L in construction
    total_coins: u64,
}

impl Parameters {
    pub fn new(total_coins: u64) -> Parameters {
        assert!(total_coins > 0);
        Parameters { total_coins }
    }
    pub fn grp(&self) -> &GroupParameters {
        ecash_group_parameters()
    }

    pub fn get_total_coins(&self) -> u64 {
        self.total_coins
    }
}
