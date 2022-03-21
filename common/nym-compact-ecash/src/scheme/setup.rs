use bls12_381::{G1Affine, G2Affine, Scalar};
use ff::Field;
use group::Curve;
use rand::thread_rng;

use crate::error::Result;
use crate::utils::hash_g1;

const ATTRIBUTES_LEN: usize = 3;
const MAX_COIN_VALUE: usize = 32;

pub struct Parameters {
    /// Generator of the G1 group
    g1: G1Affine,
    /// Generator of the G2 group
    g2: G2Affine,
    /// Additional generators of the G1 group
    gammas: Vec<G1Affine>,
    /// Value of wallet
    L: usize,
}

impl Parameters {
    pub fn new() -> Result<Parameters> {
        let gammas = (1..=ATTRIBUTES_LEN)
            .map(|i| hash_g1(format!("gamma{}", i)).to_affine())
            .collect();
        Ok(Parameters {
            g1: G1Affine::generator(),
            g2: G2Affine::generator(),
            gammas,
            L: MAX_COIN_VALUE,
        })
    }

    pub(crate) fn gen1(&self) -> &G1Affine {
        &self.g1
    }

    pub(crate) fn gen2(&self) -> &G2Affine {
        &self.g2
    }

    pub(crate) fn gammas(&self) -> &Vec<G1Affine> { &self.gammas }

    pub fn random_scalar(&self) -> Scalar {
        // lazily-initialized thread-local random number generator, seeded by the system
        let mut rng = thread_rng();
        Scalar::random(&mut rng)
    }

    pub fn n_random_scalars(&self, n: usize) -> Vec<Scalar> {
        (0..n).map(|_| self.random_scalar()).collect()
    }
}

pub fn setup() -> Result<Parameters> {
    Parameters::new()
}