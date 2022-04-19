use std::collections::HashMap;

use bls12_381::{G1Affine, G1Projective, G2Affine, G2Prepared, G2Projective, Scalar};
use ff::Field;
use group::{Curve, GroupEncoding};
use rand::thread_rng;

use crate::error::Result;
use crate::utils::{hash_g1, Signature};

const ATTRIBUTES_LEN: usize = 3;
const MAX_COIN_VALUE: u64 = 32;

pub struct GroupParameters {
    /// Generator of the G1 group
    g1: G1Affine,
    /// Generator of the G2 group
    g2: G2Affine,
    /// Additional generators of the G1 group
    gammas: Vec<G1Projective>,
    /// Precomputed G2 generator used for the miller loop
    _g2_prepared_miller: G2Prepared,
}

impl GroupParameters {
    pub fn new() -> Result<GroupParameters> {
        let gammas = (1..=ATTRIBUTES_LEN)
            .map(|i| hash_g1(format!("gamma{}", i)))
            .collect();


        Ok(GroupParameters {
            g1: G1Affine::generator(),
            g2: G2Affine::generator(),
            gammas,
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

    pub(crate) fn gamma1(&self) -> &G1Projective {
        &self.gammas[0]
    }

    pub(crate) fn gamma2(&self) -> Option<&G1Projective> {
        self.gammas.get(2)
    }

    pub(crate) fn gamma3(&self) -> Option<&G1Projective> {
        self.gammas.get(3)
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


#[derive(Debug, PartialEq, Clone)]
pub struct SecretKeyRP {
    pub(crate) x: Scalar,
    pub(crate) y: Scalar,
}

impl SecretKeyRP {
    pub fn public_key(&self, params: &GroupParameters) -> PublicKeyRP {
        PublicKeyRP { alpha: params.gen2() * self.x, beta: params.gen2() * self.y }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct PublicKeyRP {
    pub(crate) alpha: G2Projective,
    pub(crate) beta: G2Projective,
}

pub struct Parameters {
    /// group parameters
    grp: GroupParameters,
    /// Public Key for range proof verification
    pkRP: PublicKeyRP,
    /// Max value of wallet
    L: u64,
    /// list of signatures for values l in [0, L]
    signs: HashMap<u64, Signature>,
}

impl Parameters {
    pub fn grp(&self) -> &GroupParameters { &self.grp }
    pub fn pkRP(&self) -> &PublicKeyRP { &self.pkRP }
    pub fn L(&self) -> u64 { self.L }
    pub fn signs(&self) -> &HashMap<u64, Signature> { &self.signs }
    pub fn get_sign_by_idx(&self, idx: u64) -> &Signature { self.signs.get(&idx).unwrap() }
}

pub fn setup() -> Parameters {
    let grp = GroupParameters::new().unwrap();
    let x = grp.random_scalar();
    let y = grp.random_scalar();
    let skRP = SecretKeyRP { x, y };
    let pkRP = skRP.public_key(&grp);
    let mut signs = HashMap::new();
    for l in 0..MAX_COIN_VALUE {
        let r = grp.random_scalar();
        let h = grp.gen1() * r;
        signs.insert(l, Signature { 0: h, 1: h * (x + y * Scalar::from(l)) });
    }
    Parameters {
        grp,
        pkRP,
        L: MAX_COIN_VALUE,
        signs,
    }
}


