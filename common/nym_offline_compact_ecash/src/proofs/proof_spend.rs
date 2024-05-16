// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::convert::{TryFrom, TryInto};

use bls12_381::{G1Projective, G2Projective, Scalar};
use group::{Curve, GroupEncoding};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::error::{CompactEcashError, Result};
use crate::proofs::{compute_challenge, produce_response, produce_responses, ChallengeDigest};
use crate::scheme::keygen::VerificationKeyAuth;
use crate::scheme::setup::Parameters;
use crate::scheme::PayInfo;
use crate::utils::{
    try_deserialize_g1_projective, try_deserialize_g2_projective, try_deserialize_scalar,
    try_deserialize_scalar_vec,
};

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct SpendInstance {
    pub kappa: G2Projective,
    pub cc: G1Projective,
    pub aa: Vec<G1Projective>,
    pub ss: Vec<G1Projective>,
    pub tt: Vec<G1Projective>,
    pub kappa_k: Vec<G2Projective>,
    pub kappa_e: G2Projective,
}

impl TryFrom<&[u8]> for SpendInstance {
    type Error = CompactEcashError;

    fn try_from(bytes: &[u8]) -> Result<SpendInstance> {
        if bytes.len() < 48 * 5 + 3 * 96 || (bytes.len()) % 48 != 0 {
            return Err(CompactEcashError::DeserializationInvalidLength {
                actual: bytes.len(),
                modulus_target: bytes.len(),
                target: 48 * 5 + 3 * 96,
                modulus: 48,
                object: "spend instance".to_string(),
            });
        }

        let mut j = 0;
        //SAFETY : slice to array conversion after a length check
        #[allow(clippy::unwrap_used)]
        let kappa_bytes = bytes[j..j + 96].try_into().unwrap();
        let kappa = try_deserialize_g2_projective(
            &kappa_bytes,
            CompactEcashError::Deserialization("Failed to deserialize kappa".to_string()),
        )?;
        j += 96;
        //SAFETY : slice to array conversion after a length check
        #[allow(clippy::unwrap_used)]
        let kappa_e_bytes = bytes[j..j + 96].try_into().unwrap();
        let kappa_e = try_deserialize_g2_projective(
            &kappa_e_bytes,
            CompactEcashError::Deserialization("Failed to deserialize kappa_e".to_string()),
        )?;
        j += 96;
        //SAFETY : slice to array conversion after a length check
        #[allow(clippy::unwrap_used)]
        let a_len = u64::from_le_bytes(bytes[j..j + 8].try_into().unwrap());
        j += 8;
        if bytes[j..].len() < a_len as usize * 48 {
            return Err(CompactEcashError::DeserializationMinLength {
                min: a_len as usize * 48,
                actual: bytes[j..].len(),
            });
        }

        let mut aa = Vec::with_capacity(a_len as usize);
        for i in 0..a_len as usize {
            let start = j + i * 48;
            let end = start + 48;
            //SAFETY : slice to array conversion after a length check
            #[allow(clippy::unwrap_used)]
            let aa_elem_bytes = bytes[start..end].try_into().unwrap();
            let aa_elem = try_deserialize_g1_projective(
                &aa_elem_bytes,
                CompactEcashError::Deserialization(
                    "Failed to deserialize compressed A values".to_string(),
                ),
            )?;

            aa.push(aa_elem)
        }
        j += a_len as usize * 48;
        //SAFETY : slice to array conversion after a length check
        #[allow(clippy::unwrap_used)]
        let cc_bytes = bytes[j..j + 48].try_into().unwrap();
        let cc = try_deserialize_g1_projective(
            &cc_bytes,
            CompactEcashError::Deserialization("Failed to deserialize C".to_string()),
        )?;
        j += 48;
        //SAFETY : slice to array conversion after a length check
        #[allow(clippy::unwrap_used)]
        let s_len = u64::from_le_bytes(bytes[j..j + 8].try_into().unwrap());
        j += 8;
        if bytes[j..].len() < s_len as usize * 48 {
            return Err(CompactEcashError::DeserializationMinLength {
                min: s_len as usize * 48,
                actual: bytes[j..].len(),
            });
        }

        let mut ss = Vec::with_capacity(s_len as usize);
        for i in 0..s_len as usize {
            let start = j + i * 48;
            let end = start + 48;
            //SAFETY : slice to array conversion after a length check
            #[allow(clippy::unwrap_used)]
            let ss_elem_bytes = bytes[start..end].try_into().unwrap();
            let ss_elem = try_deserialize_g1_projective(
                &ss_elem_bytes,
                CompactEcashError::Deserialization(
                    "Failed to deserialize compressed S values".to_string(),
                ),
            )?;

            ss.push(ss_elem)
        }
        j += s_len as usize * 48;
        //SAFETY : slice to array conversion after a length check
        #[allow(clippy::unwrap_used)]
        let t_len = u64::from_le_bytes(bytes[j..j + 8].try_into().unwrap());
        j += 8;
        if bytes[j..].len() < t_len as usize * 48 {
            return Err(CompactEcashError::DeserializationMinLength {
                min: t_len as usize * 48,
                actual: bytes[j..].len(),
            });
        }

        let mut tt = Vec::with_capacity(t_len as usize);
        for i in 0..t_len as usize {
            let start = j + i * 48;
            let end = start + 48;
            //SAFETY : slice to array conversion after a length check
            #[allow(clippy::unwrap_used)]
            let tt_elem_bytes = bytes[start..end].try_into().unwrap();
            let tt_elem = try_deserialize_g1_projective(
                &tt_elem_bytes,
                CompactEcashError::Deserialization(
                    "Failed to deserialize compressed T values".to_string(),
                ),
            )?;

            tt.push(tt_elem)
        }
        j += t_len as usize * 48;
        //SAFETY : slice to array conversion after a length check
        #[allow(clippy::unwrap_used)]
        let kappa_k_len = u64::from_le_bytes(bytes[j..j + 8].try_into().unwrap());
        j += 8;
        if bytes[j..].len() < kappa_k_len as usize * 96 {
            return Err(CompactEcashError::DeserializationMinLength {
                min: kappa_k_len as usize * 96,
                actual: bytes[j..].len(),
            });
        }

        let mut kappa_k = Vec::with_capacity(kappa_k_len as usize);
        for i in 0..kappa_k_len as usize {
            let start = j + i * 48;
            let end = start + 48;
            //SAFETY : slice to array conversion after a length check
            #[allow(clippy::unwrap_used)]
            let kappa_k_elem_bytes = bytes[start..end].try_into().unwrap();
            let kappa_k_elem = try_deserialize_g2_projective(
                &kappa_k_elem_bytes,
                CompactEcashError::Deserialization(
                    "Failed to deserialize compressed kappa_k values".to_string(),
                ),
            )?;

            kappa_k.push(kappa_k_elem)
        }

        Ok(SpendInstance {
            kappa,
            aa,
            cc,
            ss,
            tt,
            kappa_k,
            kappa_e,
        })
    }
}

