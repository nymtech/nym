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
    pub cc: G1Projective,
    pub aa: Vec<G1Projective>,
    pub ss: Vec<G1Projective>,
    pub tt: Vec<G1Projective>,
    pub kappa_k: Vec<G2Projective>,
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

        let mut j = 0;
        let kappa_bytes = bytes[j..j + 96].try_into().unwrap();
        let kappa = try_deserialize_g2_projective(
            &kappa_bytes,
            CompactEcashError::Deserialization("Failed to deserialize kappa".to_string()),
        )?;
        j += 96;

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

            let aa_elem_bytes = bytes[start..end].try_into().unwrap();
            let aa_elem = try_deserialize_g1_projective(
                &aa_elem_bytes,
                CompactEcashError::Deserialization(
                    "Failed to deserialize compressed A values".to_string(),
                ),
            )?;

            aa.push(aa_elem)
        }
        j += j + a_len as usize * 48;


        let cc_bytes = bytes[j..j + 48].try_into().unwrap();
        let cc = try_deserialize_g1_projective(
            &cc_bytes,
            CompactEcashError::Deserialization("Failed to deserialize C".to_string()),
        )?;
        j += 48;

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

            let ss_elem_bytes = bytes[start..end].try_into().unwrap();
            let ss_elem = try_deserialize_g1_projective(
                &ss_elem_bytes,
                CompactEcashError::Deserialization(
                    "Failed to deserialize compressed S values".to_string(),
                ),
            )?;

            ss.push(ss_elem)
        }
        j += j + s_len as usize * 48;

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

            let tt_elem_bytes = bytes[start..end].try_into().unwrap();
            let tt_elem = try_deserialize_g1_projective(
                &tt_elem_bytes,
                CompactEcashError::Deserialization(
                    "Failed to deserialize compressed T values".to_string(),
                ),
            )?;

            tt.push(tt_elem)
        }
        j += j + t_len as usize * 48;


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
        })
    }
}

