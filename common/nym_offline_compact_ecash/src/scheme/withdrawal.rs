// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::common_types::{BlindedSignature, Signature, SignerIndex};
use crate::error::{CompactEcashError, Result};
use crate::proofs::proof_withdrawal::{
    WithdrawalReqInstance, WithdrawalReqProof, WithdrawalReqWitness,
};
use crate::scheme::keygen::{PublicKeyUser, SecretKeyAuth, SecretKeyUser, VerificationKeyAuth};
use crate::scheme::setup::GroupParameters;
use crate::scheme::PartialWallet;
use crate::utils::{check_bilinear_pairing, hash_g1};
use crate::{constants, ecash_group_parameters, Attribute};
use bls12_381::{multi_miller_loop, G1Projective, G2Prepared, G2Projective, Scalar};
use group::{Curve, Group, GroupEncoding};
use serde::{Deserialize, Serialize};
use std::ops::Neg;

/// Represents a withdrawal request generate by the client who wants to obtain a zk-nym credential.
///
/// This struct encapsulates the necessary components for a withdrawal request, including the joined commitment hash, the joined commitment,
/// individual Pedersen commitments for private attributes, and a zero-knowledge proof for the withdrawal request.
///
/// # Fields
///
/// * `joined_commitment_hash` - The joined commitment hash represented as a G1Projective element.
/// * `joined_commitment` - The joined commitment represented as a G1Projective element.
/// * `private_attributes_commitments` - A vector of individual Pedersen commitments for private attributes represented as G1Projective elements.
/// * `zk_proof` - The zero-knowledge proof for the withdrawal request.
///
/// # Derives
///
/// The struct derives `Debug` and `PartialEq` to provide debug output and basic comparison functionality.
///
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct WithdrawalRequest {
    joined_commitment_hash: G1Projective,
    joined_commitment: G1Projective,
    private_attributes_commitments: Vec<G1Projective>,
    zk_proof: WithdrawalReqProof,
}

impl WithdrawalRequest {
    pub fn get_private_attributes_commitments(&self) -> &[G1Projective] {
        &self.private_attributes_commitments
    }
}

/// Represents information associated with a withdrawal request.
///
/// This structure holds the commitment hash, commitment opening, private attributes openings,
/// the wallet secret (scalar), and the expiration date related to a withdrawal request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestInfo {
    joined_commitment_hash: G1Projective,
    joined_commitment_opening: Scalar,
    private_attributes_openings: Vec<Scalar>,
    wallet_secret: Scalar,
    expiration_date: Scalar,
}

impl RequestInfo {
    pub fn get_joined_commitment_hash(&self) -> &G1Projective {
        &self.joined_commitment_hash
    }
    pub fn get_joined_commitment_opening(&self) -> &Scalar {
        &self.joined_commitment_opening
    }
    pub fn get_private_attributes_openings(&self) -> &[Scalar] {
        &self.private_attributes_openings
    }
    pub fn get_v(&self) -> &Scalar {
        &self.wallet_secret
    }
    pub fn get_expiration_date(&self) -> &Scalar {
        &self.expiration_date
    }
}

/// Computes Pedersen commitments for private attributes.
///
/// Given a set of private attributes and the commitment hash for all attributes,
/// this function generates random blinding factors (`openings`) and computes corresponding
/// Pedersen commitments for each private attribute.
/// Pedersen commitments have the hiding and binding properties, providing a secure way
/// to represent private values in a commitment scheme.
///
/// # Arguments
///
/// * `params` - Group parameters for the cryptographic group.
/// * `joined_commitment_hash` - The commitment hash to be used in the Pedersen commitments.
/// * `private_attributes` - A slice of private attributes to be committed.
///
/// # Returns
///
/// A tuple containing vectors of blinding factors (`openings`) and corresponding
/// Pedersen commitments for each private attribute.
fn compute_private_attribute_commitments(
    params: &GroupParameters,
    joined_commitment_hash: &G1Projective,
    private_attributes: &[Scalar],
) -> (Vec<Scalar>, Vec<G1Projective>) {
    let (openings, commitments): (Vec<Scalar>, Vec<G1Projective>) = private_attributes
        .iter()
        .map(|m_j| {
            let o_j = params.random_scalar();
            (o_j, params.gen1() * o_j + joined_commitment_hash * m_j)
        })
        .unzip();

    (openings, commitments)
}

