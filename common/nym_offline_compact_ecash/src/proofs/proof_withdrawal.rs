use std::convert::{TryFrom, TryInto};

use bls12_381::{G1Projective, Scalar};
use group::GroupEncoding;
use itertools::izip;

use crate::error::{CompactEcashError, Result};
use crate::proofs::{compute_challenge, produce_response, produce_responses, ChallengeDigest};
use crate::scheme::keygen::PublicKeyUser;
use crate::scheme::setup::GroupParameters;
use crate::utils::{
    try_deserialize_g1_projective, try_deserialize_scalar, try_deserialize_scalar_vec,
};

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
// instance: g, gamma1, gamma2, gamma3, com, h, com1, com2, com3, pkUser
pub struct WithdrawalReqInstance {
    // Joined commitment to all attributes
    pub joined_commitment: G1Projective,
    // Hash of the joined commitment com
    pub joined_commitment_hash: G1Projective,
    // Pedersen commitments to each attribute
    pub private_attributes_commitments: Vec<G1Projective>,
    // Public key of a user
    pub pk_user: PublicKeyUser,
}

impl TryFrom<&[u8]> for WithdrawalReqInstance {
    type Error = CompactEcashError;

    fn try_from(bytes: &[u8]) -> Result<WithdrawalReqInstance> {
        if bytes.len() < 48 * 4 + 8 || (bytes.len() - 8) % 48 != 0 {
            return Err(CompactEcashError::DeserializationInvalidLength {
                actual: bytes.len(),
                modulus_target: bytes.len() - 8,
                target: 48 * 4 + 8,
                modulus: 48,
                object: "withdrawal request zkp instance".to_string(),
            });
        }
        let com_bytes: [u8; 48] = bytes[..48].try_into().unwrap();
        let joined_commitment = try_deserialize_g1_projective(
            &com_bytes,
            CompactEcashError::Deserialization("Failed to deserialize com".to_string()),
        )?;
        let h_bytes: [u8; 48] = bytes[48..96].try_into().unwrap();
        let joined_commitment_hash = try_deserialize_g1_projective(
            &h_bytes,
            CompactEcashError::Deserialization("Failed to deserialize h".to_string()),
        )?;
        let pc_coms_len = u64::from_le_bytes(bytes[96..104].try_into().unwrap());
        let actual_pc_coms_len = (bytes.len() - 152) / 48;
        if pc_coms_len as usize != actual_pc_coms_len {
            return Err(CompactEcashError::Deserialization(format!(
                "Tried to deserialize pedersen commitments with inconsistent pc_coms_len (expected {}, got {})",
                pc_coms_len, actual_pc_coms_len
            )));
        }
        let mut private_attributes_commitments = Vec::new();
        let mut pc_coms_end: usize = 0;
        for i in 0..pc_coms_len {
            let start = (104 + i * 48) as usize;
            let end = start + 48;
            let pc_i_bytes = bytes[start..end].try_into().unwrap();
            let pc_i = try_deserialize_g1_projective(
                &pc_i_bytes,
                CompactEcashError::Deserialization(
                    "Failed to deserialize pedersen commitment".to_string(),
                ),
            )?;
            pc_coms_end = end;
            private_attributes_commitments.push(pc_i);
        }
        let pk_bytes = bytes[pc_coms_end..].try_into().unwrap();
        let pk = try_deserialize_g1_projective(
            &pk_bytes,
            CompactEcashError::Deserialization(
                "Failed to deserialize user's public key".to_string(),
            ),
        )?;

        Ok(WithdrawalReqInstance {
            joined_commitment,
            joined_commitment_hash,
            private_attributes_commitments,
            pk_user: PublicKeyUser { pk },
        })
    }
}

impl WithdrawalReqInstance {
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let pc_coms_len = self.private_attributes_commitments.len();
        let mut bytes = Vec::with_capacity(8 + (pc_coms_len + 3) * 48);
        bytes.extend_from_slice(self.joined_commitment.to_bytes().as_ref());
        bytes.extend_from_slice(self.joined_commitment_hash.to_bytes().as_ref());
        bytes.extend_from_slice(&pc_coms_len.to_le_bytes());
        for pc in self.private_attributes_commitments.iter() {
            bytes.extend_from_slice((pc.to_bytes()).as_ref());
        }
        bytes.extend_from_slice(self.pk_user.pk.to_bytes().as_ref());
        bytes
    }

    #[allow(dead_code)]
    pub fn from_bytes(bytes: &[u8]) -> Result<WithdrawalReqInstance> {
        WithdrawalReqInstance::try_from(bytes)
    }
}

