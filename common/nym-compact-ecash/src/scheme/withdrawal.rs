use bls12_381::{G1Projective, Scalar};
use group::GroupEncoding;

use crate::error::{CompactEcashError, Result};
use crate::proofs::{WithdrawalReqInstance, WithdrawalReqProof, WithdrawalReqWitness};
use crate::scheme::BlindedSignature;
use crate::scheme::keygen::{PublicKeyUser, SecretKeyAuth, SecretKeyUser};
use crate::scheme::keygen::ttp_keygen;
use crate::scheme::setup::Parameters;
use crate::utils::hash_g1;

pub struct WithdrawalRequest {
    commitment_hash: G1Projective,
    attrs_commitment: G1Projective,
    pc_commitments: Vec<G1Projective>,
    zk_proof: WithdrawalReqProof,
}

pub struct RequestInfo {
    commitment_hash: G1Projective,
    attrs_commitment_opening: Scalar,
    pc_openings: Vec<Scalar>,
    v: Scalar,
    t: Scalar,
}

pub fn withdrawal_request(
    params: &Parameters,
    sk_user: &SecretKeyUser,
) -> Result<(WithdrawalRequest, RequestInfo)> {
    let v = params.random_scalar();
    let t = params.random_scalar();

    let attributes = vec![sk_user.sk, v, t];
    let gammas = params.gammas();
    let attrs_commitment_opening = params.random_scalar();
    let attrs_commitment = attributes
        .iter()
        .zip(gammas)
        .map(|(&m, gamma)| gamma * m)
        .sum::<G1Projective>();

    let attrs_commitment_hash = hash_g1(attrs_commitment.to_bytes());

    let pc_openings = params.n_random_scalars(attributes.len());

    // Compute Pedersen commitment for each attribute
    let pc_commitments = pc_openings
        .iter()
        .zip(attributes.iter())
        .map(|(o_j, m_j)| params.gen1() * o_j + attrs_commitment_hash * m_j)
        .collect::<Vec<_>>();

    // construct a zk proof of knowledge proving possession of m1, m2, m3, o, o1, o2, o3
    let instance = WithdrawalReqInstance {
        g1_gen: *params.gen1(),
        gammas: gammas.clone(),
        attrs_commitment,
        attrs_commitment_hash,
        pc_commitments: pc_commitments.clone(),
        pk_user: PublicKeyUser {
            pk: params.gen1() * sk_user.sk,
        },
    };

    let witness = WithdrawalReqWitness {
        attributes,
        attrs_commitment_opening,
        pc_openings: pc_openings.clone(),
    };

    let zk_proof = WithdrawalReqProof::construct(&params, &instance, &witness);

    let req = WithdrawalRequest {
        commitment_hash: attrs_commitment_hash,
        attrs_commitment,
        pc_commitments: pc_commitments.clone(),
        zk_proof,
    };

    let req_info = RequestInfo {
        commitment_hash: attrs_commitment_hash,
        attrs_commitment_opening,
        pc_openings: pc_openings.clone(),
        v,
        t,
    };

    Ok((req, req_info))
}

pub fn issue_wallet(
    params: &Parameters,
    sk_auth: SecretKeyAuth,
    pk_user: PublicKeyUser,
    withdrawal_req: &WithdrawalRequest,
) -> Result<BlindedSignature> {
    let h = hash_g1(withdrawal_req.attrs_commitment.to_bytes());
    if !(h == withdrawal_req.commitment_hash) {
        return Err(CompactEcashError::WithdrawalRequestVerification(
            "Failed to verify the commitment hash".to_string(),
        ));
    }

    // verify zk proof
    let instance = WithdrawalReqInstance {
        g1_gen: *params.gen1(),
        gammas: params.gammas().clone(),
        attrs_commitment: withdrawal_req.attrs_commitment,
        attrs_commitment_hash: withdrawal_req.commitment_hash,
        pc_commitments: withdrawal_req.pc_commitments.clone(),
        pk_user,
    };
    if !withdrawal_req.zk_proof.verify(&instance) {
        return Err(CompactEcashError::WithdrawalRequestVerification(
            "Failed to verify the proof of knowledge".to_string(),
        ));
    }

    let sig = withdrawal_req
        .pc_commitments
        .iter()
        .zip(sk_auth.ys.iter())
        .map(|(pc, yi)| pc * yi)
        .chain(std::iter::once(h * sk_auth.x))
        .sum();

    Ok(BlindedSignature(h, sig))
}


#[cfg(test)]
mod tests {
    use super::*;
}