/// Generates a withdrawal request for the given user to request a zk-nym credential wallet.
///
/// # Arguments
///
/// * `sk_user` - A reference to the user's secret key.
/// * `expiration_date` - The expiration date for the withdrawal request.
///
/// # Returns
///
/// A tuple containing the generated `WithdrawalRequest` and `RequestInfo`, or an error if the operation fails.
///
/// # Details
///
/// The function starts by generating a random, unique wallet secret `v` and computing the joined commitment for all attributes,
/// including public (expiration date) and private ones (user secret key and wallet secret).
/// It then calculates the commitment hash (`joined_commitment_hash`) and computes Pedersen commitments for private attributes.
/// A zero-knowledge proof of knowledge is constructed to prove possession of specific attributes.
///
/// The resulting `WithdrawalRequest` includes the commitment hash, joined commitment, commitments for private
/// attributes, and the constructed zero-knowledge proof.
///
/// The associated `RequestInfo` includes information such as commitment hash, commitment opening,
/// openings for private attributes, `v`, and the expiration date.
pub fn withdrawal_request(
    sk_user: &SecretKeyUser,
    expiration_date: u64,
) -> Result<(WithdrawalRequest, RequestInfo)> {
    let params = ecash_group_parameters();
    // Generate random and unique wallet secret
    let v = params.random_scalar();
    let joined_commitment_opening = params.random_scalar();
    // Compute joined commitment for all attributes (public and private)
    //SAFETY: params is static with length 3
    #[allow(clippy::unwrap_used)]
    let joined_commitment: G1Projective = params.gen1() * joined_commitment_opening
        + params.gamma_idx(0).unwrap() * sk_user.sk
        + params.gamma_idx(1).unwrap() * v;

    // Compute commitment hash h
    #[allow(clippy::unwrap_used)]
    let joined_commitment_hash = hash_g1(
        (joined_commitment + params.gamma_idx(2).unwrap() * Scalar::from(expiration_date))
            .to_bytes(),
    );

    // Compute Pedersen commitments for private attributes (wallet secret and user's secret)
    let private_attributes = vec![sk_user.sk, v];
    let (private_attributes_openings, private_attributes_commitments) =
        compute_private_attribute_commitments(params, &joined_commitment_hash, &private_attributes);

    // construct a NIZK proof of knowledge proving possession of m1, m2, o, o1, o2
    let instance = WithdrawalReqInstance {
        joined_commitment,
        joined_commitment_hash,
        private_attributes_commitments: private_attributes_commitments.clone(),
        pk_user: PublicKeyUser {
            pk: params.gen1() * sk_user.sk,
        },
    };

    let witness = WithdrawalReqWitness {
        private_attributes,
        joined_commitment_opening,
        private_attributes_openings: private_attributes_openings.clone(),
    };
    let zk_proof = WithdrawalReqProof::construct(&instance, &witness);

    // Create and return WithdrawalRequest and RequestInfo
    Ok((
        WithdrawalRequest {
            joined_commitment_hash,
            joined_commitment,
            private_attributes_commitments,
            zk_proof,
        },
        RequestInfo {
            joined_commitment_hash,
            joined_commitment_opening,
            private_attributes_openings: private_attributes_openings.clone(),
            wallet_secret: v,
            expiration_date: Scalar::from(expiration_date),
        },
    ))
}

/// Verifies the integrity of a withdrawal request, including the joined commitment hash
/// and the zero-knowledge proof of knowledge.
///
/// # Arguments
///
/// * `req` - The withdrawal request to be verified.
/// * `pk_user` - Public key of the user associated with the withdrawal request.
/// * `expiration_date` - Expiration date for the withdrawal request.
///
/// # Returns
///
/// Returns `Ok(true)` if the verification is successful, otherwise returns an error
/// with a specific message indicating the verification failure.
pub fn request_verify(
    req: &WithdrawalRequest,
    pk_user: PublicKeyUser,
    expiration_date: u64,
) -> Result<()> {
    let params = ecash_group_parameters();
    // Verify the joined commitment hash
    //SAFETY: params is static with length 3
    #[allow(clippy::unwrap_used)]
    let expected_commitment_hash = hash_g1(
        (req.joined_commitment + params.gamma_idx(2).unwrap() * Scalar::from(expiration_date))
            .to_bytes(),
    );
    if req.joined_commitment_hash != expected_commitment_hash {
        return Err(CompactEcashError::WithdrawalRequestVerification);
    }
    // Verify zk proof
    let instance = WithdrawalReqInstance {
        joined_commitment: req.joined_commitment,
        joined_commitment_hash: req.joined_commitment_hash,
        private_attributes_commitments: req.private_attributes_commitments.clone(),
        pk_user,
    };
    if !req.zk_proof.verify(&instance) {
        return Err(CompactEcashError::WithdrawalRequestVerification);
    }
    Ok(())
}

