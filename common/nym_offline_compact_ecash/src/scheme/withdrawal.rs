use bls12_381::{G1Projective, G2Prepared, G2Projective, Scalar};
use group::{Curve, GroupEncoding};

use crate::error::{CompactEcashError, Result};
use crate::proofs::proof_withdrawal::{
    WithdrawalReqInstance, WithdrawalReqProof, WithdrawalReqWitness,
};
use crate::scheme::keygen::{PublicKeyUser, SecretKeyAuth, SecretKeyUser, VerificationKeyAuth};
use crate::scheme::setup::GroupParameters;
use crate::scheme::PartialWallet;
use crate::utils::{check_bilinear_pairing, hash_g1, try_deserialize_g1_projective};
use crate::utils::{BlindedSignature, Signature};

#[derive(Debug, PartialEq)]
pub struct WithdrawalRequest {
    joined_commitment_hash: G1Projective,
    joined_commitment: G1Projective,
    private_attributes_commitments: Vec<G1Projective>,
    zk_proof: WithdrawalReqProof,
}

impl WithdrawalRequest {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(
            48 + // joined_commitment_hash
                48 + // joined_commitment
                8 +  // private_attributes_commitments length
                (self.private_attributes_commitments.len() as usize) * 48 + // private attributes commitments
                self.zk_proof.to_bytes().len(), // zk_proof_bytes
        );

        bytes.extend_from_slice(&self.joined_commitment_hash.to_affine().to_compressed());
        bytes.extend_from_slice(&self.joined_commitment.to_affine().to_compressed());

        bytes.extend_from_slice(&(self.private_attributes_commitments.len() as u64).to_le_bytes());
        bytes.extend(
            self.private_attributes_commitments
                .iter()
                .flat_map(|c| c.to_affine().to_compressed()),
        );

        bytes.extend_from_slice(&self.zk_proof.to_bytes());

        bytes
    }
}

impl TryFrom<&[u8]> for WithdrawalRequest {
    type Error = CompactEcashError;

    fn try_from(bytes: &[u8]) -> Result<WithdrawalRequest> {
        let min_length = 48 + 48 + 8 + 48;
        if bytes.len() < min_length {
            return Err(CompactEcashError::DeserializationMinLength {
                min: min_length,
                actual: bytes.len(),
            });
        }

        let mut j = 0;
        let commitment_hash_bytes_len = 48;
        let commitment_bytes_len = 48;

        let com_hash_bytes = bytes[..j + commitment_hash_bytes_len].try_into().unwrap();
        let joined_commitment_hash = try_deserialize_g1_projective(
            &com_hash_bytes,
            CompactEcashError::Deserialization(
                "Failed to deserialize compressed commitment hash".to_string(),
            ),
        )?;
        j += commitment_hash_bytes_len;

        let com_bytes = bytes[j..j + commitment_bytes_len].try_into().unwrap();
        let joined_commitment = try_deserialize_g1_projective(
            &com_bytes,
            CompactEcashError::Deserialization(
                "Failed to deserialize compressed commitment".to_string(),
            ),
        )?;
        j += commitment_bytes_len;

        let pc_len = u64::from_le_bytes(bytes[j..j + 8].try_into().unwrap());
        j += 8;
        if bytes[j..].len() < pc_len as usize * 48 {
            return Err(CompactEcashError::DeserializationMinLength {
                min: pc_len as usize * 48,
                actual: bytes[56..].len(),
            });
        }
        let mut private_attributes_commitments = Vec::with_capacity(pc_len as usize);
        for i in 0..pc_len as usize {
            let start = j + i * 48;
            let end = start + 48;

            let pc_com_bytes = bytes[start..end].try_into().unwrap();
            let pc_com = try_deserialize_g1_projective(
                &pc_com_bytes,
                CompactEcashError::Deserialization(
                    "Failed to deserialize compressed Pedersen commitment".to_string(),
                ),
            )?;

            private_attributes_commitments.push(pc_com)
        }

        let zk_proof = WithdrawalReqProof::try_from(&bytes[j + pc_len as usize * 48..])?;

        Ok(WithdrawalRequest {
            joined_commitment_hash,
            joined_commitment,
            private_attributes_commitments,
            zk_proof,
        })
    }
}

