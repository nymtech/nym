use std::convert::{TryFrom, TryInto};
use std::ops::Neg;

use bls12_381::{G1Projective, G2Projective, Scalar};
use group::{Curve, Group, GroupEncoding};

use crate::error::{CompactEcashError, Result};
use crate::proofs::{ChallengeDigest, compute_challenge, produce_response, produce_responses};
use crate::scheme::keygen::{SecretKeyUser, VerificationKeyAuth};
use crate::scheme::setup::{GroupParameters, Parameters};
use crate::utils::{try_deserialize_g1_projective, try_deserialize_g2_projective};

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct SpendInstance {
    pub kappa: G2Projective,
    pub aa: G1Projective,
    pub cc: G1Projective,
    pub dd: G1Projective,
    pub ss: G1Projective,
    pub tt: G1Projective,
    pub kappa_l: G2Projective,
}

impl TryFrom<&[u8]> for SpendInstance {
    type Error = CompactEcashError;

    fn try_from(bytes: &[u8]) -> Result<SpendInstance> {
        if bytes.len() < 48 * 5 + 2 * 96 || (bytes.len()) % 48 != 0 {
            return Err(CompactEcashError::DeserializationInvalidLength {
                actual: bytes.len(),
                modulus_target: bytes.len(),
                target: 48 * 5 + 2 * 96,
                modulus: 48,
                object: "spend instance".to_string(),
            });
        }

        let kappa_bytes = bytes[..96].try_into().unwrap();
        let kappa = try_deserialize_g2_projective(
            &kappa_bytes,
            CompactEcashError::Deserialization("Failed to deserialize kappa".to_string()),
        )?;
        let aa_bytes = bytes[96..144].try_into().unwrap();
        let aa = try_deserialize_g1_projective(
            &aa_bytes,
            CompactEcashError::Deserialization("Failed to deserialize A".to_string()),
        )?;
        let cc_bytes = bytes[144..192].try_into().unwrap();
        let cc = try_deserialize_g1_projective(
            &cc_bytes,
            CompactEcashError::Deserialization("Failed to deserialize C".to_string()),
        )?;
        let dd_bytes = bytes[192..240].try_into().unwrap();
        let dd = try_deserialize_g1_projective(
            &dd_bytes,
            CompactEcashError::Deserialization("Failed to deserialize D".to_string()),
        )?;
        let ss_bytes = bytes[240..288].try_into().unwrap();
        let ss = try_deserialize_g1_projective(
            &ss_bytes,
            CompactEcashError::Deserialization("Failed to deserialize S".to_string()),
        )?;
        let tt_bytes = bytes[288..336].try_into().unwrap();
        let tt = try_deserialize_g1_projective(
            &tt_bytes,
            CompactEcashError::Deserialization("Failed to deserialize T".to_string()),
        )?;
        let kappa_l_bytes = bytes[336..432].try_into().unwrap();
        let kappa_l = try_deserialize_g2_projective(
            &kappa_l_bytes,
            CompactEcashError::Deserialization("Failed to deserialize kappa_l".to_string()),
        )?;

        Ok(SpendInstance {
            kappa,
            aa,
            cc,
            dd,
            ss,
            tt,
            kappa_l,
        })
    }
}

impl SpendInstance {
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(2 * 96 + 5 * 48);
        bytes.extend_from_slice(self.kappa.to_bytes().as_ref());
        bytes.extend_from_slice(self.aa.to_bytes().as_ref());
        bytes.extend_from_slice(self.cc.to_bytes().as_ref());
        bytes.extend_from_slice(self.dd.to_bytes().as_ref());
        bytes.extend_from_slice(self.ss.to_bytes().as_ref());
        bytes.extend_from_slice(self.tt.to_bytes().as_ref());
        bytes.extend_from_slice(self.kappa_l.to_bytes().as_ref());
        bytes
    }
}

