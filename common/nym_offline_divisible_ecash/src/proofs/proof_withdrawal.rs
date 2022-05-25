use std::convert::{TryFrom, TryInto};

use bls12_381::{G1Projective, Scalar};
use group::GroupEncoding;
use itertools::izip;

use crate::error::{DivisibleEcashError, Result};
use crate::proofs::{ChallengeDigest, compute_challenge, produce_response, produce_responses};
use crate::scheme::keygen::PublicKeyUser;
use crate::scheme::setup::{GroupParameters, Parameters};
use crate::utils::try_deserialize_g1_projective;

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
// instance: g, gamma1, gamma2, gamma3, com, h, com1, com2, com3, pkUser
pub struct WithdrawalReqInstance {
    // Joined commitment to all attributes
    pub com: G1Projective,
    // Hash of the joined commitment com
    pub h: G1Projective,
    // Pedersen commitments to each attribute
    pub pc_coms: Vec<G1Projective>,
    // Public key of a user
    pub pk_user: PublicKeyUser,
}

impl TryFrom<&[u8]> for WithdrawalReqInstance {
    type Error = DivisibleEcashError;

    fn try_from(bytes: &[u8]) -> Result<WithdrawalReqInstance> {
        if bytes.len() < 48 * 4 + 8 || (bytes.len() - 8) % 48 != 0 {
            return Err(DivisibleEcashError::DeserializationInvalidLength {
                actual: bytes.len(),
                modulus_target: bytes.len() - 8,
                target: 48 * 4 + 8,
                modulus: 48,
                object: "withdrawal request zkp instance".to_string(),
            });
        }
        let com_bytes: [u8; 48] = bytes[..48].try_into().unwrap();
        let com = try_deserialize_g1_projective(
            &com_bytes,
            DivisibleEcashError::Deserialization("Failed to deserialize com".to_string()),
        )?;
        let h_bytes: [u8; 48] = bytes[48..96].try_into().unwrap();
        let h = try_deserialize_g1_projective(
            &h_bytes,
            DivisibleEcashError::Deserialization("Failed to deserialize h".to_string()),
        )?;
        let pc_coms_len = u64::from_le_bytes(bytes[96..104].try_into().unwrap());
        let actual_pc_coms_len = (bytes.len() - 152) / 48;
        if pc_coms_len as usize != actual_pc_coms_len {
            return Err(DivisibleEcashError::Deserialization(format!(
                "Tried to deserialize pedersen commitments with inconsistent pc_coms_len (expected {}, got {})",
                pc_coms_len, actual_pc_coms_len
            )));
        }
        let mut pc_coms = Vec::new();
        let mut pc_coms_end: usize = 0;
        for i in 0..pc_coms_len {
            let start = (104 + i * 48) as usize;
            let end = (start + 48) as usize;
            let pc_i_bytes = bytes[start..end].try_into().unwrap();
            let pc_i = try_deserialize_g1_projective(
                &pc_i_bytes,
                DivisibleEcashError::Deserialization(
                    "Failed to deserialize pedersen commitment".to_string(),
                ),
            )?;
            pc_coms_end = end;
            pc_coms.push(pc_i);
        }
        let pk_bytes = bytes[pc_coms_end..].try_into().unwrap();
        let pk = try_deserialize_g1_projective(
            &pk_bytes,
            DivisibleEcashError::Deserialization(
                "Failed to deserialize user's public key".to_string(),
            ),
        )?;

        Ok(WithdrawalReqInstance {
            com,
            h,
            pc_coms,
            pk_user: PublicKeyUser { pk },
        })
    }
}

impl WithdrawalReqInstance {
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let pc_coms_len = self.pc_coms.len();
        let mut bytes = Vec::with_capacity(8 + (pc_coms_len + 3) as usize * 48);
        bytes.extend_from_slice(self.com.to_bytes().as_ref());
        bytes.extend_from_slice(self.h.to_bytes().as_ref());
        bytes.extend_from_slice(&pc_coms_len.to_le_bytes());
        for pc in self.pc_coms.iter() {
            bytes.extend_from_slice((pc.to_bytes()).as_ref());
        }
        bytes.extend_from_slice(self.pk_user.pk.to_bytes().as_ref());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<WithdrawalReqInstance> {
        WithdrawalReqInstance::try_from(bytes)
    }
}

// witness: m1, m2, m3, o, o1, o2, o3,
pub struct WithdrawalReqWitness {
    pub attributes: Vec<Scalar>,
    // Opening for the joined commitment com
    pub com_opening: Scalar,
    // Openings for the pedersen commitments
    pub pc_coms_openings: Vec<Scalar>,
}

pub struct WithdrawalReqProof {
    challenge: Scalar,
    response_opening: Scalar,
    response_openings: Vec<Scalar>,
    response_attributes: Vec<Scalar>,
}

