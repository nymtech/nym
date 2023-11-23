use std::collections::HashMap;
use std::ops::Index;

use bls12_381::{G1Affine, G1Projective, G2Affine, G2Prepared, G2Projective, Scalar};
use ff::Field;
use rand::thread_rng;

use crate::error::{CompactEcashError, Result};
use crate::utils::{hash_g1, Signature};
use crate::scheme::keygen::{SecretKeyAuth, VerificationKeyAuth};
use crate::constants;
use rayon::prelude::*;


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
    pub fn new() -> Result<GroupParameters> {
        let gammas = (1..=constants::ATTRIBUTES_LEN)
            .map(|i| hash_g1(format!("gamma{}", i)))
            .collect();

        let delta = hash_g1("delta");

        Ok(GroupParameters {
            g1: G1Affine::generator(),
            g2: G2Affine::generator(),
            gammas,
            delta,
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

    pub(crate) fn gamma_idx(&self, i: usize) -> Option<&G1Projective>{
        self.gammas.get(i)
    }

    pub(crate) fn delta(&self) -> &G1Projective { &self.delta }

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
        PublicKeyRP {
            alpha: params.gen2() * self.x,
            beta: params.gen2() * self.y,
        }
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
    pk_rp: PublicKeyRP,
    /// Max value of wallet
    L: u64,
    /// list of signatures for values l in [0, L]
    signs: HashMap<u64, Signature>,
}

impl Parameters {
    pub fn grp(&self) -> &GroupParameters {
        &self.grp
    }
    pub fn pk_rp(&self) -> &PublicKeyRP {
        &self.pk_rp
    }
    pub fn L(&self) -> u64 {
        self.L
    }
    pub fn signs(&self) -> &HashMap<u64, Signature> {
        &self.signs
    }
    pub fn get_sign_by_idx(&self, idx: u64) -> Result<&Signature> {
        match self.signs.get(&idx) {
            Some(val) => return Ok(val),
            None => {
                return Err(CompactEcashError::RangeProofOutOfBound(
                    "Cannot find the range proof signature for the given value. \
                        Check if the requested value is within the bound 0..L"
                        .to_string(),
                ));
            }
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct CoinIndexSignature{
    pub(crate) h : G1Projective,
    pub(crate) s : G1Projective,
}

pub type PartialCoinIndexSignature = CoinIndexSignature;

pub fn sign_coin_indices(params: Parameters, vk: &VerificationKeyAuth, sk_auth: SecretKeyAuth) -> Vec<PartialCoinIndexSignature>{

    let m1: Scalar = Scalar::from_bytes(&constants::TYPE_IDX).unwrap();
    let m2: Scalar = Scalar::from_bytes(&constants::TYPE_IDX).unwrap();
    let mut partial_coins_signatures = Vec::with_capacity(params.L() as usize);

    for l in 0..params.L(){
        let m0: Scalar = Scalar::from(l);
        // Compute the hash h
        let mut concatenated_bytes = Vec::with_capacity(vk.to_bytes().len() + l.to_le_bytes().len());
        concatenated_bytes.extend_from_slice(&vk.to_bytes());
        concatenated_bytes.extend_from_slice(&l.to_le_bytes());
        let h = hash_g1(concatenated_bytes);

        // Sign the attributes by performing scalar-point multiplications and accumulating the result
        let mut s_exponent = sk_auth.x;
        s_exponent += &sk_auth.ys[0] * m0;
        s_exponent += &sk_auth.ys[1] * m1;
        s_exponent += &sk_auth.ys[2] * m2;
        // Create the signature struct of on the coin index
        let coin_idx_sign = CoinIndexSignature{
            h,
            s: h * s_exponent,
        };
        partial_coins_signatures.push(coin_idx_sign);
    }
    partial_coins_signatures
}

pub fn setup(L: u64) -> Parameters {
    let grp = GroupParameters::new().unwrap();
    let x = grp.random_scalar();
    let y = grp.random_scalar();
    let sk_rp = SecretKeyRP { x, y };
    let pk_rp = sk_rp.public_key(&grp);
    let mut signs = HashMap::new();
    for l in 0..L {
        let r = grp.random_scalar();
        let h = grp.gen1() * r;
        signs.insert(
            l,
            Signature {
                0: h,
                1: h * (x + y * Scalar::from(l)),
            },
        );
    }
    Parameters {
        grp,
        pk_rp,
        L,
        signs,
    }
}