impl SpendInstance {
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Default::default();
        bytes.extend_from_slice(self.kappa.to_bytes().as_ref());
        bytes.extend_from_slice(self.kappa_e.to_bytes().as_ref());

        bytes.extend_from_slice(&self.aa.len().to_le_bytes());
        for a in &self.aa {
            bytes.extend_from_slice(&a.to_affine().to_compressed());
        }

        bytes.extend_from_slice(self.cc.to_bytes().as_ref());

        bytes.extend_from_slice(&self.ss.len().to_le_bytes());
        for s in &self.ss {
            bytes.extend_from_slice(&s.to_affine().to_compressed());
        }
        bytes.extend_from_slice(&self.tt.len().to_le_bytes());
        for t in &self.tt {
            bytes.extend_from_slice(&t.to_affine().to_compressed());
        }

        bytes.extend_from_slice(&self.kappa_k.len().to_le_bytes());
        for k in &self.kappa_k {
            bytes.extend_from_slice(&k.to_affine().to_compressed());
        }
        bytes
    }
}

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct SpendWitness {
    // includes skUser, v, t
    pub(crate) attributes: Vec<Scalar>,
    // signature randomizing element
    pub(crate) r: Scalar,
    pub(crate) o_c: Scalar,
    pub(crate) lk: Vec<Scalar>,
    pub(crate) o_a: Vec<Scalar>,
    pub(crate) mu: Vec<Scalar>,
    pub(crate) o_mu: Vec<Scalar>,
    pub(crate) r_k: Vec<Scalar>,
    pub(crate) r_e: Scalar,
}