pub struct SpendWitness {
    // includes skUser, v, t
    pub attributes: Vec<Scalar>,
    // signature randomizing element
    pub r: Scalar,
    pub r_l: Scalar,
    pub l: Scalar,
    pub o_a: Scalar,
    pub o_c: Scalar,
    pub o_d: Scalar,
    pub mu: Scalar,
    pub lambda: Scalar,
    pub o_mu: Scalar,
    pub o_lambda: Scalar,
}

#[derive(Debug, Clone)]
pub struct SpendProof {
    challenge: Scalar,
    response_r: Scalar,
    response_r_l: Scalar,
    response_l: Scalar,
    response_o_a: Scalar,
    response_o_c: Scalar,
    response_o_d: Scalar,
    response_mu: Scalar,
    response_lambda: Scalar,
    response_o_mu: Scalar,
    response_o_lambda: Scalar,
    response_attributes: Vec<Scalar>,
}

impl SpendProof {
    pub fn construct(
        params: &Parameters,
        instance: &SpendInstance,
        witness: &SpendWitness,
        verification_key: &VerificationKeyAuth,
        R: Scalar,
    ) -> Self {
        let grparams = params.grp();
        // generate random values to replace each witness
        let r_attributes = grparams.n_random_scalars(witness.attributes.len());
        let r_sk = r_attributes[0];
        let r_v = r_attributes[1];
        let r_t = r_attributes[2];
        let r_r = grparams.random_scalar();
        let r_r_l = grparams.random_scalar();
        let r_l = grparams.random_scalar();
        let r_o_a = grparams.random_scalar();
        let r_o_c = grparams.random_scalar();
        let r_o_d = grparams.random_scalar();
        let r_mu = grparams.random_scalar();
        let r_lambda = grparams.random_scalar();
        let r_o_mu = grparams.random_scalar();
        let r_o_lambda = grparams.random_scalar();

        let g1 = *grparams.gen1();
        let gamma1 = *grparams.gamma1();
        let beta2_bytes = verification_key
            .beta_g2
            .iter()
            .map(|beta_i| beta_i.to_bytes())
            .collect::<Vec<_>>();

        // compute zkp commitment for each instance
        let zkcm_kappa = grparams.gen2() * r_r
            + verification_key.alpha
            + r_attributes
            .iter()
            .zip(verification_key.beta_g2.iter())
            .map(|(attr, beta_i)| beta_i * attr)
            .sum::<G2Projective>();

        let zkcm_A = g1 * r_o_a + gamma1 * r_l;
        let zkcm_C = g1 * r_o_c + gamma1 * r_v;
        let zkcm_D = g1 * r_o_d + gamma1 * r_t;
        let zkcm_S = g1 * r_mu;
        let zkcm_gamma11 = (instance.aa + instance.cc + gamma1) * r_mu + g1 * r_o_mu;
        let zkcm_T = g1 * r_sk + (g1 * R) * r_lambda;
        let zkcm_gamma12 = (instance.aa + instance.dd + gamma1) * r_lambda + g1 * r_o_lambda;
        let zkcm_kappa_l = grparams.gen2() * r_r_l + params.pkRP().alpha + params.pkRP().beta * r_l;

        // compute the challenge
        let challenge = compute_challenge::<ChallengeDigest, _, _>(
            std::iter::once(grparams.gen1().to_bytes().as_ref())
                .chain(std::iter::once(gamma1.to_bytes().as_ref()))
                .chain(std::iter::once(verification_key.alpha.to_bytes().as_ref()))
                .chain(beta2_bytes.iter().map(|b| b.as_ref()))
                .chain(std::iter::once(instance.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_kappa.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_A.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_C.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_D.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_S.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_kappa_l.to_bytes().as_ref()))
                .chain(std::iter::once(
                    zkcm_gamma11.to_affine().to_bytes().as_ref(),
                ))
                .chain(std::iter::once(zkcm_T.to_bytes().as_ref()))
                .chain(std::iter::once(
                    zkcm_gamma12.to_affine().to_bytes().as_ref(),
                )),
        );

        // compute response for each witness
        let response_attributes = produce_responses(
            &r_attributes,
            &challenge,
            &witness.attributes.iter().collect::<Vec<_>>(),
        );
        let response_r = produce_response(&r_r, &challenge, &witness.r);
        let response_r_l = produce_response(&r_r_l, &challenge, &witness.r_l);
        let response_l = produce_response(&r_l, &challenge, &witness.l);
        let response_o_a = produce_response(&r_o_a, &challenge, &witness.o_a);
        let response_o_c = produce_response(&r_o_c, &challenge, &witness.o_c);
        let response_o_d = produce_response(&r_o_d, &challenge, &witness.o_d);

        let response_mu = produce_response(&r_mu, &challenge, &witness.mu);
        let response_lambda = produce_response(&r_lambda, &challenge, &witness.lambda);
        let response_o_mu = produce_response(&r_o_mu, &challenge, &witness.o_mu);
        let response_o_lambda = produce_response(&r_o_lambda, &challenge, &witness.o_lambda);

        SpendProof {
            challenge,
            response_r,
            response_r_l,
            response_l,
            response_o_a,
            response_o_c,
            response_o_d,
            response_mu,
            response_lambda,
            response_o_mu,
            response_o_lambda,
            response_attributes,
        }
    }
    pub fn verify(
        &self,
        params: &Parameters,
        instance: &SpendInstance,
        verification_key: &VerificationKeyAuth,
        R: Scalar,
    ) -> bool {
        let grparams = params.grp();
        let g1 = *grparams.gen1();
        let gamma1 = *grparams.gamma1();
        let beta2_bytes = verification_key
            .beta_g2
            .iter()
            .map(|beta_i| beta_i.to_bytes())
            .collect::<Vec<_>>();

        // re-compute each zkp commitment
        let zkcm_kappa = instance.kappa * self.challenge
            + grparams.gen2() * self.response_r
            + verification_key.alpha * (Scalar::one() - self.challenge)
            + self
            .response_attributes
            .iter()
            .zip(verification_key.beta_g2.iter())
            .map(|(attr, beta_i)| beta_i * attr)
            .sum::<G2Projective>();

        let zkcm_A =
            g1 * self.response_o_a + gamma1 * self.response_l + instance.aa * self.challenge;
        let zkcm_C = g1 * self.response_o_c
            + gamma1 * self.response_attributes[1]
            + instance.cc * self.challenge;
        let zkcm_D = g1 * self.response_o_d
            + gamma1 * self.response_attributes[2]
            + instance.dd * self.challenge;
        let zkcm_S = g1 * self.response_mu + instance.ss * self.challenge;
        let zkcm_gamma11 = (instance.aa + instance.cc + gamma1) * self.response_mu
            + g1 * self.response_o_mu
            + gamma1 * self.challenge;
        let zkcm_T = g1 * self.response_attributes[0]
            + (g1 * R) * self.response_lambda
            + instance.tt * self.challenge;
        let zkcm_gamma12 = (instance.aa + instance.dd + gamma1) * self.response_lambda
            + g1 * self.response_o_lambda
            + gamma1 * self.challenge;

        let zkcm_kappa_l = instance.kappa_l * self.challenge
            + grparams.gen2() * self.response_r_l
            + params.pkRP().alpha * (Scalar::one() - self.challenge)
            + params.pkRP().beta * self.response_l;

        // re-compute the challenge
        let challenge = compute_challenge::<ChallengeDigest, _, _>(
            std::iter::once(grparams.gen1().to_bytes().as_ref())
                .chain(std::iter::once(gamma1.to_bytes().as_ref()))
                .chain(std::iter::once(verification_key.alpha.to_bytes().as_ref()))
                .chain(beta2_bytes.iter().map(|b| b.as_ref()))
                .chain(std::iter::once(instance.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_kappa.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_A.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_C.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_D.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_S.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_kappa_l.to_bytes().as_ref()))
                .chain(std::iter::once(
                    zkcm_gamma11.to_affine().to_bytes().as_ref(),
                ))
                .chain(std::iter::once(zkcm_T.to_bytes().as_ref()))
                .chain(std::iter::once(
                    zkcm_gamma12.to_affine().to_bytes().as_ref(),
                )),
        );

        challenge == self.challenge
    }
}

