use bls12_381::{G1Projective, G2Projective, Gt, Scalar};

use crate::scheme::{Phi, VarPhi};
use crate::scheme::keygen::{SecretKeyUser, VerificationKeyAuth};
use crate::scheme::setup::Parameters;

pub struct SpendInstance {
    pub kappa: G2Projective,
    pub phi: Phi,
    pub varphi: VarPhi,
    pub rr: G1Projective,
    pub ss: G1Projective,
    pub tt: G2Projective,
    pub pg_varsigpr1_delta: Gt,
    pub pg_psi0_delta: Gt,
    pub pg_varsigpr2_gen2: Gt,
    pub pg_psi0_gen2: Gt,
    pub pg_thetapr1_delta: Gt,
    pub pg_thetapr2_gen2: Gt,
    pub pg_rr_yy: Gt,
    pub pg_psi0_yy: Gt,
    pub pg_ssprime_gen2: Gt,
    pub pg_varsigpr2_ww1: Gt,
    pub pg_psi0_ww1: Gt,
    pub pg_thetapr2_ww2: Gt,
    pub pg_psi0_ww2: Gt,
    pub pg_gen1_zz: Gt,
    pub pg_rr_tt: Gt,
    pub pg_rr_psi1: Gt,
    pub pg_psi0_tt: Gt,
    pub pg_psi0_psi1: Gt,
    pub pg_gen1_gen2: Gt,
}

pub struct SpendWitness {
    pub sk_u: SecretKeyUser,
    pub v: Scalar,
    pub r: Scalar,
    pub r1: Scalar,
    pub r2: Scalar,
    pub r_varsig1: Scalar,
    pub r_theta1: Scalar,
    pub r_varsig2: Scalar,
    pub r_theta2: Scalar,
    pub r_rr: Scalar,
    pub r_ss: Scalar,
    pub r_tt: Scalar,
    pub rho1: Scalar,
    pub rho2: Scalar,
    pub rho3: Scalar,
}

#[derive(Debug, Clone)]
pub struct SpendProof {}

impl SpendProof {
    pub fn construct(
        params: &Parameters,
        instance: &SpendInstance,
        witness: &SpendWitness) -> Self {
        SpendProof {}
    }
}



