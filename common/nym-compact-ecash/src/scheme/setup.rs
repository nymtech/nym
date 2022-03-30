use bls12_381::{G1Affine, G1Projective, G2Affine, G2Prepared, Scalar};
use ff::Field;
use group::{Curve, GroupEncoding};
use rand::thread_rng;

use crate::error::Result;
use crate::utils::hash_g1;

const ATTRIBUTES_LEN: usize = 3;
const MAX_COIN_VALUE: u64 = 32;

pub struct Parameters {
    /// Generator of the G1 group
    g1: G1Affine,
    /// Generator of the G2 group
    g2: G2Affine,
    /// Additional generators of the G1 group
    gammas: Vec<G1Projective>,
    /// Value of wallet
    L: u64,
    /// Precomputed G2 generator used for the miller loop
    _g2_prepared_miller: G2Prepared,
}

impl Parameters {
    pub fn new() -> Result<Parameters> {
        let gammas = (1..=ATTRIBUTES_LEN)
            .map(|i| hash_g1(format!("gamma{}", i)))
            .collect();
        Ok(Parameters {
            g1: G1Affine::generator(),
            g2: G2Affine::generator(),
            gammas,
            L: MAX_COIN_VALUE,
            _g2_prepared_miller: G2Prepared::from(G2Affine::generator()),
        })
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

    pub(crate) fn gamma1(&self) -> &G1Projective { &self.gammas[0] }

    pub(crate) fn gamma2(&self) -> Option<&G1Projective> { self.gammas.get(2) }

    pub(crate) fn gamma3(&self) -> Option<&G1Projective> { self.gammas.get(3) }

    pub(crate) fn L(&self) -> u64 { self.L }

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

pub fn setup() -> Result<Parameters> {
    Parameters::new()
}
