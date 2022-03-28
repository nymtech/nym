use std::convert::{TryFrom, TryInto};

use bls12_381::{G1Projective, G2Projective, Scalar};
use group::GroupEncoding;

use crate::error::{CompactEcashError, Result};
use crate::proofs::{ChallengeDigest, compute_challenge, produce_response, produce_responses};
use crate::scheme::keygen::{SecretKeyUser, VerificationKeyAuth};
use crate::scheme::setup::Parameters;
use crate::utils::{try_deserialize_g1_projective, try_deserialize_g2_projective};

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct SpendInstance {
    pub kappa: G2Projective,
    pub A: G1Projective,
    pub C: G1Projective,
    pub D: G1Projective,
    pub S: G1Projective,
    pub T: G1Projective,
}

impl TryFrom<&[u8]> for SpendInstance {
    type Error = CompactEcashError;

    fn try_from(bytes: &[u8]) -> Result<SpendInstance> {
        if bytes.len() < 48 * 5 + 96 || (bytes.len()) % 48 != 0 {
            return Err(CompactEcashError::DeserializationInvalidLength {
                actual: bytes.len(),
                modulus_target: bytes.len(),
                target: 48 * 5 + 96,
                modulus: 48,
                object: "spend instance".to_string(),
            });
        }

        let kappa_bytes = bytes[..96].try_into().unwrap();
        let kappa = try_deserialize_g2_projective(
            &kappa_bytes,
            CompactEcashError::Deserialization("Failed to deserialize kappa".to_string()),
        )?;
        let A_bytes = bytes[96..144].try_into().unwrap();
        let A = try_deserialize_g1_projective(
            &A_bytes,
            CompactEcashError::Deserialization("Failed to deserialize A".to_string()),
        )?;
        let C_bytes = bytes[144..192].try_into().unwrap();
        let C = try_deserialize_g1_projective(
            &C_bytes,
            CompactEcashError::Deserialization("Failed to deserialize C".to_string()),
        )?;
        let D_bytes = bytes[192..240].try_into().unwrap();
        let D = try_deserialize_g1_projective(
            &D_bytes,
            CompactEcashError::Deserialization("Failed to deserialize D".to_string()),
        )?;
        let S_bytes = bytes[240..288].try_into().unwrap();
        let S = try_deserialize_g1_projective(
            &S_bytes,
            CompactEcashError::Deserialization("Failed to deserialize S".to_string()),
        )?;
        let T_bytes = bytes[288..336].try_into().unwrap();
        let T = try_deserialize_g1_projective(
            &T_bytes,
            CompactEcashError::Deserialization("Failed to deserialize T".to_string()),
        )?;

        Ok(SpendInstance {
            kappa,
            A,
            C,
            D,
            S,
            T,
        })
    }
}

impl SpendInstance {
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(96 + 5 * 48);
        bytes.extend_from_slice(self.kappa.to_bytes().as_ref());
        bytes.extend_from_slice(self.A.to_bytes().as_ref());
        bytes.extend_from_slice(self.C.to_bytes().as_ref());
        bytes.extend_from_slice(self.D.to_bytes().as_ref());
        bytes.extend_from_slice(self.S.to_bytes().as_ref());
        bytes.extend_from_slice(self.T.to_bytes().as_ref());
        bytes
    }
}

pub struct SpendWitness {
    // includes skUser, v, t
    pub attributes: Vec<Scalar>,
    // signature randomizing element
    pub r: Scalar,
    pub l: Scalar,
    pub o_a: Scalar,
    pub o_c: Scalar,
    pub o_d: Scalar,
    pub mu: Scalar,
    pub lambda: Scalar,
    pub o_mu: Scalar,
    pub o_lambda: Scalar,

}