// witness: m1, m2, m3, o, o1, o2, o3,
pub struct WithdrawalReqWitness {
    pub private_attributes: Vec<Scalar>,
    // Opening for the joined commitment com
    pub joined_commitment_opening: Scalar,
    // Openings for the pedersen commitments of private attributes
    pub private_attributes_openings: Vec<Scalar>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct WithdrawalReqProof {
    challenge: Scalar,
    response_opening: Scalar,
    response_openings: Vec<Scalar>,
    response_attributes: Vec<Scalar>,
}

impl WithdrawalReqProof {
    pub(crate) fn construct(
        params: &GroupParameters,
        instance: &WithdrawalReqInstance,
        witness: &WithdrawalReqWitness,
    ) -> Self {
        // generate random values to replace the witnesses
        let r_com_opening = params.random_scalar();
        let r_pedcom_openings = params.n_random_scalars(witness.private_attributes_openings.len());
        let r_attributes = params.n_random_scalars(witness.private_attributes.len());

        // compute zkp commitments for each instance
        let zkcm_com = params.gen1() * r_com_opening
            + r_attributes
                .iter()
                .zip(params.gammas().iter())
                .map(|(rm_i, gamma_i)| gamma_i * rm_i)
                .sum::<G1Projective>();

        let zkcm_pedcom = r_pedcom_openings
            .iter()
            .zip(r_attributes.iter())
            .map(|(o_j, m_j)| params.gen1() * o_j + instance.joined_commitment_hash * m_j)
            .collect::<Vec<_>>();

        let zkcm_user_sk = params.gen1() * r_attributes[0];

        // covert to bytes
        let gammas_bytes = params
            .gammas()
            .iter()
            .map(|gamma| gamma.to_bytes())
            .collect::<Vec<_>>();

        let zkcm_pedcom_bytes = zkcm_pedcom
            .iter()
            .map(|cm| cm.to_bytes())
            .collect::<Vec<_>>();

        // compute zkp challenge using g1, gammas, c, h, c1, c2, c3, zk commitments
        let challenge = compute_challenge::<ChallengeDigest, _, _>(
            std::iter::once(params.gen1().to_bytes().as_ref())
                .chain(gammas_bytes.iter().map(|gamma| gamma.as_ref()))
                .chain(std::iter::once(instance.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_com.to_bytes().as_ref()))
                .chain(zkcm_pedcom_bytes.iter().map(|c| c.as_ref()))
                .chain(std::iter::once(zkcm_user_sk.to_bytes().as_ref())),
        );

        // compute response
        let response_opening = produce_response(
            &r_com_opening,
            &challenge,
            &witness.joined_commitment_opening,
        );
        let response_openings = produce_responses(
            &r_pedcom_openings,
            &challenge,
            &witness
                .private_attributes_openings
                .iter()
                .collect::<Vec<_>>(),
        );
        let response_attributes = produce_responses(
            &r_attributes,
            &challenge,
            &witness.private_attributes.iter().collect::<Vec<_>>(),
        );

        WithdrawalReqProof {
            challenge,
            response_opening,
            response_openings,
            response_attributes,
        }
    }

    pub(crate) fn verify(
        &self,
        params: &GroupParameters,
        instance: &WithdrawalReqInstance,
    ) -> bool {
        // recompute zk commitments for each instance
        let zkcm_com = instance.joined_commitment * self.challenge
            + params.gen1() * self.response_opening
            + self
                .response_attributes
                .iter()
                .zip(params.gammas().iter())
                .map(|(m_i, gamma_i)| gamma_i * m_i)
                .sum::<G1Projective>();

        let zkcm_pedcom = izip!(
            instance.private_attributes_commitments.iter(),
            self.response_openings.iter(),
            self.response_attributes.iter()
        )
        .map(|(cm_j, resp_o_j, resp_m_j)| {
            cm_j * self.challenge
                + params.gen1() * resp_o_j
                + instance.joined_commitment_hash * resp_m_j
        })
        .collect::<Vec<_>>();

        let zk_commitment_user_sk =
            instance.pk_user.pk * self.challenge + params.gen1() * self.response_attributes[0];

        // covert to bytes
        let gammas_bytes = params
            .gammas()
            .iter()
            .map(|gamma| gamma.to_bytes())
            .collect::<Vec<_>>();

        let zkcm_pedcom_bytes = zkcm_pedcom
            .iter()
            .map(|cm| cm.to_bytes())
            .collect::<Vec<_>>();

        // recompute zkp challenge
        let challenge = compute_challenge::<ChallengeDigest, _, _>(
            std::iter::once(params.gen1().to_bytes().as_ref())
                .chain(gammas_bytes.iter().map(|hs| hs.as_ref()))
                .chain(std::iter::once(instance.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_com.to_bytes().as_ref()))
                .chain(zkcm_pedcom_bytes.iter().map(|c| c.as_ref()))
                .chain(std::iter::once(zk_commitment_user_sk.to_bytes().as_ref())),
        );

        challenge == self.challenge
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let challenge_bytes = self.challenge.to_bytes();
        let response_opening_bytes = self.response_opening.to_bytes();
        let ro_len = self.response_openings.len() as u64;
        let ra_len = self.response_attributes.len() as u64;

        let mut bytes =
            Vec::with_capacity(32 + 32 + 8 + ro_len as usize * 32 + 8 + ra_len as usize * 32);
        bytes.extend_from_slice(&challenge_bytes);
        bytes.extend_from_slice(&response_opening_bytes);
        bytes.extend_from_slice(&ro_len.to_le_bytes());
        for ro in &self.response_openings {
            bytes.extend_from_slice(&ro.to_bytes());
        }
        bytes.extend_from_slice(&ra_len.to_le_bytes());
        for ra in &self.response_attributes {
            bytes.extend_from_slice(&ra.to_bytes());
        }
        bytes
    }
}

impl TryFrom<&[u8]> for WithdrawalReqProof {
    type Error = CompactEcashError;

