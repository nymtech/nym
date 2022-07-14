use bls12_381::{G1Projective, G2Prepared, G2Projective, Scalar};
use group::{Curve, GroupEncoding};

use crate::error::{CompactEcashError, Result};
use crate::proofs::proof_withdrawal::{
    WithdrawalReqInstance, WithdrawalReqProof, WithdrawalReqWitness,
};
use crate::scheme::keygen::{PublicKeyUser, SecretKeyAuth, SecretKeyUser, VerificationKeyAuth};
use crate::scheme::keygen::ttp_keygen;
use crate::scheme::PartialWallet;
use crate::scheme::setup::{GroupParameters, Parameters};
use crate::utils::{check_bilinear_pairing, hash_g1};
use crate::utils::{BlindedSignature, Signature};

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

impl RequestInfo {
    pub fn get_com(&self) -> G1Projective {
        self.com_hash
    }
    pub fn get_com_openings(&self) -> Scalar {
        self.com_opening
    }
    pub fn get_pc_coms_openings(&self) -> &Vec<Scalar> {
        &self.pc_coms_openings
    }
    pub fn get_v(&self) -> Scalar {
        self.v
    }
    pub fn get_t(&self) -> Scalar {
        self.t
    }
}

pub fn withdrawal_request(
    params: &GroupParameters,
    sk_user: &SecretKeyUser,
) -> Result<(WithdrawalRequest, RequestInfo)> {
    let v = params.random_scalar();
    let t = params.random_scalar();

    let attributes = vec![sk_user.sk, v, t];
    let gammas = params.gammas();
    let com_opening = params.random_scalar();
    let com = params.gen1() * com_opening
        + attributes
        .iter()
        .zip(gammas)
        .map(|(&m, gamma)| gamma * m)
        .sum::<G1Projective>();

    // Value h in the paper
    let com_hash = hash_g1(com.to_bytes());

    // For each private attribute we compute a pedersen commitment
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
        pk_user: PublicKeyUser {
            pk: params.gen1() * sk_user.sk,
        },
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
    params: &GroupParameters,
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

pub fn issue_verify(
    params: &GroupParameters,
    vk_auth: &VerificationKeyAuth,
    sk_user: &SecretKeyUser,
    blind_signature: &BlindedSignature,
    req_info: &RequestInfo,
) -> Result<PartialWallet> {
    // Parse the blinded signature
    let h = blind_signature.0;
    let c = blind_signature.1;

    // Verify the integrity of the response from the authority
    if !(req_info.com_hash == h) {
        return Err(CompactEcashError::IssuanceVfy(
            "Integrity verification failed".to_string(),
        ));
    }

    // Unblind the blinded signature on the partial wallet
    let blinding_removers = vk_auth
        .beta_g1
        .iter()
        .zip(req_info.pc_coms_openings.iter())
        .map(|(beta, opening)| beta * opening)
        .sum::<G1Projective>();

    let unblinded_c = c - blinding_removers;

    let attr = vec![sk_user.sk, req_info.v, req_info.t];

    let signed_attributes = attr
        .iter()
        .zip(vk_auth.beta_g2.iter())
        .map(|(attr, beta_i)| beta_i * attr)
        .sum::<G2Projective>();

    // Verify the signature correctness on the wallet share
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

    Ok(PartialWallet {
        sig: Signature(h, unblinded_c),
        v: req_info.v,
        idx: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
}