pub struct WitnessReplacement {
    pub r_attributes: Vec<Scalar>,
    pub r_r: Scalar,
    pub r_r_e: Scalar,
    pub r_o_c: Scalar,
    pub r_r_lk: Vec<Scalar>,
    pub r_lk: Vec<Scalar>,
    pub r_o_a: Vec<Scalar>,
    pub r_mu: Vec<Scalar>,
    pub r_o_mu: Vec<Scalar>,
}

pub struct InstanceCommitments {
    pub tt_kappa: G2Projective,
    pub tt_kappa_e: G2Projective,
    pub tt_cc: G1Projective,
    pub tt_aa: Vec<G1Projective>,
    pub tt_ss: Vec<G1Projective>,
    pub tt_tt: Vec<G1Projective>,
    pub tt_gamma1: Vec<G1Projective>,
    pub tt_kappa_k: Vec<G2Projective>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpendProof {
    challenge: Scalar,
    response_r: Scalar,
    response_r_e: Scalar,
    responses_r_k: Vec<Scalar>,
    responses_l: Vec<Scalar>,
    responses_o_a: Vec<Scalar>,
    response_o_c: Scalar,
    responses_mu: Vec<Scalar>,
    responses_o_mu: Vec<Scalar>,
    responses_attributes: Vec<Scalar>,
}

pub fn generate_witness_replacement(
    params: &Parameters,
    witness: &SpendWitness,
) -> WitnessReplacement {
    let grp_params = params.grp();
    let r_attributes = grp_params.n_random_scalars(witness.attributes.len());
    let r_r = grp_params.random_scalar();
    let r_r_e = grp_params.random_scalar();
    let r_o_c = grp_params.random_scalar();

    let r_r_lk = grp_params.n_random_scalars(witness.r_k.len());
    let r_lk = grp_params.n_random_scalars(witness.lk.len());
    let r_o_a = grp_params.n_random_scalars(witness.o_a.len());
    let r_mu = grp_params.n_random_scalars(witness.mu.len());
    let r_o_mu = grp_params.n_random_scalars(witness.o_mu.len());
    WitnessReplacement {
        r_attributes,
        r_r,
        r_r_e,
        r_o_c,
        r_r_lk,
        r_lk,
        r_o_a,
        r_mu,
        r_o_mu,
    }
}

pub fn compute_instance_commitments(
    params: &Parameters,
    witness_replacement: &WitnessReplacement,
    instance: &SpendInstance,
    verification_key: &VerificationKeyAuth,
    rr: &[Scalar],
) -> InstanceCommitments {
    let grp_params = params.grp();
    let g1 = *grp_params.gen1();
    let gamma0 = grp_params.gamma_idx(0).unwrap();
    let gamma1 = grp_params.gamma_idx(1).unwrap();

    let tt_kappa = grp_params.gen2() * witness_replacement.r_r
        + verification_key.alpha
        + witness_replacement
            .r_attributes
            .iter()
            .zip(verification_key.beta_g2.iter())
            .map(|(attr, beta_i)| beta_i * attr)
            .sum::<G2Projective>();

    let tt_cc = g1 * witness_replacement.r_o_c + gamma0 * witness_replacement.r_attributes[1];

    let tt_kappa_e = grp_params.gen2() * witness_replacement.r_r_e
        + verification_key.alpha
        + verification_key.beta_g2[0] * witness_replacement.r_attributes[2];

    let tt_aa: Vec<G1Projective> = witness_replacement
        .r_o_a
        .iter()
        .zip(witness_replacement.r_lk.iter())
        .map(|(r_o_a_k, r_l_k)| g1 * r_o_a_k + gamma0 * r_l_k)
        .collect::<Vec<_>>();

    let tt_kappa_k = witness_replacement
        .r_lk
        .iter()
        .zip(witness_replacement.r_r_lk.iter())
        .map(|(r_l_k, r_r_k)| {
            verification_key.alpha + verification_key.beta_g2[0] * r_l_k + grp_params.gen2() * r_r_k
        })
        .collect::<Vec<_>>();

    let tt_ss = witness_replacement
        .r_mu
        .iter()
        .map(|r_mu_k| grp_params.delta() * r_mu_k)
        .collect::<Vec<_>>();

    let tt_tt = rr
        .iter()
        .zip(witness_replacement.r_mu.iter())
        .map(|(rr_k, r_mu_k)| g1 * witness_replacement.r_attributes[0] + (g1 * rr_k) * r_mu_k)
        .collect::<Vec<_>>();

    let tt_gamma1 = instance
        .aa
        .iter()
        .zip(witness_replacement.r_mu.iter())
        .zip(witness_replacement.r_o_mu.iter())
        .map(|((aa_k, r_mu_k), r_o_mu_k)| (aa_k + instance.cc + gamma1) * r_mu_k + g1 * r_o_mu_k)
        .collect::<Vec<_>>();

    InstanceCommitments {
        tt_kappa,
        tt_kappa_e,
        tt_cc,
        tt_aa,
        tt_ss,
        tt_tt,
        tt_gamma1,
        tt_kappa_k,
    }
}

impl SpendProof {
    pub fn construct(
        params: &Parameters,
        instance: &SpendInstance,
        witness: &SpendWitness,
        verification_key: &VerificationKeyAuth,
        rr: &[Scalar],
        pay_info: &PayInfo,
        spend_value: u64,
    ) -> Self {
        let grp_params = params.grp();
        // generate random values to replace each witness
        let witness_replacement = generate_witness_replacement(params, witness);

        // compute zkp commitment for each instance
        let instance_commitments = compute_instance_commitments(
            params,
            &witness_replacement,
            instance,
            verification_key,
            rr,
        );

        let tt_aa_bytes = instance_commitments
            .tt_aa
            .iter()
            .map(|x| x.to_bytes())
            .collect::<Vec<_>>();
        let tt_ss_bytes = instance_commitments
            .tt_ss
            .iter()
            .map(|x| x.to_bytes())
            .collect::<Vec<_>>();
        let tt_tt_bytes = instance_commitments
            .tt_tt
            .iter()
            .map(|x| x.to_bytes())
            .collect::<Vec<_>>();
        let tt_kappa_k_bytes = instance_commitments
            .tt_kappa_k
            .iter()
            .map(|x| x.to_bytes())
            .collect::<Vec<_>>();

        // compute the challenge
        let challenge = compute_challenge::<ChallengeDigest, _, _>(
            std::iter::once(grp_params.gen1().to_bytes().as_ref())
                .chain(std::iter::once(grp_params.gen2().to_bytes().as_ref()))
                .chain(std::iter::once(grp_params.gammas_to_bytes().as_ref()))
                .chain(std::iter::once(verification_key.to_bytes().as_ref()))
                .chain(std::iter::once(instance.to_bytes().as_ref()))
                .chain(std::iter::once(
                    instance_commitments.tt_kappa.to_bytes().as_ref(),
                ))
                .chain(std::iter::once(
                    instance_commitments.tt_kappa_e.to_bytes().as_ref(),
                ))
                .chain(std::iter::once(
                    instance_commitments.tt_cc.to_bytes().as_ref(),
                ))
                .chain(tt_aa_bytes.iter().map(|x| x.as_ref()))
                .chain(tt_ss_bytes.iter().map(|x| x.as_ref()))
                .chain(tt_kappa_k_bytes.iter().map(|x| x.as_ref()))
                .chain(tt_tt_bytes.iter().map(|x| x.as_ref()))
                .chain(std::iter::once(pay_info.pay_info_bytes.as_ref()))
                .chain(std::iter::once(spend_value.to_le_bytes().as_ref())),
        );

        // compute response for each witness
        let responses_attributes = produce_responses(
            &witness_replacement.r_attributes,
            &challenge,
            &witness.attributes.iter().collect::<Vec<_>>(),
        );
        let response_r = produce_response(&witness_replacement.r_r, &challenge, &witness.r);
        let response_r_e = produce_response(&witness_replacement.r_r_e, &challenge, &witness.r_e);
        let response_o_c = produce_response(&witness_replacement.r_o_c, &challenge, &witness.o_c);

        let responses_r_k =
            produce_responses(&witness_replacement.r_r_lk, &challenge, &witness.r_k);
        let responses_l = produce_responses(&witness_replacement.r_lk, &challenge, &witness.lk);
        let responses_o_a = produce_responses(&witness_replacement.r_o_a, &challenge, &witness.o_a);
        let responses_mu = produce_responses(&witness_replacement.r_mu, &challenge, &witness.mu);
        let responses_o_mu =
            produce_responses(&witness_replacement.r_o_mu, &challenge, &witness.o_mu);

        SpendProof {
            challenge,
            response_r,
            response_r_e,
            responses_r_k,
            responses_l,
            responses_o_a,
            response_o_c,
            responses_mu,
            responses_o_mu,
            responses_attributes,
        }
    }

