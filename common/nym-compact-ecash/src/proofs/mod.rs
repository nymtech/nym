use std::borrow::Borrow;

use bls12_381::{G1Affine, G1Projective, Scalar};
use digest::Digest;
use digest::generic_array::typenum::Unsigned;
use group::GroupEncoding;
use itertools::izip;
use sha2::Sha256;

use crate::scheme::keygen::PublicKeyUser;
use crate::scheme::setup::Parameters;

type ChallengeDigest = Sha256;

/// Generates a Scalar [or Fp] challenge by hashing a number of elliptic curve points.
fn compute_challenge<D, I, B>(iter: I) -> Scalar
    where
        D: Digest,
        I: Iterator<Item=B>,
        B: AsRef<[u8]>,
{
    let mut h = D::new();
    for point_representation in iter {
        h.update(point_representation);
    }
    let digest = h.finalize();

    // TODO: I don't like the 0 padding here (though it's what we've been using before,
    // but we never had a security audit anyway...)
    // instead we could maybe use the `from_bytes` variant and adding some suffix
    // when computing the digest until we produce a valid scalar.
    let mut bytes = [0u8; 64];
    let pad_size = 64usize
        .checked_sub(D::OutputSize::to_usize())
        .unwrap_or_default();

    bytes[pad_size..].copy_from_slice(&digest);

    Scalar::from_bytes_wide(&bytes)
}

fn produce_response(witness_replacement: &Scalar, challenge: &Scalar, secret: &Scalar) -> Scalar {
    witness_replacement - challenge * secret
}

// note: it's caller's responsibility to ensure witnesses.len() = secrets.len()
fn produce_responses<S>(witnesses: &[Scalar], challenge: &Scalar, secrets: &[S]) -> Vec<Scalar>
    where
        S: Borrow<Scalar>,
{
    debug_assert_eq!(witnesses.len(), secrets.len());

    witnesses
        .iter()
        .zip(secrets.iter())
        .map(|(w, x)| produce_response(w, challenge, x.borrow()))
        .collect()
}

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
        // generate random values to replace the witnesses
        let r_com_opening = params.random_scalar();
        let r_pedcom_openings = params.n_random_scalars(witness.pc_coms_openings.len());
        let r_attributes = params.n_random_scalars(witness.attributes.len());

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
            .map(|(o_j, m_j)| params.gen1() * o_j + instance.h * m_j)
            .collect::<Vec<_>>();

        let zkcm_user_sk = params.gen1() * r_attributes[0];

        // covert to bytes
        let gammas_bytes = params
            .gammas()
            .iter()
            .map(|gamma| gamma.to_bytes())
            .collect::<Vec<_>>();

        let pc_coms_bytes = instance
            .pc_coms
            .iter()
            .map(|cm| cm.to_bytes())
            .collect::<Vec<_>>();

        let zkcm_pedcom_bytes = zkcm_pedcom
            .iter()
            .map(|cm| cm.to_bytes())
            .collect::<Vec<_>>();

        println!("Zk commitments to com {:?}", zkcm_com.to_bytes());
        // compute zkp challenge using g1, gammas, c, h, c1, c2, c3, zk commitments
        let challenge = compute_challenge::<ChallengeDigest, _, _>(
            std::iter::once(params.gen1().to_bytes().as_ref())
                .chain(gammas_bytes.iter().map(|gamma| gamma.as_ref()))
                .chain(std::iter::once(instance.com.to_bytes().as_ref()))
                .chain(std::iter::once(instance.h.to_bytes().as_ref()))
                .chain(pc_coms_bytes.iter().map(|pcm| pcm.as_ref()))
                .chain(std::iter::once(zkcm_com.to_bytes().as_ref()))
                .chain(zkcm_pedcom_bytes.iter().map(|c| c.as_ref()))
                .chain(std::iter::once(zkcm_user_sk.to_bytes().as_ref())),
        );

        // compute response
        let response_opening = produce_response(
            &r_com_opening,
            &challenge,
            &witness.com_opening,
        );
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

    pub(crate) fn verify(&self, params: &Parameters, instance: &WithdrawalReqInstance) -> bool {
        // recompute zk commitments for each instance
        let zkcm_com = instance.com * self.challenge
            + params.gen1() * self.response_opening
            + self
            .response_attributes
            .iter()
            .zip(params.gammas().iter())
            .map(|(m_i, gamma_i)| gamma_i * m_i)
            .sum::<G1Projective>();

        let zkcm_pedcom = izip!(
            instance.pc_coms.iter(),
            self.response_openings.iter(),
            self.response_attributes.iter())
            .map(|(cm_j, resp_o_j, resp_m_j)| cm_j * self.challenge + params.gen1() * resp_o_j + instance.h * resp_m_j)
            .collect::<Vec<_>>();

        let zk_commitment_user_sk = instance.pk_user.pk * self.challenge + params.gen1() * self.response_attributes[0];

        // covert to bytes
        let gammas_bytes = params
            .gammas()
            .iter()
            .map(|gamma| gamma.to_bytes())
            .collect::<Vec<_>>();

        let pc_coms_bytes = instance
            .pc_coms
            .iter()
            .map(|cm| cm.to_bytes())
            .collect::<Vec<_>>();

        let zkcm_pedcom_bytes = zkcm_pedcom
            .iter()
            .map(|cm| cm.to_bytes())
            .collect::<Vec<_>>();

        println!("Zk commitments to com Vfy {:?}", zkcm_com.to_bytes());

        // recompute zkp challenge
        let challenge = compute_challenge::<ChallengeDigest, _, _>(
            std::iter::once(params.gen1().to_bytes().as_ref())
                .chain(gammas_bytes.iter().map(|hs| hs.as_ref()))
                .chain(std::iter::once(instance.com.to_bytes().as_ref()))
                .chain(std::iter::once(instance.h.to_bytes().as_ref()))
                .chain(pc_coms_bytes.iter().map(|pcm| pcm.as_ref()))
                .chain(std::iter::once(zkcm_com.to_bytes().as_ref()))
                .chain(zkcm_pedcom_bytes.iter().map(|c| c.as_ref()))
                .chain(std::iter::once(zk_commitment_user_sk.to_bytes().as_ref())),
        );

        println!("Original challenge: {:?}", self.challenge);
        println!("Recomputed challenge: {:?}", challenge);
        challenge == self.challenge
    }
}

#[cfg(test)]
mod tests {
    use group::Group;
    use rand::thread_rng;

    use crate::utils::hash_g1;

    use super::*;

    #[test]
    fn withdrawal_proof_construct_and_verify() {
        let mut rng = thread_rng();
        let params = Parameters::new().unwrap();
        let sk = params.random_scalar();
        let pk_user = PublicKeyUser { pk: params.gen1() * sk };
        let v = params.random_scalar();
        let t = params.random_scalar();
        let attr = vec![sk, v, t];

        let com_opening = params.random_scalar();
        let com = params.gen1() * com_opening + attr
            .iter()
            .zip(params.gammas())
            .map(|(&m, gamma)| gamma * m)
            .sum::<G1Projective>();
        let h = hash_g1(com.to_bytes());

        let pc_openings = params.n_random_scalars(attr.len());
        let pc_coms = pc_openings
            .iter()
            .zip(attr.iter())
            .map(|(o_j, m_j)| params.gen1() * o_j + h * m_j)
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