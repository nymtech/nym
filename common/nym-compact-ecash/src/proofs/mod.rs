use std::borrow::Borrow;

use bls12_381::{G1Affine, G1Projective, Scalar};
use digest::generic_array::typenum::Unsigned;
use digest::Digest;
use group::GroupEncoding;
use sha2::Sha256;

use crate::scheme::keygen::PublicKeyUser;
use crate::scheme::setup::Parameters;

type ChallengeDigest = Sha256;

/// Generates a Scalar [or Fp] challenge by hashing a number of elliptic curve points.
fn compute_challenge<D, I, B>(iter: I) -> Scalar
where
    D: Digest,
    I: Iterator<Item = B>,
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

fn produce_response(witness: &Scalar, challenge: &Scalar, secret: &Scalar) -> Scalar {
    witness - challenge * secret
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
    pub g1_gen: G1Affine,
    pub gammas: Vec<G1Affine>,
    pub attrs_commitment: G1Projective,
    pub attrs_commitment_hash: G1Projective,
    pub pc_commitments: Vec<G1Projective>,
    pub pk_user: PublicKeyUser,
}

// witness: m1, m2, m3, o, o1, o2, o3,
pub struct WithdrawalReqWitness {
    pub attributes: Vec<Scalar>,
    pub attrs_commitment_opening: Scalar,
    pub pc_openings: Vec<Scalar>,
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
        let r_commitment_opening = params.random_scalar();
        let r_pedersen_commitments_openings = params.n_random_scalars(witness.pc_openings.len());
        let r_witness_attributes = params.n_random_scalars(witness.attributes.len());

        // compute zkp commitments
        let zk_commitment_attributes = instance.g1_gen * r_commitment_opening
            + r_witness_attributes
                .iter()
                .zip(instance.gammas.iter())
                .map(|(wm_i, gamma_i)| gamma_i * wm_i)
                .sum::<G1Projective>();

        let zk_pc_commitments_attributes = r_pedersen_commitments_openings
            .iter()
            .zip(r_witness_attributes.iter())
            .map(|(o_j, m_j)| instance.g1_gen * o_j + instance.attrs_commitment_hash * m_j)
            .collect::<Vec<_>>();

        let zk_commitment_user_sk = instance.g1_gen * r_witness_attributes[0];

        // covert to bytes
        let gammas_bytes = instance
            .gammas
            .iter()
            .map(|gamma| gamma.to_bytes())
            .collect::<Vec<_>>();

        let pc_commitments_bytes = instance
            .pc_commitments
            .iter()
            .map(|cm| cm.to_bytes())
            .collect::<Vec<_>>();

        let zk_commitments_attributes_bytes = zk_pc_commitments_attributes
            .iter()
            .map(|cm| cm.to_bytes())
            .collect::<Vec<_>>();

        // compute zkp challenge
        let challenge = compute_challenge::<ChallengeDigest, _, _>(
            std::iter::once(instance.g1_gen.to_bytes().as_ref())
                .chain(gammas_bytes.iter().map(|hs| hs.as_ref()))
                .chain(std::iter::once(
                    instance.attrs_commitment.to_bytes().as_ref(),
                ))
                .chain(std::iter::once(
                    instance.attrs_commitment_hash.to_bytes().as_ref(),
                ))
                .chain(pc_commitments_bytes.iter().map(|pcm| pcm.as_ref()))
                .chain(std::iter::once(
                    zk_commitment_attributes.to_bytes().as_ref(),
                ))
                .chain(zk_commitments_attributes_bytes.iter().map(|c| c.as_ref()))
                .chain(std::iter::once(zk_commitment_user_sk.to_bytes().as_ref())),
        );

        // compute response
        let response_opening = produce_response(
            &r_commitment_opening,
            &challenge,
            &witness.attrs_commitment_opening,
        );
        let response_openings = produce_responses(
            &r_pedersen_commitments_openings,
            &challenge,
            &witness.pc_openings.iter().collect::<Vec<_>>(),
        );
        let response_attributes = produce_responses(
            &r_witness_attributes,
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

    pub(crate) fn verify(&self, instance: &WithdrawalReqInstance) -> bool {
        // recompute zk commitments
        let zk_commitment_attributes = instance.g1_gen * self.response_opening
            + self
                .response_attributes
                .iter()
                .zip(instance.gammas.iter())
                .map(|(wm_i, gamma_i)| gamma_i * wm_i)
                .sum::<G1Projective>();

        let zk_pc_commitments_attributes = self
            .response_openings
            .iter()
            .zip(self.response_attributes.iter())
            .map(|(o_j, m_j)| instance.g1_gen * o_j + instance.attrs_commitment_hash * m_j)
            .collect::<Vec<_>>();

        let zk_commitment_user_sk = instance.g1_gen * self.response_attributes[0];

        // covert to bytes
        let gammas_bytes = instance
            .gammas
            .iter()
            .map(|gamma| gamma.to_bytes())
            .collect::<Vec<_>>();

        let pc_commitments_bytes = instance
            .pc_commitments
            .iter()
            .map(|cm| cm.to_bytes())
            .collect::<Vec<_>>();

        let zk_commitments_attributes_bytes = zk_pc_commitments_attributes
            .iter()
            .map(|cm| cm.to_bytes())
            .collect::<Vec<_>>();

        // recompute zkp challenge
        let challenge = compute_challenge::<ChallengeDigest, _, _>(
            std::iter::once(instance.g1_gen.to_bytes().as_ref())
                .chain(gammas_bytes.iter().map(|hs| hs.as_ref()))
                .chain(std::iter::once(
                    instance.attrs_commitment.to_bytes().as_ref(),
                ))
                .chain(std::iter::once(
                    instance.attrs_commitment_hash.to_bytes().as_ref(),
                ))
                .chain(pc_commitments_bytes.iter().map(|pcm| pcm.as_ref()))
                .chain(std::iter::once(
                    zk_commitment_attributes.to_bytes().as_ref(),
                ))
                .chain(zk_commitments_attributes_bytes.iter().map(|c| c.as_ref()))
                .chain(std::iter::once(zk_commitment_user_sk.to_bytes().as_ref())),
        );

        challenge == self.challenge
    }
}