impl WithdrawalReqProof {
    pub(crate) fn construct(
        params: &Parameters,
        instance: &WithdrawalReqInstance,
        witness: &WithdrawalReqWitness,
    ) -> Self {
        let grp = params.get_grp();
        let g1 = grp.gen1();
        let params_u = params.get_params_u();

        // generate random values to replace the witnesses
        let r_com_opening = grp.random_scalar();
        let r_pedcom_openings = grp.n_random_scalars(witness.pc_coms_openings.len());
        let r_attributes = grp.n_random_scalars(witness.attributes.len());

        // compute zkp commitments for each instance
        let zkcm_com = g1 * r_com_opening
            + r_attributes
            .iter()
            .zip(params_u.get_gammas().iter())
            .map(|(rm_i, gamma_i)| gamma_i * rm_i)
            .sum::<G1Projective>();

        let zkcm_pedcom = r_pedcom_openings
            .iter()
            .zip(r_attributes.iter())
            .map(|(o_j, m_j)| g1 * o_j + instance.h * m_j)
            .collect::<Vec<_>>();

        let zkcm_user_sk = g1 * r_attributes[0];

        // covert to bytes
        let gammas_bytes = params_u
            .get_gammas()
            .iter()
            .map(|gamma| gamma.to_bytes())
            .collect::<Vec<_>>();

        let zkcm_pedcom_bytes = zkcm_pedcom
            .iter()
            .map(|cm| cm.to_bytes())
            .collect::<Vec<_>>();

        // compute zkp challenge using g1, gammas, c, h, c1, c2, c3, zk commitments
        let challenge = compute_challenge::<ChallengeDigest, _, _>(
            std::iter::once(g1.to_bytes().as_ref())
                .chain(gammas_bytes.iter().map(|gamma| gamma.as_ref()))
                .chain(std::iter::once(instance.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_com.to_bytes().as_ref()))
                .chain(zkcm_pedcom_bytes.iter().map(|c| c.as_ref()))
                .chain(std::iter::once(zkcm_user_sk.to_bytes().as_ref())),
        );

        // compute response
        let response_opening = produce_response(&r_com_opening, &challenge, &witness.com_opening);
        let response_openings = produce_responses(
            &r_pedcom_openings,
            &challenge,
            &witness.pc_coms_openings.iter().collect::<Vec<_>>(),
        );
        let response_attributes = produce_responses(
            &r_attributes,
            &challenge,
            &witness.attributes.iter().collect::<Vec<_>>(),
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
        params: &Parameters,
        instance: &WithdrawalReqInstance,
    ) -> bool {
        let grp = params.get_grp();
        let g1 = grp.gen1();
        let params_u = params.get_params_u();

        // recompute zk commitments for each instance
        let zkcm_com = instance.com * self.challenge
            + g1 * self.response_opening
            + self
            .response_attributes
            .iter()
            .zip(params_u.get_gammas().iter())
            .map(|(m_i, gamma_i)| gamma_i * m_i)
            .sum::<G1Projective>();

        let zkcm_pedcom = izip!(
            instance.pc_coms.iter(),
            self.response_openings.iter(),
            self.response_attributes.iter()
        )
            .map(|(cm_j, resp_o_j, resp_m_j)| {
                cm_j * self.challenge + g1 * resp_o_j + instance.h * resp_m_j
            })
            .collect::<Vec<_>>();

        let zk_commitment_user_sk =
            instance.pk_user.pk * self.challenge + g1 * self.response_attributes[0];

        // covert to bytes
        let gammas_bytes = params_u
            .get_gammas()
            .iter()
            .map(|gamma| gamma.to_bytes())
            .collect::<Vec<_>>();

        let zkcm_pedcom_bytes = zkcm_pedcom
            .iter()
            .map(|cm| cm.to_bytes())
            .collect::<Vec<_>>();

        // recompute zkp challenge
        let challenge = compute_challenge::<ChallengeDigest, _, _>(
            std::iter::once(g1.to_bytes().as_ref())
                .chain(gammas_bytes.iter().map(|hs| hs.as_ref()))
                .chain(std::iter::once(instance.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_com.to_bytes().as_ref()))
                .chain(zkcm_pedcom_bytes.iter().map(|c| c.as_ref()))
                .chain(std::iter::once(zk_commitment_user_sk.to_bytes().as_ref())),
        );

        challenge == self.challenge
    }
}

#[cfg(test)]
mod tests {
    use group::Group;
    use rand::thread_rng;

    use crate::scheme::setup::Parameters;
    use crate::utils::hash_g1;

    use super::*;

    #[test]
    fn withdrawal_request_instance_roundtrip() {
        let mut rng = thread_rng();
        let params = GroupParameters::new().unwrap();
        let instance = WithdrawalReqInstance {
            com: G1Projective::random(&mut rng),
            h: G1Projective::random(&mut rng),
            pc_coms: vec![
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
        let rng = thread_rng();
        let grp = GroupParameters::new().unwrap();
        let params = Parameters::new(grp.clone());


        let sk = grp.random_scalar();
        let pk_user = PublicKeyUser {
            pk: grp.gen1() * sk,
        };
        let v = grp.random_scalar();
        let t = grp.random_scalar();
        let attr = vec![sk, v, t];

        let com_opening = grp.random_scalar();
        let com = grp.gen1() * com_opening
            + attr
            .iter()
            .zip(params.get_params_u().get_gammas())
            .map(|(&m, gamma)| gamma * m)
            .sum::<G1Projective>();
        let h = hash_g1(com.to_bytes());

        let pc_openings = grp.n_random_scalars(attr.len());
        let pc_coms = pc_openings
            .iter()
            .zip(attr.iter())
            .map(|(o_j, m_j)| grp.gen1() * o_j + h * m_j)
            .collect::<Vec<_>>();

        let instance = WithdrawalReqInstance {
            com,
            h,
            pc_coms,
            pk_user,
        };

        let witness = WithdrawalReqWitness {
            attributes: attr,
            com_opening,
            pc_coms_openings: pc_openings,
        };
        let zk_proof = WithdrawalReqProof::construct(&params, &instance, &witness);
        assert!(zk_proof.verify(&params, &instance))
    }
}
