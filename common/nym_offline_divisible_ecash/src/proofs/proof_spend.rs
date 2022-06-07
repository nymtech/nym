use std::convert::TryFrom;
use std::ops::Neg;

use bls12_381::{G1Projective, G2Projective, Gt, Scalar};
use group::GroupEncoding;

use crate::proofs::{ChallengeDigest, compute_challenge, produce_response, produce_responses};
use crate::scheme::{Phi, VarPhi, Wallet};
use crate::scheme::keygen::{SecretKeyUser, VerificationKeyAuth};
use crate::scheme::setup::Parameters;

pub struct SpendInstance {
    pub kappa: G2Projective,
    pub phi: Phi,
    pub varphi: VarPhi,
    pub rr: Scalar,
    pub rr_prime: G1Projective,
    pub ss: G1Projective,
    pub tt: G2Projective,
    pub varsig_prime1: G1Projective,
    pub theta_prime1: G1Projective,
    pub pg_eq1: Gt,
    pub pg_eq2: Gt,
    pub pg_eq3: Gt,
    pub pg_eq4: Gt,
    pub psi_g1: G1Projective,
    pub psi_g2: G2Projective,
    pub pg_psi0_delta: Gt,
    pub pg_psi0_gen2: Gt,
    pub pg_psi0_yy: Gt,
    pub pg_psi0_ww1: Gt,
    pub pg_psi0_ww2: Gt,
    pub pg_rr_psi1: Gt,
    pub pg_psi0_tt: Gt,
    pub pg_psi0_psi1: Gt,
}

impl SpendInstance {
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(96 + 96 + 3 * 96 + 5 * 48 + 12 * 288);
        bytes.extend_from_slice(self.kappa.to_bytes().as_ref());
        bytes.extend_from_slice(self.phi.to_bytes().as_ref());
        bytes.extend_from_slice(self.varphi.to_bytes().as_ref());
        bytes.extend_from_slice(self.rr.to_bytes().as_ref());
        bytes.extend_from_slice(self.rr_prime.to_bytes().as_ref());
        bytes.extend_from_slice(self.ss.to_bytes().as_ref());
        bytes.extend_from_slice(self.tt.to_bytes().as_ref());
        bytes.extend_from_slice(self.varsig_prime1.to_bytes().as_ref());
        bytes.extend_from_slice(self.theta_prime1.to_bytes().as_ref());
        bytes.extend_from_slice(self.pg_eq1.to_compressed().as_ref());
        bytes.extend_from_slice(self.pg_eq2.to_compressed().as_ref());
        bytes.extend_from_slice(self.pg_eq3.to_compressed().as_ref());
        bytes.extend_from_slice(self.pg_eq4.to_compressed().as_ref());
        bytes.extend_from_slice(self.psi_g1.to_bytes().as_ref());
        bytes.extend_from_slice(self.psi_g2.to_bytes().as_ref());
        bytes.extend_from_slice(self.pg_psi0_delta.to_compressed().as_ref());
        bytes.extend_from_slice(self.pg_psi0_gen2.to_compressed().as_ref());
        bytes.extend_from_slice(self.pg_psi0_yy.to_compressed().as_ref());
        bytes.extend_from_slice(self.pg_psi0_ww1.to_compressed().as_ref());
        bytes.extend_from_slice(self.pg_psi0_ww2.to_compressed().as_ref());
        bytes.extend_from_slice(self.pg_rr_psi1.to_compressed().as_ref());
        bytes.extend_from_slice(self.pg_psi0_tt.to_compressed().as_ref());
        bytes.extend_from_slice(self.pg_psi0_psi1.to_compressed().as_ref());

        bytes
    }
}

pub struct SpendWitness {
    pub sk_u: SecretKeyUser,
    pub v: Scalar,
    pub r: Scalar,
    pub r1: Scalar,
    pub r2: Scalar,
    pub r_varsig1: Scalar,
    pub r_theta1: Scalar,
    pub r_varsig2: Scalar,
    pub r_theta2: Scalar,
    pub r_rr: Scalar,
    pub r_ss: Scalar,
    pub r_tt: Scalar,
    pub rho1: Scalar,
    pub rho2: Scalar,
    pub rho3: Scalar,
}