    fn try_from(bytes: &[u8]) -> Result<WithdrawalReqProof> {
        if bytes.len() < 32 + 32 + 16 + 32 + 32 || (bytes.len() - 16) % 32 != 0 {
            return Err(CompactEcashError::Deserialization(
                "tried to deserialize proof of withdrawal with bytes of invalid length".to_string(),
            ));
        }

        let mut idx = 0;
        let challenge_bytes = bytes[idx..idx + 32].try_into().unwrap();
        idx += 32;
        let response_opening_bytes = bytes[idx..idx + 32].try_into().unwrap();
        idx += 32;

        let challenge = try_deserialize_scalar(
            &challenge_bytes,
            CompactEcashError::Deserialization("Failed to deserialize challenge".to_string()),
        )?;

        let response_opening = try_deserialize_scalar(
            &response_opening_bytes,
            CompactEcashError::Deserialization(
                "Failed to deserialize the response to the random".to_string(),
            ),
        )?;

        let ro_len = u64::from_le_bytes(bytes[idx..idx + 8].try_into().unwrap());
        idx += 8;
        if bytes[idx..].len() < ro_len as usize * 32 + 8 {
            return Err(CompactEcashError::Deserialization(
                "tried to deserialize response openings".to_string(),
            ));
        }
        let ro_end = idx + ro_len as usize * 32;
        let response_openings = try_deserialize_scalar_vec(
            ro_len,
            &bytes[idx..ro_end],
            CompactEcashError::Deserialization(
                "Failed to deserialize openings response".to_string(),
            ),
        )?;

        let ra_len = u64::from_le_bytes(bytes[ro_end..ro_end + 8].try_into().unwrap());
        let response_attributes = try_deserialize_scalar_vec(
            ra_len,
            &bytes[ro_end + 8..],
            CompactEcashError::Deserialization(
                "Failed to deserialize attributes response".to_string(),
            ),
        )?;

        Ok(WithdrawalReqProof {
            challenge,
            response_opening,
            response_openings,
            response_attributes,
        })
    }
}

#[cfg(test)]
mod tests {
    use group::Group;
    use rand::thread_rng;

    use crate::utils::hash_g1;

    use super::*;

    #[test]
    fn withdrawal_request_instance_roundtrip() {
        let mut rng = thread_rng();
        let params = GroupParameters::new();
        let instance = WithdrawalReqInstance {
            joined_commitment: G1Projective::random(&mut rng),
            joined_commitment_hash: G1Projective::random(&mut rng),
            private_attributes_commitments: vec![
                G1Projective::random(&mut rng),
                G1Projective::random(&mut rng),
                G1Projective::random(&mut rng),
            ],
            pk_user: PublicKeyUser {
                pk: params.gen1() * params.random_scalar(),
            },
        };

        let instance_bytes = instance.to_bytes();
        let instance_p = WithdrawalReqInstance::from_bytes(&instance_bytes).unwrap();
        assert_eq!(instance, instance_p)
    }

    #[test]
    fn withdrawal_proof_construct_and_verify() {
        let _rng = thread_rng();
        let params = GroupParameters::new();
        let sk = params.random_scalar();
        let pk_user = PublicKeyUser {
            pk: params.gen1() * sk,
        };
        let v = params.random_scalar();
        let t = params.random_scalar();
        let private_attributes = vec![sk, v, t];

        let joined_commitment_opening = params.random_scalar();
        let joined_commitment = params.gen1() * joined_commitment_opening
            + private_attributes
                .iter()
                .zip(params.gammas())
                .map(|(&m, gamma)| gamma * m)
                .sum::<G1Projective>();
        let joined_commitment_hash = hash_g1(joined_commitment.to_bytes());

        let private_attributes_openings = params.n_random_scalars(private_attributes.len());
        let private_attributes_commitments = private_attributes_openings
            .iter()
            .zip(private_attributes.iter())
            .map(|(o_j, m_j)| params.gen1() * o_j + joined_commitment_hash * m_j)
            .collect::<Vec<_>>();

        let instance = WithdrawalReqInstance {
            joined_commitment,
            joined_commitment_hash,
            private_attributes_commitments,
            pk_user,
        };

        let witness = WithdrawalReqWitness {
            private_attributes,
            joined_commitment_opening,
            private_attributes_openings,
        };
        let zk_proof = WithdrawalReqProof::construct(&params, &instance, &witness);
        assert!(zk_proof.verify(&params, &instance))
    }
}
