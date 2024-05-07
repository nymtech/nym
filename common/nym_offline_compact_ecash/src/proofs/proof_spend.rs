// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ecash_group_parameters;
use crate::proofs::{compute_challenge, produce_response, produce_responses, ChallengeDigest};
use crate::scheme::keygen::VerificationKeyAuth;
use crate::scheme::PayInfo;
use bls12_381::{G1Projective, G2Projective, Scalar};
use group::GroupEncoding;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

pub fn generate_witness_replacement(witness: &SpendWitness) -> WitnessReplacement {
    let grp_params = ecash_group_parameters();
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
    witness_replacement: &WitnessReplacement,
    instance: &SpendInstance,
    verification_key: &VerificationKeyAuth,
    rr: &[Scalar],
) -> InstanceCommitments {
    let grp_params = ecash_group_parameters();
    let g1 = *grp_params.gen1();
    //SAFETY: grp_params is static with length 3
    #[allow(clippy::unwrap_used)]
    let gamma0 = grp_params.gamma_idx(0).unwrap();
    #[allow(clippy::unwrap_used)]
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
        instance: &SpendInstance,
        witness: &SpendWitness,
        verification_key: &VerificationKeyAuth,
        rr: &[Scalar],
        pay_info: &PayInfo,
        spend_value: u64,
    ) -> Self {
        let grp_params = ecash_group_parameters();
        // generate random values to replace each witness
        let witness_replacement = generate_witness_replacement(witness);

        // compute zkp commitment for each instance
        let instance_commitments =
            compute_instance_commitments(&witness_replacement, instance, verification_key, rr);

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
        instance: &SpendInstance,
        verification_key: &VerificationKeyAuth,
        rr: &[Scalar],
        pay_info: &PayInfo,
        spend_value: u64,
    ) -> bool {
        let grp_params = ecash_group_parameters();
        let g1 = *grp_params.gen1();
        //SAFETY: grp_params is static with length 3
        #[allow(clippy::unwrap_used)]
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
}