#[derive(Debug, Clone)]
pub struct SpendProof {
    challenge: Scalar,
    response_r: Scalar,
    response_r_sk_u: Scalar,
    response_r_v: Scalar,
    response_r_r: Scalar,
    response_r_r1: Scalar,
    response_r_r2: Scalar,
    response_r_varsig1: Scalar,
    response_r_theta1: Scalar,
    response_r_varsig2: Scalar,
    response_r_theta2: Scalar,
    response_r_rr: Scalar,
    response_r_ss: Scalar,
    response_r_tt: Scalar,
    response_r_rho1: Scalar,
    response_r_rho2: Scalar,
    response_r_rho3: Scalar,
}

impl SpendProof {
    pub fn construct(
        params: &Parameters,
        instance: &SpendInstance,
        witness: &SpendWitness,
        verification_key: &VerificationKeyAuth,
        vv: u64) -> Self {
        let grp = params.get_grp();
        let params_u = params.get_params_u();
        let params_a = params.get_params_a();

        // generate random values to replace each witness
        let r_attributes = grp.n_random_scalars(2);
        let r_sk_u = r_attributes[0];
        let r_v = r_attributes[1];
        let r_r = grp.random_scalar();
        let r_r1 = grp.random_scalar();
        let r_r2 = grp.random_scalar();
        let r_r_varsig1 = grp.random_scalar();
        let r_r_theta1 = grp.random_scalar();
        let r_r_varsig2 = grp.random_scalar();
        let r_r_theta2 = grp.random_scalar();
        let r_r_rr = grp.random_scalar();
        let r_r_ss = grp.random_scalar();
        let r_r_tt = grp.random_scalar();
        let r_rho1 = grp.random_scalar();
        let r_rho2 = grp.random_scalar();
        let r_rho3 = grp.random_scalar();

        let g1 = grp.gen1();

        // compute zkp commitment for each instance
        let zkcm_kappa = grp.gen2() * r_r
            + verification_key.alpha
            + r_attributes
            .iter()
            .zip(verification_key.beta_g2.iter())
            .map(|(attr, beta_i)| beta_i * attr)
            .sum::<G2Projective>();

        let zkcm_phi0 = g1 * r_r1;
        let zkcm_phi1 = instance.varsig_prime1 * r_v + instance.psi_g1 * r_rho1 + params_u.get_ith_eta(vv as usize) * r_r1;
        let zkcm_varphi0 = g1 * r_r2;
        let zkcm_varphi1 = (g1 * instance.rr) * r_sk_u + instance.theta_prime1 * r_v + instance.psi_g1 * r_rho2 + params_u.get_ith_eta(vv as usize) * r_r2;
        let zkcm_pg_eq1 = instance.pg_psi0_delta * r_r_varsig1 + instance.pg_psi0_gen2 * r_r_varsig2.neg();
        let zkcm_pg_eq2 = instance.pg_psi0_delta * r_r_theta1 + instance.pg_psi0_gen2 * r_r_theta2.neg();
        let zkcm_pg_eq3 = instance.pg_psi0_yy * r_r_rr + instance.pg_psi0_gen2 * r_r_ss + instance.pg_psi0_ww1 * r_r_varsig2 + instance.pg_psi0_ww2 * r_r_theta2;
        let zkcm_pg_eq4 = instance.pg_rr_psi1 * r_r_tt + instance.pg_psi0_tt * r_r_rr + instance.pg_psi0_psi1 * r_rho3.neg();


        let challenge = compute_challenge::<ChallengeDigest, _, _>(
            std::iter::once(g1.to_bytes().as_ref())
                .chain(std::iter::once(instance.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_kappa.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_phi0.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_phi1.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_varphi0.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_varphi1.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_pg_eq1.to_compressed().as_ref()))
                .chain(std::iter::once(zkcm_pg_eq2.to_compressed().as_ref()))
                // .chain(std::iter::once(zkcm_pg_eq3.to_compressed().as_ref()))
                .chain(std::iter::once(zkcm_pg_eq4.to_compressed().as_ref()))
        );

        // compute response for each witness
        let response_r = produce_response(&r_r, &challenge, &witness.r);
        let response_r_sk_u = produce_response(&r_sk_u, &challenge, &witness.sk_u.sk);
        let response_r_v = produce_response(&r_v, &challenge, &witness.v);
        let response_r_r = produce_response(&r_r, &challenge, &witness.r);
        let response_r_r1 = produce_response(&r_r1, &challenge, &witness.r1);
        let response_r_r2 = produce_response(&r_r2, &challenge, &witness.r2);
        let response_r_varsig1 = produce_response(&r_r_varsig1, &challenge, &witness.r_varsig1);
        let response_r_theta1 = produce_response(&r_r_theta1, &challenge, &witness.r_theta1);
        let response_r_varsig2 = produce_response(&r_r_varsig2, &challenge, &witness.r_varsig2);
        let response_r_theta2 = produce_response(&r_r_theta2, &challenge, &witness.r_theta2);
        let response_r_rr = produce_response(&r_r_rr, &challenge, &witness.r_rr);
        let response_r_ss = produce_response(&r_r_ss, &challenge, &witness.r_ss);
        let response_r_tt = produce_response(&r_r_tt, &challenge, &witness.r_tt);
        let response_r_rho1 = produce_response(&r_rho1, &challenge, &witness.rho1);
        let response_r_rho2 = produce_response(&r_rho2, &challenge, &witness.rho2);
        let response_r_rho3 = produce_response(&r_rho3, &challenge, &witness.rho3);


        SpendProof {
            challenge,
            response_r,
            response_r_sk_u,
            response_r_v,
            response_r_r,
            response_r_r1,
            response_r_r2,
            response_r_varsig1,
            response_r_theta1,
            response_r_varsig2,
            response_r_theta2,
            response_r_rr,
            response_r_ss,
            response_r_tt,
            response_r_rho1,
            response_r_rho2,
            response_r_rho3,
        }
    }

