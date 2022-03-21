use bls12_381::{G1Projective, Scalar};
use group::{Curve, GroupEncoding};

use nymcoconut::utils::hash_g1;

use crate::error::{CompactEcashError, Result};
use crate::proofs::WithdrawalProof;
use crate::scheme::BlindedSignature;
use crate::scheme::keygen::{PublicKeyUser, SecretKeyAuth, SecretKeyUser};
use crate::scheme::setup::Parameters;

pub struct WithdrawalRequest {
    commitment_hash: G1Projective,
    attrs_commitment: G1Projective,
    pc_commitments: Vec<G1Projective>,
    zk_proof: WithdrawalProof,
}

pub struct RequestInfo {
    commitment_hash: G1Projective,
    attrs_commitment_opening: Scalar,
    pc_openings: Vec<Scalar>,
    v: Scalar,
    t: Scalar,
}

pub fn withdrawal_request(params: &Parameters, skUser: SecretKeyUser) {
    let v = params.random_scalar();
    let t = params.random_scalar();

    let attributes = [skUser.sk, v, t];
    let gammas = params.gammas();
    let attrs_commitment_opening = params.random_scalar();
    let attribute_commitment = attributes
        .iter()
        .zip(gammas).map(|(&m, gamma)| gamma * m)
        .sum::<G1Projective>();

    let commitment_hash = hash_g1(attribute_commitment.to_bytes());

    let pc_openings = params.n_random_scalars(attributes.len());

    // Compute Pedersen commitment for each attribute
    let pc_attributes = pc_openings
        .iter()
        .zip(attributes.iter())
        .map(|(o_j, m_j)| params.gen1() * o_j + commitment_hash * m_j)
        .collect::<Vec<_>>();

    // construct a zk proof of knowledge proving possession of m1, m2, m3, o, o1, o2, o3

    let req_info = RequestInfo {
        commitment_hash,
        attrs_commitment_opening,
        pc_openings,
        v,
        t,
    };
}

pub fn issue(params: &Parameters, skAuth: SecretKeyAuth, pkUser: PublicKeyUser, withReq: &WithdrawalRequest) -> Result<BlindedSignature> {
    let h = hash_g1(withReq.attrs_commitment.to_bytes());
    if !(h == withReq.commitment_hash) {
        return Err(CompactEcashError::WithdrawalVerification(
            "Failed to verify the commitment hash".to_string(),
        ));
    }

    let sig = withReq.pc_commitments
        .iter()
        .zip(skAuth.ys.iter())
        .map(|(pc, yi)| pc * yi)
        .chain(std::iter::once(h * skAuth.x)).sum();
    // verify zk proof

    Ok(BlindedSignature(h, sig))
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn withdrawal_request() {
        let params = Parameters::new().unwrap();
    }
}