impl SpendInstance {
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Default::default();
        bytes.extend_from_slice(self.kappa.to_bytes().as_ref());

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

pub struct SpendWitness {
    // includes skUser, v, t
    pub attributes: Vec<Scalar>,
    // signature randomizing element
    pub r: Scalar,
    pub o_c: Scalar,
    pub lk: Vec<Scalar>,
    pub o_a: Vec<Scalar>,
    pub mu: Vec<Scalar>,
    pub o_mu: Vec<Scalar>,
    pub r_k: Vec<Scalar>,
}

#[derive(Debug, Clone)]
pub struct SpendProof {
    challenge: Scalar,
    response_r: Scalar,
    response_r_l: Vec<Scalar>,
    response_l: Vec<Scalar>,
    response_o_a: Vec<Scalar>,
    response_o_c: Scalar,
    response_mu: Vec<Scalar>,
    response_o_mu: Vec<Scalar>,
    response_attributes: Vec<Scalar>,
}

impl SpendProof {
    pub fn construct(
        params: &Parameters,
        instance: &SpendInstance,
        witness: &SpendWitness,
        verification_key: &VerificationKeyAuth,
        rr: &[Scalar],
    ) -> Self {
        let grparams = params.grp();
        // generate random values to replace each witness
        let r_attributes = grparams.n_random_scalars(witness.attributes.len());
        let r_sk = r_attributes[0];
        let r_v = r_attributes[1];
        let r_r = grparams.random_scalar();
        let r_o_c = grparams.random_scalar();

        let r_r_lk = grparams.n_random_scalars(witness.r_k.len());
        let r_lk = grparams.n_random_scalars(witness.lk.len());
        let r_o_a = grparams.n_random_scalars(witness.o_a.len());
        let r_mu = grparams.n_random_scalars(witness.mu.len());
        let r_o_mu = grparams.n_random_scalars(witness.o_mu.len());

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

        let zkcm_cc = g1 * r_o_c + gamma1 * r_v;

        let zkcm_aa: Vec<G1Projective> =
            r_o_a
                .iter()
                .zip(r_lk.iter()).map(|(r_o_a_k, r_l_k)| g1 * r_o_a_k + gamma1 * r_l_k)
                .collect::<Vec<_>>();

        let zkcm_aa_bytes = zkcm_aa
            .iter()
            .map(|x| x.to_bytes())
            .collect::<Vec<_>>();

        let zkcm_ss = r_mu.iter().map(|r_mu_k| grparams.delta() * r_mu_k).collect::<Vec<_>>();

        let zkcm_ss_bytes = zkcm_ss
            .iter()
            .map(|x| x.to_bytes())
            .collect::<Vec<_>>();

        let zkcm_tt = rr
            .iter()
            .zip(r_mu.iter()).map(|(rr_k, r_mu_k)| g1 * r_sk + (g1 * rr_k) * r_mu_k).collect::<Vec<_>>();

        let zkcm_tt_bytes = zkcm_tt
            .iter()
            .map(|x| x.to_bytes())
            .collect::<Vec<_>>();

        let zkcm_gamma11 = instance.aa
            .iter()
            .zip(r_mu.iter())
            .zip(r_o_mu.iter())
            .map(|((aa_k, r_mu_k), r_o_mu_k)| (aa_k + instance.cc + gamma1) * r_mu_k + g1 * r_o_mu_k)
            .collect::<Vec<_>>();

        let zkcm_gamma11_bytes = zkcm_gamma11
            .iter()
            .map(|x| x.to_bytes())
            .collect::<Vec<_>>();

        let zkcm_kappa_k = r_lk.iter()
            .zip(r_r_lk.iter())
            .map(|(r_k, r_r_k)| params.pkRP().alpha + params.pkRP().beta * r_k + grparams.gen2() * r_r_k)
            .collect::<Vec<_>>();

        let zkcm_kappa_k_bytes = zkcm_kappa_k
            .iter()
            .map(|x| x.to_bytes())
            .collect::<Vec<_>>();

        // compute the challenge
        let challenge = compute_challenge::<ChallengeDigest, _, _>(
            std::iter::once(grparams.gen1().to_bytes().as_ref())
                .chain(std::iter::once(gamma1.to_bytes().as_ref()))
                .chain(std::iter::once(verification_key.alpha.to_bytes().as_ref()))
                .chain(beta2_bytes.iter().map(|b| b.as_ref()))
                .chain(std::iter::once(instance.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_kappa.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_cc.to_bytes().as_ref()))
                .chain(zkcm_aa_bytes.iter().map(|x| x.as_ref()))
                .chain(zkcm_ss_bytes.iter().map(|x| x.as_ref()))
                .chain(zkcm_kappa_k_bytes.iter().map(|x| x.as_ref()))
                .chain(zkcm_tt_bytes.iter().map(|x| x.as_ref()))
                .chain(zkcm_gamma11_bytes.iter().map(|x| x.as_ref()))
        );

        // compute response for each witness
        let response_attributes = produce_responses(
            &r_attributes,
            &challenge,
            &witness.attributes.iter().collect::<Vec<_>>(),
        );
        let response_r = produce_response(&r_r, &challenge, &witness.r);
        let response_r_l = produce_responses(&r_r_lk, &challenge, &witness.r_k);
        let response_l = produce_responses(&r_lk, &challenge, &witness.lk);
        let response_o_a = produce_responses(&r_o_a, &challenge, &witness.o_a);
        let response_o_c = produce_response(&r_o_c, &challenge, &witness.o_c);

        let response_mu = produce_responses(&r_mu, &challenge, &witness.mu);
        let response_o_mu = produce_responses(&r_o_mu, &challenge, &witness.o_mu);

        SpendProof {
            challenge,
            response_r,
            response_r_l,
            response_l,
            response_o_a,
            response_o_c,
            response_mu,
            response_o_mu,
            response_attributes,
        }
    }
    pub fn verify(
        &self,
        params: &Parameters,
        instance: &SpendInstance,
        verification_key: &VerificationKeyAuth,
        rr: &[Scalar],
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


        let zkcm_aa = self.response_o_a
            .iter()
            .zip(self.response_l.iter())
            .zip(instance.aa.iter())
            .map(|((resp_o_a_k, resp_l_k), aa_k)| g1 * resp_o_a_k + gamma1 * resp_l_k + aa_k * self.challenge)
            .collect::<Vec<_>>();

        let zkcm_aa_bytes = zkcm_aa
            .iter()
            .map(|x| x.to_bytes())
            .collect::<Vec<_>>();

        let zkcm_cc = g1 * self.response_o_c
            + gamma1 * self.response_attributes[1]
            + instance.cc * self.challenge;

        let zkcm_ss = self.response_mu
            .iter()
            .zip(instance.ss.iter())
            .map(|(resp_mu_k, ss_k)| grparams.delta() * resp_mu_k + ss_k * self.challenge)
            .collect::<Vec<_>>();

        let zkcm_ss_bytes = zkcm_ss
            .iter()
            .map(|x| x.to_bytes())
            .collect::<Vec<_>>();

        let zkcm_tt = self.response_mu
            .iter()
            .zip(rr.iter())
            .zip(instance.tt.iter())
            .map(|((resp_mu_k, rr_k), tt_k)| g1 * self.response_attributes[0] + (g1 * rr_k) * resp_mu_k + tt_k * self.challenge)
            .collect::<Vec<_>>();

        let zkcm_tt_bytes = zkcm_tt
            .iter()
            .map(|x| x.to_bytes())
            .collect::<Vec<_>>();

        let zkcm_gamma11 = instance.aa
            .iter()
            .zip(self.response_mu.iter())
            .zip(self.response_o_mu.iter())
            .map(|((aa_k, resp_mu_k), resp_o_mu_k)| (aa_k + instance.cc + gamma1) * resp_mu_k
                + g1 * resp_o_mu_k + gamma1 * self.challenge)
            .collect::<Vec<_>>();

        let zkcm_gamma11_bytes = zkcm_gamma11
            .iter()
            .map(|x| x.to_bytes())
            .collect::<Vec<_>>();


        let zkcm_kappa_k = instance.kappa_k
            .iter()
            .zip(self.response_r_l.iter())
            .zip(self.response_l.iter())
            .map(|((kappa_k, resp_r_k), resp_r_l_k)| kappa_k * self.challenge + grparams.gen2() * resp_r_k + params.pkRP().alpha * (Scalar::one() - self.challenge) + params.pkRP().beta * resp_r_l_k)
            .collect::<Vec<_>>();

        let zkcm_kappa_k_bytes = zkcm_kappa_k
            .iter()
            .map(|x| x.to_bytes())
            .collect::<Vec<_>>();

        // re-compute the challenge
        let challenge = compute_challenge::<ChallengeDigest, _, _>(
            std::iter::once(grparams.gen1().to_bytes().as_ref())
                .chain(std::iter::once(gamma1.to_bytes().as_ref()))
                .chain(std::iter::once(verification_key.alpha.to_bytes().as_ref()))
                .chain(beta2_bytes.iter().map(|b| b.as_ref()))
                .chain(std::iter::once(instance.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_kappa.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_cc.to_bytes().as_ref()))
                .chain(zkcm_aa_bytes.iter().map(|x| x.as_ref()))
                .chain(zkcm_ss_bytes.iter().map(|x| x.as_ref()))
                .chain(zkcm_kappa_k_bytes.iter().map(|x| x.as_ref()))
                .chain(zkcm_tt_bytes.iter().map(|x| x.as_ref()))
                .chain(zkcm_gamma11_bytes.iter().map(|x| x.as_ref()))
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
    use crate::scheme::{pseudorandom_f_delta_v, pseudorandom_f_g_v};
    use crate::scheme::aggregation::aggregate_verification_keys;
    use crate::scheme::keygen::{PublicKeyUser, ttp_keygen, VerificationKeyAuth};
    use crate::scheme::PayInfo;
    use crate::scheme::setup::{GroupParameters, setup};
    use crate::utils::hash_to_scalar;

    #[test]
    fn spend_proof_construct_and_verify() {
        let rng = thread_rng();
        let L = 32;
        let params = setup(L);
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

        // compute commitments A, C, D
        let aa = g1 * o_a + gamma1 * Scalar::from(l);
        let cc = g1 * o_c + gamma1 * v;

        // compute hash of the payment info
        let pay_info = PayInfo { info: [37u8; 32] };
        let rr = hash_to_scalar(pay_info.info);

        // evaluate the pseudorandom functions
        let ss = pseudorandom_f_delta_v(&grparams, v, l);
        let tt = g1 * sk + pseudorandom_f_g_v(&grparams, v, l) * rr;

        // compute values mu, o_mu, lambda, o_lambda
        let mu: Scalar = (v + Scalar::from(l) + Scalar::from(1)).invert().unwrap();
        let o_mu = ((o_a + o_c) * mu).neg();

        // parse the signature associated with value l
        let sign_l = params.get_sign_by_idx(l).unwrap();
        // randomise the signature associated with value l
        let (sign_l_prime, r_l) = sign_l.randomise(grparams);
        // compute kappa_l
        let kappa_k =
            grparams.gen2() * r_l + params.pkRP().alpha + params.pkRP().beta * Scalar::from(l);

        let instance = SpendInstance {
            kappa,
            aa: vec![aa],
            cc,
            ss: vec![ss],
            tt: vec![tt],
            kappa_k: vec![kappa_k],
        };

        let witness = SpendWitness {
            attributes,
            r,
            o_c,
            lk: vec![Scalar::from(l)],
            o_a: vec![o_a],
            mu: vec![mu],
            o_mu: vec![o_mu],
            r_k: vec![r_l],
        };

        let zk_proof = SpendProof::construct(&params, &instance, &witness, &verification_key, &[rr]);
        assert!(zk_proof.verify(&params, &instance, &verification_key, &[rr]))
    }
}