    pub fn verify(
        &self,
        params: &Parameters,
        instance: &SpendInstance,
        verification_key: &VerificationKeyAuth,
        vv: u64,
    ) -> bool {
        let grp = params.get_grp();
        let params_u = params.get_params_u();
        let params_a = params.get_params_a();
        let g1 = grp.gen1();

        // re-compute each zkp commitment
        let zkcm_kappa = instance.kappa * self.challenge
            + grp.gen2() * self.response_r
            + verification_key.alpha * (Scalar::one() - self.challenge)
            + [self.response_r_sk_u, self.response_r_v]
            .iter()
            .zip(verification_key.beta_g2.iter())
            .map(|(attr, beta_i)| beta_i * attr)
            .sum::<G2Projective>();

        let zkcm_phi0 = g1 * self.response_r_r1 + instance.phi.0 * self.challenge;
        let zkcm_phi1 = instance.varsig_prime1 * self.response_r_v
            + instance.psi_g1 * self.response_r_rho1
            + params_u.get_ith_eta(vv as usize) * self.response_r_r1
            + instance.phi.1 * self.challenge;
        let zkcm_varphi0 = g1 * self.response_r_r2 + instance.varphi.0 * self.challenge;
        let zkcm_varphi1 = (g1 * instance.rr) * self.response_r_sk_u
            + instance.theta_prime1 * self.response_r_v
            + instance.psi_g1 * self.response_r_rho2
            + params_u.get_ith_eta(vv as usize) * self.response_r_r2
            + instance.varphi.1 * self.challenge;
        let zkcm_pg_eq1 = instance.pg_psi0_delta * self.response_r_varsig1
            + instance.pg_psi0_gen2 * self.response_r_varsig2.neg()
            + instance.pg_eq1 * self.challenge;
        let zkcm_pg_eq2 = instance.pg_psi0_delta * self.response_r_theta1
            + instance.pg_psi0_gen2 * self.response_r_theta2.neg()
            + instance.pg_eq2 * self.challenge;


        let zkcm_pg_eq3 = instance.pg_psi0_yy * self.response_r_rr
            + instance.pg_psi0_gen2 * self.response_r_ss
            + instance.pg_psi0_ww1 * self.response_r_varsig2
            + instance.pg_psi0_ww2 * self.response_r_theta2
            + instance.pg_eq3 * self.challenge;

        let zkcm_pg_eq4 = instance.pg_rr_psi1 * self.response_r_tt
            + instance.pg_psi0_tt * self.response_r_rr
            + instance.pg_psi0_psi1 * self.response_r_rho3.neg()
            + instance.pg_eq4 * self.challenge;

        // re-compute the challenge
        let challenge = compute_challenge::<ChallengeDigest, _, _>(
            std::iter::once(g1.to_bytes().as_ref())
                .chain(std::iter::once(instance.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_kappa.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_phi0.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_phi1.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_varphi0.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_varphi1.to_bytes().as_ref()))
                .chain(std::iter::once(zkcm_pg_eq1.to_compressed().as_ref()))
                .chain(std::iter::once(zkcm_pg_eq2.to_compressed().as_ref()))
                // .chain(std::iter::once(zkcm_pg_eq3.to_compressed().as_ref()))
                .chain(std::iter::once(zkcm_pg_eq4.to_compressed().as_ref()))
        );

        challenge == self.challenge
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Neg;

    use bls12_381::{G2Projective, Gt, pairing};
    use group::Curve;
    use rand::thread_rng;

    use crate::proofs::proof_spend::{SpendInstance, SpendProof, SpendWitness};
    use crate::scheme::{PayInfo, Phi, VarPhi};
    use crate::scheme::aggregation::aggregate_verification_keys;
    use crate::scheme::keygen::{PublicKeyUser, SecretKeyUser, ttp_keygen_authorities, VerificationKeyAuth};
    use crate::scheme::setup::{GroupParameters, Parameters};
    use crate::utils::hash_to_scalar;

    #[test]
    fn spend_proof_construct_and_verify() {
        let rng = thread_rng();
        let grp = GroupParameters::new().unwrap();
        let params = Parameters::new(grp.clone());
        let params_u = params.get_params_u();
        let params_a = params.get_params_a();

        let sk = grp.random_scalar();
        let pk_user = PublicKeyUser {
            pk: grp.gen1() * sk,
        };
        let v = grp.random_scalar();
        let attributes = vec![sk, v];
        let l: usize = 10;
        let vv: u64 = 20;

        let authorities_keypairs = ttp_keygen_authorities(&params, 2, 3).unwrap();
        let verification_keys_auth: Vec<VerificationKeyAuth> = authorities_keypairs
            .iter()
            .map(|keypair| keypair.verification_key())
            .collect();

        let verification_key =
            aggregate_verification_keys(&verification_keys_auth, Some(&[1, 2, 3])).unwrap();

        let r = grp.random_scalar();
        let kappa = grp.gen2() * r
            + verification_key.alpha
            + attributes
            .iter()
            .zip(verification_key.beta_g2.iter())
            .map(|(priv_attr, beta_i)| beta_i * priv_attr)
            .sum::<G2Projective>();

        let r1 = grp.random_scalar();
        let r2 = grp.random_scalar();
        let phi = Phi(grp.gen1() * r1, params_u.get_ith_sigma(l as usize) * v + params_u.get_ith_eta(vv as usize) * r1);

        let pay_info = PayInfo { info: [78u8; 32] };
        let rr = hash_to_scalar(pay_info.info);
        let varphi = VarPhi(grp.gen1() * r2, (grp.gen1() * rr) * sk + params_u.get_ith_theta(l as usize) * v + params_u.get_ith_eta(vv as usize) * r2);

        // random value used to compute blinded bases
        let r_varsig1 = grp.random_scalar();
        let r_theta1 = grp.random_scalar();
        let r_varsig2 = grp.random_scalar();
        let r_theta2 = grp.random_scalar();
        let r_rr = grp.random_scalar();
        let r_ss = grp.random_scalar();
        let r_tt = grp.random_scalar();


        // compute blinded bases
        let psi_g1 = params_u.get_psi_g1();
        let psi_g2 = params_u.get_psi_g2();
        let varsig_prime1 = params_u.get_ith_sigma(l as usize) + (psi_g1 * r_varsig1);
        let theta_prime1 = params_u.get_ith_theta(l as usize) + (psi_g1 * r_theta1);
        let varsig_prime2 = params_u.get_ith_sigma(l as usize + vv as usize - 1) + (psi_g1 * r_varsig2);
        let theta_prime2 = params_u.get_ith_theta(l as usize + vv as usize - 1) + (psi_g1 * r_theta2);
        let rr_prime = params_u.get_ith_sps_sign(l as usize + vv as usize - 1).rr + (psi_g1 * r_rr);
        let ss_prime = params_u.get_ith_sps_sign(l as usize + vv as usize - 1).ss + (psi_g1 * r_ss);
        let tt_prime = params_u.get_ith_sps_sign(l as usize + vv as usize - 1).tt + (psi_g2 * r_tt);


        let rho1 = v.neg() * r_varsig1;
        let rho2 = v.neg() * r_theta1;
        let rho3 = r_rr * r_tt;


        let pg_varsigpr1_delta = pairing(&varsig_prime1.to_affine(), &params_a.get_ith_delta((vv - 1) as usize).to_affine());
        let pg_psi0_delta = pairing(&psi_g1.to_affine(), &params_a.get_ith_delta((vv - 1) as usize).to_affine());
        let pg_varsigpr2_gen2 = pairing(&varsig_prime2.to_affine(), grp.gen2());
        let pg_psi0_gen2 = pairing(&psi_g1.to_affine(), grp.gen2());
        let pg_thetapr1_delta = pairing(&theta_prime1.to_affine(), &params_a.get_ith_delta((vv - 1) as usize).to_affine());
        let pg_thetapr2_gen2 = pairing(&theta_prime2.to_affine(), grp.gen2());
        let yy = params_u.get_sps_pk().get_yy();
        let pg_rrprime_yy = pairing(&rr_prime.to_affine(), &yy.to_affine());
        let pg_psi0_yy = pairing(&psi_g1.to_affine(), &yy.to_affine());
        let pg_ssprime_gen2 = pairing(&ss_prime.to_affine(), grp.gen2());
        let ww1 = params_u.get_sps_pk().get_ith_ww(0);
        let ww2 = params_u.get_sps_pk().get_ith_ww(1);
        let pg_varsigpr2_ww1 = pairing(&varsig_prime2.to_affine(), &ww1.to_affine());
        let pg_psi0_ww1 = pairing(&psi_g1.to_affine(), &ww1.to_affine());
        let pg_thetapr2_ww2 = pairing(&theta_prime1.to_affine(), &ww2.to_affine());
        let pg_psi0_ww2 = pairing(&psi_g1.to_affine(), &ww2.to_affine());
        let pg_gen1_zz = pairing(grp.gen1(), &params_u.get_sps_pk().get_zz().to_affine());
        let pg_rr_tt = pairing(&rr_prime.to_affine(), &tt_prime.to_affine());
        let pg_rr_psi1 = pairing(&rr_prime.to_affine(), &psi_g2.to_affine());
        let pg_psi0_tt = pairing(&psi_g1.to_affine(), &tt_prime.to_affine());
        let pg_psi0_psi1 = pairing(&psi_g1.to_affine(), &psi_g2.to_affine());
        let pg_gen1_gen2 = pairing(grp.gen1(), grp.gen2());

        let pg_eq1 = pg_varsigpr1_delta - pg_varsigpr2_gen2;
        let pg_eq2 = pg_thetapr1_delta - pg_thetapr2_gen2;
        let pg_eq3 = pg_rrprime_yy + pg_ssprime_gen2 + pg_varsigpr2_ww1 + pg_thetapr2_ww2 - pg_gen1_zz;
        let pg_eq4 = pg_rr_tt - pg_gen1_gen2;


        let instance = SpendInstance {
            kappa,
            phi,
            varphi,
            rr,
            rr_prime,
            ss: ss_prime,
            tt: tt_prime,
            varsig_prime1,
            theta_prime1,
            pg_eq1,
            pg_eq2,
            pg_eq3,
            pg_eq4,
            psi_g1: *psi_g1,
            psi_g2: *psi_g2,
            pg_psi0_delta,
            pg_psi0_gen2,
            pg_psi0_yy,
            pg_psi0_ww1,
            pg_psi0_ww2,
            pg_rr_psi1,
            pg_psi0_tt,
            pg_psi0_psi1,
        };

        let witness = SpendWitness {
            sk_u: SecretKeyUser { sk },
            v,
            r,
            r1,
            r2,
            r_varsig1,
            r_theta1,
            r_varsig2,
            r_theta2,
            r_rr,
            r_ss,
            r_tt,
            rho1,
            rho2,
            rho3,
        };

        // compute the zk proof
        let zk_proof = SpendProof::construct(&params, &instance, &witness, &verification_key, vv);
        assert!(zk_proof.verify(&params, &instance, &verification_key, vv))
    }
}


