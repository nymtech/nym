use bls12_381::{G1Projective, G2Projective, Scalar};

use crate::scheme::keygen::SecretKeyUser;
use crate::scheme::setup::Parameters;

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct SpendInstance {
    pub kappa: G2Projective,
    pub A: G1Projective,
    pub C: G1Projective,
    pub D: G1Projective,
    pub S: G1Projective,
    pub T: G1Projective,
}

pub struct SpendWitness {
    // includes skUser, v, t
    pub attributes: Vec<Scalar>,
    // signature randomizing element
    pub r: Scalar,
    pub l: u64,
    pub o_a: Scalar,
    pub o_c: Scalar,
    pub o_d: Scalar,
    pub mu: Scalar,
    pub lambda: Scalar,
    pub o_mu: Scalar,
    pub o_lambda: Scalar,

}

pub struct SpendProof {}

impl SpendProof {
    pub fn construct(params: &Parameters,
                     instance: &SpendInstance,
                     witness: &SpendWitness, ) {}
    pub fn verify() {}
}