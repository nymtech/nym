use bls12_381::{G1Projective, G2Prepared, G2Projective, Scalar};
use group::{Curve, GroupEncoding};

use crate::error::{DivisibleEcashError, Result};
use crate::proofs::proof_withdrawal::{WithdrawalReqInstance, WithdrawalReqProof, WithdrawalReqWitness};
use crate::scheme::keygen::{PublicKeyUser, SecretKeyAuth, SecretKeyUser, VerificationKeyAuth};
use crate::scheme::PartialWallet;
use crate::scheme::setup::{GroupParameters, Parameters};
use crate::utils::{BlindedSignature, check_bilinear_pairing, hash_g1, Signature};

pub struct WithdrawalRequest {
    com_hash: G1Projective,
    com: G1Projective,
    pc_coms: Vec<G1Projective>,
    zk_proof: WithdrawalReqProof,
}

pub struct RequestInfo {
    com_hash: G1Projective,
    pc_coms_openings: Vec<Scalar>,
    v: Scalar,
}

pub fn withdrawal_request(params: Parameters, sk_user: SecretKeyUser) -> Result<(WithdrawalRequest, RequestInfo)> {
    let grp = params.get_grp();
    let g1 = grp.gen1();
    let params_u = params.get_params_u();
    let v = grp.random_scalar();
    let attributes = vec![sk_user.sk, v];
    let com_opening = grp.random_scalar();
    let commitment = g1 * com_opening
        + attributes
        .iter()
        .zip(params_u.get_gammas())
        .map(|(&m, gamma)| gamma * m)
        .sum::<G1Projective>();

    // Value h in the paper
    let com_hash = hash_g1(commitment.to_bytes());
    let pc_coms_openings = grp.n_random_scalars(attributes.len());
    // Compute Pedersen commitment for each attribute
    let pc_coms = pc_coms_openings
        .iter()
        .zip(attributes.iter())
        .map(|(o_j, m_j)| g1 * o_j + com_hash * m_j)
        .collect::<Vec<_>>();


    // construct a zk proof of knowledge proving possession of m1, m2, o, o1, o2
    let instance = WithdrawalReqInstance {
        com: commitment,
        h: com_hash,
        pc_coms: pc_coms.clone(),
        pk_user: sk_user.public_key(&params.get_grp()),
    };

    let witness = WithdrawalReqWitness {
        attributes,
        com_opening,
        pc_coms_openings: pc_coms_openings.clone(),
    };

    let zk_proof = WithdrawalReqProof::construct(&params, &instance, &witness);

    let req = WithdrawalRequest {
        com_hash,
        com: commitment,
        pc_coms: pc_coms.clone(),
        zk_proof,
    };

    let req_info = RequestInfo {
        com_hash,
        pc_coms_openings,
        v,
    };

    Ok((req, req_info))
}

pub(crate) fn issue(params: &Parameters, req: WithdrawalRequest, pk_u: PublicKeyUser, sk_a: SecretKeyAuth) -> Result<BlindedSignature> {
    let h = hash_g1(req.com.to_bytes());
    if !(h == req.com_hash) {
        return Err(DivisibleEcashError::WithdrawalRequestVerification(
            "Failed to verify the commitment hash".to_string(),
        ));
    }

    // verify zk proof
    let instance = WithdrawalReqInstance {
        com: req.com,
        h: req.com_hash,
        pc_coms: req.pc_coms.clone(),
        pk_user: pk_u,
    };
    if !req.zk_proof.verify(&params, &instance) {
        return Err(DivisibleEcashError::WithdrawalRequestVerification(
            "Failed to verify the proof of knowledge".to_string(),
        ));
    }

    let sig = req
        .pc_coms
        .iter()
        .zip(sk_a.ys.iter())
        .map(|(pc, yi)| pc * yi)
        .chain(std::iter::once(h * sk_a.x))
        .sum();

    Ok(BlindedSignature(h, sig))
}

pub(crate) fn issue_verify(
    params: &GroupParameters,
    vk_auth: &VerificationKeyAuth,
    sk_user: &SecretKeyUser,
    blind_signature: &BlindedSignature,
    req_info: &RequestInfo) -> Result<PartialWallet> {

    // Parse the blinded signature
    let h = blind_signature.0;
    let c = blind_signature.1;

    // Verify the integrity of the response from the authority
    if !(req_info.com_hash == h) {
        return Err(DivisibleEcashError::IssuanceVfy(
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

    let attr = vec![sk_user.sk, req_info.v];

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
        return Err(DivisibleEcashError::IssuanceVfy(
            "Verification of wallet share failed".to_string(),
        ));
    }

    Ok(PartialWallet {
        sig: Signature(h, unblinded_c),
        v: req_info.v,
        idx: None,
    })
}