pub struct RequestInfo {
    joined_commitment_hash: G1Projective,
    joined_commitment_opening: Scalar,
    private_attributes_openings: Vec<Scalar>,
    v: Scalar,
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
        &self.v
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
/// * `params` - A reference to the group parameters used in the protocol.
/// * `sk_user` - A reference to the user's secret key.
/// * `expiration_date` - The expiration date for the withdrawal request.
///
/// # Returns
///
/// A tuple containing the generated `WithdrawalRequest` and `RequestInfo`, or an error if the operation fails.
///
/// # Details
///
/// The function starts by generating a random wallet secret `v` and computing the joined commitment for all attributes,
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
    params: &GroupParameters,
    sk_user: &SecretKeyUser,
    expiration_date: u64,
) -> Result<(WithdrawalRequest, RequestInfo)> {
    // Generate random secret v
    let v = params.random_scalar();
    let gammas = params.gammas();
    let joined_commitment_opening = params.random_scalar();
    // Compute joined commitment for all attributes (public and private)
    let joined_commitment: G1Projective = params.gen1() * joined_commitment_opening
        + params.gamma_idx(0).unwrap() * sk_user.sk
        + params.gamma_idx(1).unwrap() * v;

    // Compute commitment hash h
    let joined_commitment_hash = hash_g1((joined_commitment + params.gamma_idx(2).unwrap() * Scalar::from(expiration_date)).to_bytes());

    // Compute Pedersen commitments for private attributes
    let private_attributes = vec![sk_user.sk, v];
    let (private_attributes_openings, private_attributes_commitments) =
        compute_private_attribute_commitments(
            &params,
            &joined_commitment_hash,
            &private_attributes,
        );

    // construct a zk proof of knowledge proving possession of m1, m2, o, o1, o2
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
    let zk_proof = WithdrawalReqProof::construct(&params, &instance, &witness);

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
            v,
            expiration_date: Scalar::from(expiration_date),
        },
    ))
}

/// Verifies the integrity of a withdrawal request, including the joined commitment hash
/// and the zero-knowledge proof of knowledge.
///
/// # Arguments
///
/// * `params` - Group parameters used in the cryptographic operations.
/// * `req` - The withdrawal request to be verified.
/// * `pk_user` - Public key of the user associated with the withdrawal request.
/// * `expiration_date` - Expiration date for the withdrawal request.
///
/// # Returns
///
/// Returns `Ok(true)` if the verification is successful, otherwise returns an error
/// with a specific message indicating the verification failure.
pub fn request_verify(
    params: &GroupParameters,
    req: &WithdrawalRequest,
    pk_user: PublicKeyUser,
    expiration_date: u64,
) -> Result<bool> {
    // Verify the joined commitment hash
    let expected_commitment_hash = hash_g1(
        (req.joined_commitment + params.gamma_idx(2).unwrap() * Scalar::from(expiration_date))
            .to_bytes(),
    );
    if req.joined_commitment_hash != expected_commitment_hash {
        return Err(CompactEcashError::WithdrawalRequestVerification(
            "Failed to verify the commitment hash".to_string(),
        ));
    }
    // Verify zk proof
    let instance = WithdrawalReqInstance {
        joined_commitment: req.joined_commitment,
        joined_commitment_hash: req.joined_commitment_hash,
        private_attributes_commitments: req.private_attributes_commitments.clone(),
        pk_user,
    };
    if !req.zk_proof.verify(&params, &instance) {
        return Err(CompactEcashError::WithdrawalRequestVerification(
            "Failed to verify the proof of knowledge".to_string(),
        ));
    }
    Ok(true)
}