/// Signs an expiration date using a joined commitment hash and a secret key.
///
/// Given a joined commitment hash (`joined_commitment_hash`), an expiration date (`expiration_date`),
/// and a secret key for authentication (`sk_auth`), this function computes the signature of the
/// expiration date by multiplying the commitment hash with the blinding factor derived from the secret key
/// and the expiration date.
///
/// # Arguments
///
/// * `joined_commitment_hash` - The G1Projective point representing the joined commitment hash.
/// * `expiration_date` - The expiration date timestamp to be signed.
/// * `sk_auth` - The secret key of the signing authority. Assumes key is long enough.
///
/// # Returns
///
/// A `Result` containing the resulting G1Projective point if successful, or an error if the
/// authentication secret key index is out of bounds.
fn sign_expiration_date(
    joined_commitment_hash: &G1Projective,
    expiration_date: u64,
    sk_auth: &SecretKeyAuth,
) -> G1Projective {
    //SAFETY : this fn assumes a long enough key
    #[allow(clippy::unwrap_used)]
    let yi = sk_auth.get_y_by_idx(2).unwrap();
    joined_commitment_hash * (yi * Scalar::from(expiration_date))
}

/// Issues a blinded signature for a withdrawal request, after verifying its integrity.
///
/// This function first verifies the withdrawal request using the provided group parameters,
/// user's public key, and expiration date. If the verification is successful,
/// the function proceeds to blind sign the private attributes and sign the expiration date,
/// combining both signatures into a final signature.
///
/// # Arguments
///
/// * `sk_auth` - Secret key of the signing authority.
/// * `pk_user` - Public key of the user associated with the withdrawal request.
/// * `withdrawal_req` - The withdrawal request to be signed.
/// * `expiration_date` - Expiration date for the withdrawal request.
///
/// # Returns
///
/// Returns a `BlindedSignature` if the issuance process is successful, otherwise returns an error
/// with a specific message indicating the failure.
pub fn issue(
    sk_auth: &SecretKeyAuth,
    pk_user: PublicKeyUser,
    withdrawal_req: &WithdrawalRequest,
    expiration_date: u64,
) -> Result<BlindedSignature> {
    // Verify the withdrawal request
    request_verify(withdrawal_req, pk_user, expiration_date)?;
    // Verify `sk_auth` is long enough
    if sk_auth.ys.len() < constants::ATTRIBUTES_LEN {
        return Err(CompactEcashError::KeyTooShort);
    }
    // Blind sign the private attributes
    let blind_signatures: G1Projective = withdrawal_req
        .private_attributes_commitments
        .iter()
        .zip(sk_auth.ys.iter().take(2))
        .map(|(pc, yi)| pc * yi)
        .sum();
    // Sign the expiration date
    //SAFETY: key length was verified before
    let expiration_date_sign = sign_expiration_date(
        &withdrawal_req.joined_commitment_hash,
        expiration_date,
        sk_auth,
    );
    // Combine both signatures
    let signature =
        blind_signatures + withdrawal_req.joined_commitment_hash * sk_auth.x + expiration_date_sign;

    Ok(BlindedSignature {
        h: withdrawal_req.joined_commitment_hash,
        c: signature,
    })
}

/// Verifies the integrity and correctness of a blinded signature
/// and returns an unblinded partial zk-nym wallet.
///
/// This function first verifies the integrity of the received blinded signature by checking
/// if the joined commitment hash matches the one provided in the `req_info`. If the verification
/// is successful, it proceeds to unblind the blinded signature and verify its correctness.
///
/// # Arguments
///
/// * `vk_auth` - Verification key of the signing authority.
/// * `sk_user` - Secret key of the user.
/// * `blind_signature` - Blinded signature received from the authority.
/// * `req_info` - Information associated with the request, including the joined commitment hash,
///                private attributes openings, v, and expiration date.
///
/// # Returns
///
/// Returns a `PartialWallet` if the verification process is successful, otherwise returns an error
/// with a specific message indicating the failure.
pub fn issue_verify(
    vk_auth: &VerificationKeyAuth,
    sk_user: &SecretKeyUser,
    blind_signature: &BlindedSignature,
    req_info: &RequestInfo,
    signer_index: SignerIndex,
) -> Result<PartialWallet> {
    let params = ecash_group_parameters();
    // Verify the integrity of the response from the authority
    if req_info.joined_commitment_hash != blind_signature.h {
        return Err(CompactEcashError::IssuanceVerification);
    }

    // Unblind the blinded signature on the partial signature
    let blinding_removers = vk_auth
        .beta_g1
        .iter()
        .zip(&req_info.private_attributes_openings)
        .map(|(beta, opening)| beta * opening)
        .sum::<G1Projective>();
    let unblinded_c = blind_signature.c - blinding_removers;

    let attr = [sk_user.sk, req_info.wallet_secret, req_info.expiration_date];

    let signed_attributes = attr
        .iter()
        .zip(vk_auth.beta_g2.iter())
        .map(|(attr, beta_i)| beta_i * attr)
        .sum::<G2Projective>();

    // Verify the signature correctness on the wallet share
    if !check_bilinear_pairing(
        &blind_signature.h.to_affine(),
        &G2Prepared::from((vk_auth.alpha + signed_attributes).to_affine()),
        &unblinded_c.to_affine(),
        params.prepared_miller_g2(),
    ) {
        return Err(CompactEcashError::IssuanceVerification);
    }

    Ok(PartialWallet {
        sig: Signature {
            h: blind_signature.h,
            s: unblinded_c,
        },
        v: req_info.wallet_secret,
        idx: signer_index,
        expiration_date: req_info.expiration_date,
    })
}