pub struct SpendProof {
    challenge: Scalar,
    response_r: Scalar,
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
    pub fn construct(params: &Parameters,
                     instance: &SpendInstance,
                     witness: &SpendWitness,
                     verification_key: &VerificationKeyAuth,
                     R: Scalar, ) -> Self {
        // generate random values to replace each witness
        let r_attributes = params.n_random_scalars(witness.attributes.len());
        let r_r = params.random_scalar();
        let r_l = params.random_scalar();
        let r_o_a = params.random_scalar();
        let r_o_c = params.random_scalar();
        let r_o_d = params.random_scalar();
        let r_mu = (r_attributes[1] + r_l + Scalar::from(1)).invert().unwrap();
        let r_lambda = params.random_scalar();
        let r_o_mu = ((r_o_a + r_o_c) * r_mu).neg();
        let r_o_lambda = params.random_scalar();

        let g1 = params.gen1();
        let gamma1 = params.gamma1().unwrap();
        let beta2_bytes = verification_key
            .beta_g2
            .iter()
            .map(|beta_i| beta_i.to_bytes())
            .collect::<Vec<_>>();

        // compute zkp commitment for each instance
        let zkcm_kappa = params.gen2() * r_r
            + verification_key.alpha
            + r_attributes
            .iter()
            .zip(verification_key.beta_g2.iter())
            .map(|(attr, beta_i)| beta_i * attr)
            .sum::<G2Projective>();

        let zkcm_A = g1 * r_o_a + gamma1 * r_l;
        let zkcm_C = g1 * r_o_c + gamma1 * r_attributes[1];
        let zkcm_D = g1 * r_o_d + gamma1 * r_attributes[2];
        let zkcm_S = g1 * r_mu;
        let zkcm_gamma11 = (g1 * r_o_a + gamma1 * r_l + g1 * r_o_c + gamma1 * r_attributes[1] + gamma1) * r_mu + g1 * r_o_mu;
        let zkcm_T = g1 * r_attributes[0] + (g1 * R) * r_lambda;
        let zkcm_gamma12 = (instance.A + instance.D + gamma1) * r_lambda + g1 * r_o_lambda;

        // TODO: Add also proof for l in [0, L-1]

        // compute the challenge
        let challenge = compute_challenge::<ChallengeDigest, _, _>(
            std::iter::once(params.gen1().to_bytes().as_ref())
                .chain(std::iter::once(gamma1.to_bytes().as_ref()))
                .chain(std::iter::once(verification_key.alpha.to_bytes().as_ref()))
                .chain(beta2_bytes.iter().map(|b| b.as_ref()))
                .chain(std::iter::once(instance.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_kappa.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_A.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_C.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_D.to_bytes().as_ref()))
            // .chain(std::iter::once(zkcm_S.to_bytes().as_ref()))
            // .chain(std::iter::once(zkcm_gamma11.to_bytes().as_ref()))
            // .chain(std::iter::once(zkcm_T.to_bytes().as_ref()))
            // .chain(std::iter::once(zkcm_gamma12.to_bytes().as_ref()))
        );

        // compute response for each witness
        let response_attributes = produce_responses(
            &r_attributes,
            &challenge,
            &witness.attributes.iter().collect::<Vec<_>>(),
        );
        let response_r = produce_response(&r_r, &challenge, &witness.r);
        let response_l = produce_response(&r_l, &challenge, &witness.l);
        let response_o_a = produce_response(&r_o_a, &challenge, &witness.o_a);
        let response_o_c = produce_response(&r_o_c, &challenge, &witness.o_c);
        let response_o_d = produce_response(&r_o_d, &challenge, &witness.o_d);
        // let response_mu = produce_response(&r_mu, &challenge, &witness.mu);
        let response_mu = (response_attributes[1] + response_l + Scalar::from(1)).invert().unwrap();
        let response_lambda = produce_response(&r_lambda, &challenge, &witness.lambda);
        // let response_o_mu = produce_response(&r_o_mu, &challenge, &witness.o_mu);
        let response_o_mu = ((response_o_a + response_o_c) * response_mu).neg();
        let response_o_lambda = produce_response(&r_o_lambda, &challenge, &witness.o_lambda);

        SpendProof {
            challenge,
            response_r,
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
    pub fn verify(&self,
                  params: &Parameters,
                  instance: &SpendInstance,
                  verification_key: &VerificationKeyAuth,
                  R: Scalar) -> bool {
        let g1 = params.gen1();
        let gamma1 = params.gamma1().unwrap();
        let beta2_bytes = verification_key
            .beta_g2
            .iter()
            .map(|beta_i| beta_i.to_bytes())
            .collect::<Vec<_>>();

        // re-compute each zkp commitment
        let zkcm_kappa = instance.kappa * self.challenge
            + params.gen2() * self.response_r
            + verification_key.alpha * (Scalar::one() - self.challenge)
            + self.response_attributes
            .iter()
            .zip(verification_key.beta_g2.iter())
            .map(|(attr, beta_i)| beta_i * attr)
            .sum::<G2Projective>();

        let zkcm_A = g1 * self.response_o_a + gamma1 * self.response_l + instance.A * self.challenge;
        let zkcm_C = g1 * self.response_o_c + gamma1 * self.response_attributes[1] + instance.C * self.challenge;
        let zkcm_D = g1 * self.response_o_d + gamma1 * self.response_attributes[2] + instance.D * self.challenge;
        let zkcm_S = g1 * self.response_mu + instance.S * self.challenge;
        let zkcm_gamma11 = (g1 * self.response_o_a + gamma1 * self.response_l + g1 * self.response_o_c + gamma1 * self.response_attributes[1] + gamma1) * self.response_mu + g1 * self.response_o_mu + gamma1 * self.challenge;
        let zkcm_T = g1 * self.response_attributes[0] + (g1 * R) * self.response_lambda + instance.T * self.challenge;
        let zkcm_gamma12 = (instance.A + instance.D + gamma1) * self.response_lambda + g1 * self.response_o_lambda + gamma1 * self.challenge;

        // re-compute the challenge
        let challenge = compute_challenge::<ChallengeDigest, _, _>(
            std::iter::once(params.gen1().to_bytes().as_ref())
                .chain(std::iter::once(gamma1.to_bytes().as_ref()))
                .chain(std::iter::once(verification_key.alpha.to_bytes().as_ref()))
                .chain(beta2_bytes.iter().map(|b| b.as_ref()))
                .chain(std::iter::once(instance.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_kappa.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_A.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_C.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_D.to_bytes().as_ref()))
            // .chain(std::iter::once(zkcm_S.to_bytes().as_ref()))
            // .chain(std::iter::once(zkcm_gamma11.to_bytes().as_ref()))
            // .chain(std::iter::once(zkcm_T.to_bytes().as_ref()))
            // .chain(std::iter::once(zkcm_gamma12.to_bytes().as_ref()))
        );

        challenge == self.challenge
    }
}

#[cfg(test)]
mod tests {
    use bls12_381::{G2Projective, Scalar};
    use rand::thread_rng;

    use crate::proofs::proof_spend::{SpendInstance, SpendProof, SpendWitness};
    use crate::scheme::aggregation::aggregate_verification_keys;
    use crate::scheme::keygen::{PublicKeyUser, ttp_keygen, VerificationKeyAuth};
    use crate::scheme::setup::Parameters;
    use crate::scheme::spend::{PayInfo, pseudorandom_fgt, pseudorandom_fgv};
    use crate::utils::hash_to_scalar;

    #[test]
    fn spend_proof_construct_and_verify() {
        let rng = thread_rng();
        let params = Parameters::new().unwrap();
        let sk = params.random_scalar();
        let pk_user = PublicKeyUser {
            pk: params.gen1() * sk,
        };
        let authorities_keypairs = ttp_keygen(&params, 2, 3).unwrap();
        let verification_keys_auth: Vec<VerificationKeyAuth> = authorities_keypairs
            .iter()
            .map(|keypair| keypair.verification_key())
            .collect();

        let verification_key = aggregate_verification_keys(&verification_keys_auth, Some(&[1, 2, 3])).unwrap();

        let v = params.random_scalar();
        let t = params.random_scalar();
        let attributes = vec![sk, v, t];
        let l = 5;
        let gamma1 = params.gamma1().unwrap();
        let g1 = params.gen1();

        let r = params.random_scalar();
        let kappa = params.gen2() * r
            + verification_key.alpha
            + attributes
            .iter()
            .zip(verification_key.beta_g2.iter())
            .map(|(priv_attr, beta_i)| beta_i * priv_attr)
            .sum::<G2Projective>();

        let o_a = params.random_scalar();
        let o_c = params.random_scalar();
        let o_d = params.random_scalar();

        // compute commitments A, C, D
        let A = g1 * o_a + gamma1 * Scalar::from(l);
        let C = g1 * o_c + gamma1 * v;
        let D = g1 * o_d + gamma1 * t;

        // compute hash of the payment info
        let payInfo = PayInfo { info: [5u8; 32] };
        let R = hash_to_scalar(payInfo.info);

        // evaluate the pseudorandom functions
        let S = pseudorandom_fgv(&params, v, l);
        let T = g1 * sk + pseudorandom_fgt(&params, t, l) * R;

        // compute values mu, o_mu, lambda, o_lambda
        let mu: Scalar = (v + Scalar::from(l) + Scalar::from(1)).invert().unwrap();
        let o_mu = ((o_a + o_c) * mu).neg();
        let lambda = (t + Scalar::from(l) + Scalar::from(1)).invert().unwrap();
        let o_lambda = ((o_a + o_d) * lambda).neg();

        let instance = SpendInstance {
            kappa,
            A,
            C,
            D,
            S,
            T,
        };

        let witness = SpendWitness {
            attributes,
            r,
            l: Scalar::from(l),
            o_a,
            o_c,
            o_d,
            mu,
            lambda,
            o_mu,
            o_lambda,
        };
        let zk_proof = SpendProof::construct(&params, &instance, &witness, &verification_key, R);
        assert!(zk_proof.verify(&params, &instance, &verification_key, R))
    }
}