    pub fn verify(
        &self,
        params: &Parameters,
        instance: &SpendInstance,
        verification_key: &VerificationKeyAuth,
        rr: &[Scalar],
        pay_info: &PayInfo,
        spend_value: u64,
    ) -> bool {
        let grp_params = params.grp();
        let g1 = *grp_params.gen1();
        let gamma0 = *grp_params.gamma_idx(0).unwrap();

        // re-compute each zkp commitment
        let tt_kappa = instance.kappa * self.challenge
            + verification_key.alpha * (self.challenge.neg())
            + verification_key.alpha
            + grp_params.gen2() * self.response_r
            + self
                .responses_attributes
                .iter()
                .zip(verification_key.beta_g2.iter())
                .map(|(attr, beta_i)| beta_i * attr)
                .sum::<G2Projective>();

        let tt_cc = g1 * self.response_o_c
            + gamma0 * self.responses_attributes[1]
            + instance.cc * self.challenge;

        let tt_kappa_e = instance.kappa_e * self.challenge
            + verification_key.alpha * (self.challenge.neg())
            + verification_key.alpha
            + verification_key.beta_g2[0] * self.responses_attributes[2]
            + grp_params.gen2() * self.response_r_e;

        let tt_aa = self
            .responses_o_a
            .iter()
            .zip(self.responses_l.iter())
            .zip(instance.aa.iter())
            .map(|((resp_o_a_k, resp_l_k), aa_k)| {
                g1 * resp_o_a_k + gamma0 * resp_l_k + aa_k * self.challenge
            })
            .collect::<Vec<_>>();

        let tt_aa_bytes = tt_aa.iter().map(|x| x.to_bytes()).collect::<Vec<_>>();

        let tt_ss = self
            .responses_mu
            .iter()
            .zip(instance.ss.iter())
            .map(|(resp_mu_k, ss_k)| grp_params.delta() * resp_mu_k + ss_k * self.challenge)
            .collect::<Vec<_>>();

        let tt_ss_bytes = tt_ss.iter().map(|x| x.to_bytes()).collect::<Vec<_>>();

        let tt_tt = self
            .responses_mu
            .iter()
            .zip(rr.iter())
            .zip(instance.tt.iter())
            .map(|((resp_mu_k, rr_k), tt_k)| {
                g1 * self.responses_attributes[0] + (g1 * rr_k) * resp_mu_k + tt_k * self.challenge
            })
            .collect::<Vec<_>>();

        let tt_tt_bytes = tt_tt.iter().map(|x| x.to_bytes()).collect::<Vec<_>>();

        let tt_kappa_k = instance
            .kappa_k
            .iter()
            .zip(self.responses_r_k.iter())
            .zip(self.responses_l.iter())
            .map(|((kappa_k, resp_r_k), resp_r_l_k)| {
                kappa_k * self.challenge
                    + grp_params.gen2() * resp_r_k
                    + verification_key.alpha * (Scalar::one() - self.challenge)
                    + verification_key.beta_g2[0] * resp_r_l_k
            })
            .collect::<Vec<_>>();

        let tt_kappa_k_bytes = tt_kappa_k.iter().map(|x| x.to_bytes()).collect::<Vec<_>>();

        // re-compute the challenge
        let challenge = compute_challenge::<ChallengeDigest, _, _>(
            std::iter::once(grp_params.gen1().to_bytes().as_ref())
                .chain(std::iter::once(grp_params.gen2().to_bytes().as_ref()))
                .chain(std::iter::once(grp_params.gammas_to_bytes().as_ref()))
                .chain(std::iter::once(verification_key.to_bytes().as_ref()))
                .chain(std::iter::once(instance.to_bytes().as_ref()))
                .chain(std::iter::once(tt_kappa.to_bytes().as_ref()))
                .chain(std::iter::once(tt_kappa_e.to_bytes().as_ref()))
                .chain(std::iter::once(tt_cc.to_bytes().as_ref()))
                .chain(tt_aa_bytes.iter().map(|x| x.as_ref()))
                .chain(tt_ss_bytes.iter().map(|x| x.as_ref()))
                .chain(tt_kappa_k_bytes.iter().map(|x| x.as_ref()))
                .chain(tt_tt_bytes.iter().map(|x| x.as_ref()))
                .chain(std::iter::once(pay_info.pay_info_bytes.as_ref()))
                .chain(std::iter::once(spend_value.to_le_bytes().as_ref())),
        );

        challenge == self.challenge
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let challenge_bytes = self.challenge.to_bytes();
        let response_r_bytes = self.response_r.to_bytes();
        let response_r_e_bytes = self.response_r_e.to_bytes();

        let rrk_len = self.responses_r_k.len();
        let rrk_len_bytes = rrk_len.to_le_bytes();

        let rl_len = self.responses_l.len();
        let rl_len_bytes = rl_len.to_le_bytes();

        let roa_len = self.responses_o_a.len();
        let roa_len_bytes = roa_len.to_le_bytes();

        let roc_bytes = self.response_o_c.to_bytes();

        let rmu_len = self.responses_mu.len();
        let rmu_len_bytes = rmu_len.to_le_bytes();

        let romu_len = self.responses_o_mu.len();
        let romu_len_bytes = romu_len.to_le_bytes();

        let rattributes_len = self.responses_attributes.len();
        let rattributes_len_bytes = rattributes_len.to_le_bytes();

        let mut bytes: Vec<u8> = Vec::with_capacity(
            128 + (rrk_len + rl_len + roa_len + rmu_len + romu_len + rattributes_len) * 8
                + (rrk_len + rl_len + roa_len + rmu_len + romu_len + rattributes_len) * 32,
        );

        bytes.extend_from_slice(&challenge_bytes);
        bytes.extend_from_slice(&response_r_bytes);
        bytes.extend_from_slice(&response_r_e_bytes);
        bytes.extend_from_slice(&roc_bytes);

        bytes.extend_from_slice(&rrk_len_bytes);
        for rrk in &self.responses_r_k {
            bytes.extend_from_slice(&rrk.to_bytes());
        }

        bytes.extend_from_slice(&rl_len_bytes);
        for rl in &self.responses_l {
            bytes.extend_from_slice(&rl.to_bytes());
        }

        bytes.extend_from_slice(&roa_len_bytes);
        for roa in &self.responses_o_a {
            bytes.extend_from_slice(&roa.to_bytes());
        }

        bytes.extend_from_slice(&rmu_len_bytes);
        for rmu in &self.responses_mu {
            bytes.extend_from_slice(&rmu.to_bytes());
        }

        bytes.extend_from_slice(&romu_len_bytes);
        for romu in &self.responses_o_mu {
            bytes.extend_from_slice(&romu.to_bytes());
        }

        bytes.extend_from_slice(&rattributes_len_bytes);
        for rattr in &self.responses_attributes {
            bytes.extend_from_slice(&rattr.to_bytes());
        }

        bytes
    }
}

impl TryFrom<&[u8]> for SpendProof {
    type Error = CompactEcashError;