/// Verifies a partial blind signature using the provided parameters and validator's verification key.
///
/// # Arguments
///
/// * `blind_sign_request` - A reference to the blind signature request signed by the client.
/// * `public_attributes` - A reference to the public attributes included in the client's request.
/// * `blind_sig` - A reference to the issued partial blinded signature to be verified.
/// * `partial_verification_key` - A reference to the validator's partial verification key.
///
/// # Returns
///
/// A boolean indicating whether the partial blind signature is valid (`true`) or not (`false`).
///
/// # Remarks
///
/// This function verifies the correctness and validity of a partial blind signature using
/// the provided cryptographic parameters, blind signature request, blinded signature,
/// and partial verification key.
/// It calculates pairings based on the provided values and checks whether the partial blind signature
/// is consistent with the verification key and commitments in the blind signature request.
/// The function returns `true` if the partial blind signature is valid, and `false` otherwise.
pub fn verify_partial_blind_signature(
    private_attribute_commitments: &[G1Projective],
    public_attributes: &[&Attribute],
    blind_sig: &BlindedSignature,
    partial_verification_key: &VerificationKeyAuth,
) -> bool {
    let params = ecash_group_parameters();
    let num_private_attributes = private_attribute_commitments.len();
    if num_private_attributes + public_attributes.len() > partial_verification_key.beta_g2.len() {
        return false;
    }

    // TODO: we're losing some memory here due to extra allocation,
    // but worst-case scenario (given SANE amount of attributes), it's just few kb at most
    let c_neg = blind_sig.c.to_affine().neg();
    let g2_prep = params.prepared_miller_g2();

    let mut terms = vec![
        // (c^{-1}, g2)
        (c_neg, g2_prep.clone()),
        // (s, alpha)
        (
            blind_sig.h.to_affine(),
            G2Prepared::from(partial_verification_key.alpha.to_affine()),
        ),
    ];

    // for each private attribute, add (cm_i, beta_i) to the miller terms
    for (private_attr_commit, beta_g2) in private_attribute_commitments
        .iter()
        .zip(&partial_verification_key.beta_g2)
    {
        // (cm_i, beta_i)
        terms.push((
            private_attr_commit.to_affine(),
            G2Prepared::from(beta_g2.to_affine()),
        ))
    }

    // for each public attribute, add (s^pub_j, beta_{priv + j}) to the miller terms
    for (&pub_attr, beta_g2) in public_attributes.iter().zip(
        partial_verification_key
            .beta_g2
            .iter()
            .skip(num_private_attributes),
    ) {
        // (s^pub_j, beta_j)
        terms.push((
            (blind_sig.h * pub_attr).to_affine(),
            G2Prepared::from(beta_g2.to_affine()),
        ))
    }

    // get the references to all the terms to get the arguments the miller loop expects
    #[allow(clippy::map_identity)]
    let terms_refs = terms.iter().map(|(g1, g2)| (g1, g2)).collect::<Vec<_>>();

    // since checking whether e(a, b) == e(c, d)
    // is equivalent to checking e(a, b) • e(c, d)^{-1} == id
    // and thus to e(a, b) • e(c^{-1}, d) == id
    //
    // compute e(c^{-1}, g2) • e(s, alpha) • e(cm_0, beta_0) • e(cm_i, beta_i) • (s^pub_0, beta_{i+1}) (s^pub_j, beta_{i + j})
    multi_miller_loop(&terms_refs)
        .final_exponentiation()
        .is_identity()
        .into()
}
