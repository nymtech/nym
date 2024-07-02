// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ecash_group_parameters;
use crate::proofs::{compute_challenge, produce_response, produce_responses, ChallengeDigest};
use crate::scheme::keygen::PublicKeyUser;
use bls12_381::{G1Projective, Scalar};
use group::GroupEncoding;
use itertools::izip;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
// instance: g, gamma1, gamma2, gamma3, com, h, com1, com2, com3, pkUser
pub struct WithdrawalReqInstance {
    // Joined commitment to all attributes
    pub(crate) joined_commitment: G1Projective,
    // Hash of the joined commitment com
    pub(crate) joined_commitment_hash: G1Projective,
    // Pedersen commitments to each attribute
    pub(crate) private_attributes_commitments: Vec<G1Projective>,
    // Public key of a user
    pub(crate) pk_user: PublicKeyUser,
}

// witness: m1, m2, m3, o, o1, o2, o3,
pub struct WithdrawalReqWitness {
    pub private_attributes: Vec<Scalar>,
    // Opening for the joined commitment com
    pub joined_commitment_opening: Scalar,
    // Openings for the pedersen commitments of private attributes
    pub private_attributes_openings: Vec<Scalar>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct WithdrawalReqProof {
    challenge: Scalar,
    response_opening: Scalar,
    response_openings: Vec<Scalar>,
    response_attributes: Vec<Scalar>,
}

impl WithdrawalReqProof {
    pub(crate) fn construct(
        instance: &WithdrawalReqInstance,
        witness: &WithdrawalReqWitness,
    ) -> Self {
        let params = ecash_group_parameters();
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

    pub(crate) fn verify(&self, instance: &WithdrawalReqInstance) -> bool {
        let params = ecash_group_parameters();
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
}

#[cfg(test)]
mod tests {
    use group::Group;
    use rand::thread_rng;

    use crate::GroupParameters;
    use crate::{constants, utils::hash_g1};

    use super::*;

    #[test]
    fn withdrawal_request_instance_roundtrip() {
        let mut rng = thread_rng();
        let params = GroupParameters::new(constants::ATTRIBUTES_LEN);
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
        let params = GroupParameters::new(constants::ATTRIBUTES_LEN);
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
        let zk_proof = WithdrawalReqProof::construct(&instance, &witness);
        assert!(zk_proof.verify(&instance))
    }
}