    fn try_from(bytes: &[u8]) -> Result<SpendProof> {
        if bytes.len() < 368 || (bytes.len() - 128 - 48) % 32 != 0 {
            return Err(CompactEcashError::Deserialization(
                "tried to deserialize proof of spending with bytes of invalid length".to_string(),
            ));
        }
        //SAFETY : four times slice to array conversion after a length check
        let mut idx = 0;
        #[allow(clippy::unwrap_used)]
        let challenge_bytes = bytes[idx..idx + 32].try_into().unwrap();
        idx += 32;
        #[allow(clippy::unwrap_used)]
        let response_r_bytes = bytes[idx..idx + 32].try_into().unwrap();
        idx += 32;
        #[allow(clippy::unwrap_used)]
        let response_r_e_bytes = bytes[idx..idx + 32].try_into().unwrap();
        idx += 32;
        #[allow(clippy::unwrap_used)]
        let response_o_c_bytes = bytes[idx..idx + 32].try_into().unwrap();
        idx += 32;

        let challenge = try_deserialize_scalar(
            &challenge_bytes,
            CompactEcashError::Deserialization("Failed to deserialize challenge".to_string()),
        )?;

        let response_r = try_deserialize_scalar(
            &response_r_bytes,
            CompactEcashError::Deserialization("Failed to deserialize response_r".to_string()),
        )?;

        let response_r_e = try_deserialize_scalar(
            &response_r_e_bytes,
            CompactEcashError::Deserialization("Failed to deserialize response_r_e".to_string()),
        )?;

        let response_o_c = try_deserialize_scalar(
            &response_o_c_bytes,
            CompactEcashError::Deserialization("Failed to deserialize response_o_c".to_string()),
        )?;
        //SAFETY : slice to array conversion after a length check
        #[allow(clippy::unwrap_used)]
        let rrl_len = u64::from_le_bytes(bytes[idx..idx + 8].try_into().unwrap());
        idx += 8;
        if bytes[idx..].len() < rrl_len as usize * 32 {
            return Err(CompactEcashError::Deserialization(
                "tried to deserialize response_r_l".to_string(),
            ));
        }
        let rrl_end = idx + rrl_len as usize * 32;
        let responses_r_k = try_deserialize_scalar_vec(
            rrl_len,
            &bytes[idx..rrl_end],
            CompactEcashError::Deserialization("Failed to deserialize response_r_l".to_string()),
        )?;

        //SAFETY : slice to array conversion after a length check
        #[allow(clippy::unwrap_used)]
        let rl_len = u64::from_le_bytes(bytes[rrl_end..rrl_end + 8].try_into().unwrap());
        let response_l_start = rrl_end + 8;
        if bytes[response_l_start..].len() < rl_len as usize * 32 {
            return Err(CompactEcashError::Deserialization(
                "tried to deserialize response_l".to_string(),
            ));
        }
        let rl_end = response_l_start + rl_len as usize * 32;
        let responses_l = try_deserialize_scalar_vec(
            rl_len,
            &bytes[response_l_start..rl_end],
            CompactEcashError::Deserialization("Failed to deserialize response_l".to_string()),
        )?;

        //SAFETY : slice to array conversion after a length check
        #[allow(clippy::unwrap_used)]
        let roa_len = u64::from_le_bytes(bytes[rl_end..rl_end + 8].try_into().unwrap());
        let roa_end = rl_end + 8;
        if bytes[roa_end..].len() < roa_len as usize * 32 {
            return Err(CompactEcashError::Deserialization(
                "tried to deserialize response_o_a".to_string(),
            ));
        }
        let roa_end = roa_end + roa_len as usize * 32;
        let responses_o_a = try_deserialize_scalar_vec(
            roa_len,
            &bytes[rl_end + 8..roa_end],
            CompactEcashError::Deserialization("Failed to deserialize response_o_a".to_string()),
        )?;

        //SAFETY : slice to array conversion after a length check
        #[allow(clippy::unwrap_used)]
        let response_mu_len = u64::from_le_bytes(bytes[roa_end..roa_end + 8].try_into().unwrap());
        let response_mu_end = roa_end + 8;
        if bytes[response_mu_end..].len() < response_mu_len as usize * 32 {
            return Err(CompactEcashError::Deserialization(
                "tried to deserialize response_mu".to_string(),
            ));
        }
        let response_mu_end = response_mu_end + response_mu_len as usize * 32;
        let responses_mu = try_deserialize_scalar_vec(
            response_mu_len,
            &bytes[roa_end + 8..response_mu_end],
            CompactEcashError::Deserialization("Failed to deserialize response_mu".to_string()),
        )?;

        //SAFETY : slice to array conversion after a length check
        #[allow(clippy::unwrap_used)]
        let response_o_mu_len = u64::from_le_bytes(
            bytes[response_mu_end..response_mu_end + 8]
                .try_into()
                .unwrap(),
        );
        let response_o_mu_end = response_mu_end + 8;
        if bytes[response_o_mu_end..].len() < response_o_mu_len as usize * 32 {
            return Err(CompactEcashError::Deserialization(
                "tried to deserialize response_o_mu".to_string(),
            ));
        }
        let response_o_mu_end = response_o_mu_end + response_o_mu_len as usize * 32;
        let responses_o_mu = try_deserialize_scalar_vec(
            response_o_mu_len,
            &bytes[response_mu_end + 8..response_o_mu_end],
            CompactEcashError::Deserialization("Failed to deserialize response_o_mu".to_string()),
        )?;

        //SAFETY : slice to array conversion after a length check
        #[allow(clippy::unwrap_used)]
        let response_attributes_len = u64::from_le_bytes(
            bytes[response_o_mu_end..response_o_mu_end + 8]
                .try_into()
                .unwrap(),
        );
        let response_attributes_end = response_o_mu_end + 8;
        if bytes[response_attributes_end..].len() < response_attributes_len as usize * 32 {
            return Err(CompactEcashError::Deserialization(
                "tried to deserialize response_attributes".to_string(),
            ));
        }
        let response_attributes_end =
            response_attributes_end + response_attributes_len as usize * 32;
        let responses_attributes = try_deserialize_scalar_vec(
            response_attributes_len,
            &bytes[response_o_mu_end + 8..response_attributes_end],
            CompactEcashError::Deserialization(
                "Failed to deserialize response_attributes".to_string(),
            ),
        )?;

        // Construct the SpendProof struct from the deserialized data
        let spend_proof = SpendProof {
            challenge,
            response_r,
            response_r_e,
            response_o_c,
            responses_r_k,
            responses_l,
            responses_o_a,
            responses_mu,
            responses_o_mu,
            responses_attributes,
        };

        Ok(spend_proof)
    }
}
