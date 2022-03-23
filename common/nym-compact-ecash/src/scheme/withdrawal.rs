use bls12_381::{G1Projective, G2Prepared, G2Projective, Scalar};
use group::{Curve, GroupEncoding};

use crate::error::{CompactEcashError, Result};
use crate::proofs::{WithdrawalReqInstance, WithdrawalReqProof, WithdrawalReqWitness};
use crate::scheme::{BlindedSignature, Signature, Wallet};
use crate::scheme::keygen::{PublicKeyUser, SecretKeyAuth, SecretKeyUser, VerificationKeyAuth};
use crate::scheme::keygen::ttp_keygen;
use crate::scheme::setup::Parameters;
use crate::utils::{check_bilinear_pairing, hash_g1};

pub struct WithdrawalRequest {
    com_hash: G1Projective,
    com: G1Projective,
    pc_coms: Vec<G1Projective>,
    zk_proof: WithdrawalReqProof,
}

pub struct RequestInfo {
    com_hash: G1Projective,
    com_opening: Scalar,
    pc_coms_openings: Vec<Scalar>,
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
    let com_opening = params.random_scalar();
    let com = params.gen1() * com_opening + attributes
        .iter()
        .zip(gammas)
        .map(|(&m, gamma)| gamma * m)
        .sum::<G1Projective>();

    // Value h in the paper
    let com_hash = hash_g1(com.to_bytes());

    let pc_coms_openings = params.n_random_scalars(attributes.len());

    // Compute Pedersen commitment for each attribute
    let pc_coms = pc_coms_openings
        .iter()
        .zip(attributes.iter())
        .map(|(o_j, m_j)| params.gen1() * o_j + com_hash * m_j)
        .collect::<Vec<_>>();

    // construct a zk proof of knowledge proving possession of m1, m2, m3, o, o1, o2, o3
    let instance = WithdrawalReqInstance {
        com,
        h: com_hash,
        pc_coms: pc_coms.clone(),
        pk_user: PublicKeyUser { pk: params.gen1() * sk_user.sk },
    };

    let witness = WithdrawalReqWitness {
        attributes,
        com_opening,
        pc_coms_openings: pc_coms_openings.clone(),
    };

    let zk_proof = WithdrawalReqProof::construct(&params, &instance, &witness);

    let req = WithdrawalRequest {
        com_hash,
        com,
        pc_coms: pc_coms.clone(),
        zk_proof,
    };

    let req_info = RequestInfo {
        com_hash,
        com_opening,
        pc_coms_openings: pc_coms_openings.clone(),
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
    let h = hash_g1(withdrawal_req.com.to_bytes());
    if !(h == withdrawal_req.com_hash) {
        return Err(CompactEcashError::WithdrawalRequestVerification(
            "Failed to verify the commitment hash".to_string(),
        ));
    }

    // verify zk proof
    let instance = WithdrawalReqInstance {
        com: withdrawal_req.com,
        h: withdrawal_req.com_hash,
        pc_coms: withdrawal_req.pc_coms.clone(),
        pk_user,
    };
    if !withdrawal_req.zk_proof.verify(&params, &instance) {
        return Err(CompactEcashError::WithdrawalRequestVerification(
            "Failed to verify the proof of knowledge".to_string(),
        ));
    }

    let sig = withdrawal_req
        .pc_coms
        .iter()
        .zip(sk_auth.ys.iter())
        .map(|(pc, yi)| pc * yi)
        .chain(std::iter::once(h * sk_auth.x))
        .sum();

    Ok(BlindedSignature(h, sig))
}


pub fn issue_verify(params: &Parameters, vk_auth: &VerificationKeyAuth, sk_user: &SecretKeyUser, blind_signature: &BlindedSignature, req_info: &RequestInfo) -> Result<Wallet> {
    // Parse the blinded signature
    let h = blind_signature.0;
    let c = blind_signature.1;

    // Verify the integrity of the reponse from the authority
    if !(req_info.com_hash == h) {
        return Err(CompactEcashError::IssuanceVfy(
            "Failed to verify the proof of knowledge".to_string(),
        ));
    }

    // Unblind the blinded signature
    let blinding_removers = vk_auth
        .beta_g1
        .iter()
        .zip(req_info.pc_coms_openings.iter())
        .map(|(beta, opening)| beta * opening)
        .sum::<G1Projective>();

    let unblinded_c = c - blinding_removers;

    // Verify the signature correctness on the wallet share
    let attr = vec![sk_user.sk, req_info.v];

    let signed_attributes = attr
        .iter()
        .zip(vk_auth.beta_g2.iter())
        .map(|(attr, beta_i)| beta_i * attr)
        .sum::<G2Projective>();

    if !check_bilinear_pairing(
        &h.to_affine(),
        &G2Prepared::from((vk_auth.alpha + signed_attributes).to_affine()),
        &unblinded_c.to_affine(),
        params.prepared_miller_g2(),
    ) {
        return Err(CompactEcashError::IssuanceVfy(
            "Verification of wallet share failed".to_string(),
        ));
    }


    Ok(Wallet {
        sig: Signature(h,
                       unblinded_c),
        v: req_info.v,
        idx: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
}