#[cfg(test)]
mod tests {
    use bls12_381::{G1Projective, G2Projective, Scalar};
    use group::Curve;
    use rand::{Rng, thread_rng};

    use crate::proofs::proof_spend::{SpendInstance, SpendProof, SpendWitness};
    use crate::scheme::{pseudorandom_fgt, pseudorandom_fgv};
    use crate::scheme::aggregation::aggregate_verification_keys;
    use crate::scheme::keygen::{PublicKeyUser, ttp_keygen, VerificationKeyAuth};
    use crate::scheme::PayInfo;
    use crate::scheme::setup::{GroupParameters, setup};
    use crate::utils::hash_to_scalar;

    #[test]
    fn spend_proof_construct_and_verify() {
        let rng = thread_rng();
        let params = setup();
        let grparams = params.grp();
        let sk = grparams.random_scalar();
        let pk_user = PublicKeyUser {
            pk: grparams.gen1() * sk,
        };
        let authorities_keypairs = ttp_keygen(&grparams, 2, 3).unwrap();
        let verification_keys_auth: Vec<VerificationKeyAuth> = authorities_keypairs
            .iter()
            .map(|keypair| keypair.verification_key())
            .collect();

        let verification_key =
            aggregate_verification_keys(&verification_keys_auth, Some(&[1, 2, 3])).unwrap();

        let v = grparams.random_scalar();
        let t = grparams.random_scalar();
        let attributes = vec![sk, v, t];
        // the below value must be from range 0 to params.L()
        let l = 5;
        let gamma1 = *grparams.gamma1();
        let g1 = *grparams.gen1();

        let r = grparams.random_scalar();
        let kappa = grparams.gen2() * r
            + verification_key.alpha
            + attributes
            .iter()
            .zip(verification_key.beta_g2.iter())
            .map(|(priv_attr, beta_i)| beta_i * priv_attr)
            .sum::<G2Projective>();

        let o_a = grparams.random_scalar();
        let o_c = grparams.random_scalar();
        let o_d = grparams.random_scalar();

        // compute commitments A, C, D
        let aa = g1 * o_a + gamma1 * Scalar::from(l);
        let cc = g1 * o_c + gamma1 * v;
        let dd = g1 * o_d + gamma1 * t;

        // compute hash of the payment info
        let pay_info = PayInfo { info: [37u8; 32] };
        let rr = hash_to_scalar(pay_info.info);

        // evaluate the pseudorandom functions
        let ss = pseudorandom_fgv(&grparams, v, l);
        let tt = g1 * sk + pseudorandom_fgt(&grparams, t, l) * rr;

        // compute values mu, o_mu, lambda, o_lambda
        let mu: Scalar = (v + Scalar::from(l) + Scalar::from(1)).invert().unwrap();
        let o_mu = ((o_a + o_c) * mu).neg();
        let lambda = (t + Scalar::from(l) + Scalar::from(1)).invert().unwrap();
        let o_lambda = ((o_a + o_d) * lambda).neg();

        // parse the signature associated with value l
        let sign_l = params.get_sign_by_idx(l).unwrap();
        // randomise the signature associated with value l
        let (sign_l_prime, r_l) = sign_l.randomise(grparams);
        // compute kappa_l
        let kappa_l =
            grparams.gen2() * r_l + params.pkRP().alpha + params.pkRP().beta * Scalar::from(l);

        let instance = SpendInstance {
            kappa,
            aa,
            cc,
            dd,
            ss,
            tt,
            kappa_l,
        };

        let witness = SpendWitness {
            attributes,
            r,
            r_l,
            l: Scalar::from(l),
            o_a,
            o_c,
            o_d,
            mu,
            lambda,
            o_mu,
            o_lambda,
        };
        let zk_proof = SpendProof::construct(&params, &instance, &witness, &verification_key, rr);
        assert!(zk_proof.verify(&params, &instance, &verification_key, rr))
    }
}