pub fn blind_sing_private_attribute(
    private_attribute_commitment: &G1Projective,
    yi: &Scalar,
) -> G1Projective {
    private_attribute_commitment * yi
}

pub fn sign_expiration_date(
    joined_commitment_hash: &G1Projective,
    expiration_date: u64,
    sk_auth: &SecretKeyAuth,
) -> G1Projective {
    joined_commitment_hash * (sk_auth.get_y_by_idx(2).unwrap() * Scalar::from(expiration_date))
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
/// * `params` - Group parameters used in the cryptographic operations.
/// * `sk_auth` - Secret key of the signing authority.
/// * `pk_user` - Public key of the user associated with the withdrawal request.
/// * `withdrawal_req` - The withdrawal request to be signed.
/// * `expiration_date` - Expiration date for the withdrawal request.
///
/// # Returns
///
/// Returns a `BlindedSignature` if the issuance process is successful, otherwise returns an error
/// with a specific message indicating the failure.
pub fn issue_wallet(
    params: &GroupParameters,
    sk_auth: SecretKeyAuth,
    pk_user: PublicKeyUser,
    withdrawal_req: &WithdrawalRequest,
    expiration_date: u64,
) -> Result<BlindedSignature> {
    // Verify the withdrawal request
    request_verify(&params, &withdrawal_req, pk_user, expiration_date)?;
    // Blind sign the private attributes
    let blind_signatures: G1Projective = withdrawal_req
        .private_attributes_commitments
        .iter()
        .zip(sk_auth.ys.iter().take(2))
        .map(|(pc, yi)| blind_sing_private_attribute(pc, yi))
        .sum();
    // Sign the expiration date
    let expiration_date_sign = sign_expiration_date(
        &withdrawal_req.joined_commitment_hash,
        expiration_date,
        &sk_auth,
    );
    // Combine both signatures
    let signature =
        blind_signatures + withdrawal_req.joined_commitment_hash * sk_auth.x + expiration_date_sign;

    Ok(BlindedSignature(
        withdrawal_req.joined_commitment_hash,
        signature,
    ))
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
/// * `params` - Group parameters used in the cryptographic operations.
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
    params: &GroupParameters,
    vk_auth: &VerificationKeyAuth,
    sk_user: &SecretKeyUser,
    blind_signature: &BlindedSignature,
    req_info: &RequestInfo,
) -> Result<PartialWallet> {
    // Verify the integrity of the response from the authority
    if req_info.joined_commitment_hash != blind_signature.0 {
        return Err(CompactEcashError::IssuanceVfy(
            "Integrity verification failed".to_string(),
        ));
    }

    // Unblind the blinded signature on the partial wallet
    let blinding_removers = vk_auth
        .beta_g1
        .iter()
        .zip(&req_info.private_attributes_openings)
        .map(|(beta, opening)| beta * opening)
        .sum::<G1Projective>();
    let unblinded_c = blind_signature.1 - blinding_removers;

    let attr = vec![sk_user.sk, req_info.v, req_info.expiration_date];

    let signed_attributes = attr
        .iter()
        .zip(vk_auth.beta_g2.iter())
        .map(|(attr, beta_i)| beta_i * attr)
        .sum::<G2Projective>();

    // Verify the signature correctness on the wallet share
    if !check_bilinear_pairing(
        &blind_signature.0.to_affine(),
        &G2Prepared::from((vk_auth.alpha + signed_attributes).to_affine()),
        &unblinded_c.to_affine(),
        params.prepared_miller_g2(),
    ) {
        return Err(CompactEcashError::IssuanceVfy(
            "Verification of wallet share failed".to_string(),
        ));
    }

    Ok(PartialWallet {
        sig: Signature(blind_signature.0, unblinded_c),
        v: req_info.v,
        idx: None,
        expiration_date: req_info.expiration_date,
    })
}
