use bls12_381::{G1Projective, G2Projective, Scalar};

use crate::scheme::{Phi, VarPhi};
use crate::scheme::keygen::{SecretKeyUser, VerificationKeyAuth};
use crate::scheme::setup::Parameters;

pub struct SpendInstance {
    pub kappa: G2Projective,
    pub phi: Phi,
    pub var_phi: VarPhi,
    pub rr: G1Projective,
    pub ss: G1Projective,
    pub tt: G1Projective,
}

pub struct SpendWitness {
    sk_u: SecretKeyUser,
    v: Scalar,
    r: Scalar,
    r1: Scalar,
    r2: Scalar,
}

#[derive(Debug, Clone)]
pub struct SpendProof {}

impl SpendProof {
    pub fn construct() {}
}



