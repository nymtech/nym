use std::cell::Cell;
use std::convert::TryFrom;
use std::convert::TryInto;

use bls12_381::{G1Projective, G2Prepared, G2Projective, Scalar};
use group::{Curve, Group};

use crate::Attribute;
use crate::error::{CompactEcashError, Result};
use crate::proofs::proof_spend::{SpendInstance, SpendProof, SpendWitness};
use crate::scheme::keygen::{SecretKeyUser, VerificationKeyAuth};
use crate::scheme::setup::{GroupParameters, Parameters};
use crate::utils::{
    check_bilinear_pairing, hash_to_scalar, Signature, SignerIndex, try_deserialize_g1_projective,
};

pub mod aggregation;
pub mod identify;
pub mod keygen;
pub mod setup;
pub mod withdrawal;

pub struct PartialWallet {
    sig: Signature,
    v: Scalar,
    idx: Option<SignerIndex>,
}

impl PartialWallet {
    pub fn signature(&self) -> &Signature {
        &self.sig
    }
    pub fn v(&self) -> Scalar {
        self.v
    }
    pub fn index(&self) -> Option<SignerIndex> {
        self.idx
    }
}

pub struct Wallet {
    sig: Signature,
    v: Scalar,
    t: Scalar,
    l: Cell<u64>,
}

impl Wallet {
    pub fn signature(&self) -> &Signature {
        &self.sig
    }
    pub fn v(&self) -> Scalar {
        self.v
    }
    pub fn t(&self) -> Scalar {
        self.t
    }
    pub fn l(&self) -> u64 {
        self.l.get()
    }

    fn up(&self) {
        self.l.set(self.l.get() + 1);
    }

    fn down(&self) {
        self.l.set(self.l.get() - 1);
    }

    pub fn spend(
        &self,
        params: &Parameters,
        verification_key: &VerificationKeyAuth,
        sk_user: &SecretKeyUser,
        pay_info: &PayInfo,
        bench_flag: bool,
        spend_vv: u64,
    ) -> Result<(Payment, &Self)> {
        if self.l() + spend_vv > params.L() {
            return Err(CompactEcashError::Spend(
                "The counter l is higher than max L".to_string(),
            ));
        }

        let grparams = params.grp();
        // randomize signature in the wallet
        let (signature_prime, sign_blinding_factor) = self.signature().randomise(grparams);
        // construct kappa i.e., blinded attributes for show
        let attributes = vec![sk_user.sk, self.v(), self.t()];
        // compute kappa
        let kappa = compute_kappa(
            &grparams,
            &verification_key,
            &attributes,
            sign_blinding_factor,
        );

        // pick random openings o_c, o_d
        let o_c = grparams.random_scalar();
        let o_d = grparams.random_scalar();

        // compute commitments C, D
        let cc = grparams.gen1() * o_c + grparams.gamma1() * self.v();
        let dd = grparams.gen1() * o_d + grparams.gamma1() * self.t();


        let mut aa: Vec<G1Projective> = Default::default();
        let mut ss: Vec<G1Projective> = Default::default();
        let mut tt: Vec<G1Projective> = Default::default();
        let mut rr: Vec<Scalar> = Default::default();
        let mut o_a: Vec<Scalar> = Default::default();
        let mut o_mu: Vec<Scalar> = Default::default();
        let mut mu: Vec<Scalar> = Default::default();
        let mut o_lambda: Vec<Scalar> = Default::default();
        let mut lambda: Vec<Scalar> = Default::default();
        let mut r_k_vec: Vec<Scalar> = Default::default();
        let mut kappa_k_vec: Vec<G2Projective> = Default::default();
        let mut sign_lk_prime_vec: Vec<Signature> = Default::default();
        let mut lk: Vec<Scalar> = Default::default();

        for k in 0..spend_vv {
            lk.push(Scalar::from(self.l() + k));

            // compute hashes R_k of the payment info
            let rr_k = hash_to_scalar(pay_info.info);
            rr.push(rr_k);

            let o_a_k = grparams.random_scalar();
            o_a.push(o_a_k);
            let aa_k = grparams.gen1() * o_a_k + grparams.gamma1() * Scalar::from(self.l() + k);
            aa.push(aa_k);

            // evaluate the pseudorandom functions
            let ss_k = pseudorandom_fgv(&grparams, self.v(), self.l() + k);
            ss.push(ss_k);
            let tt_k =
                grparams.gen1() * sk_user.sk + pseudorandom_fgt(&grparams, self.t(), self.l() + k) * rr_k;
            tt.push(tt_k);

            // compute values mu, o_mu, lambda, o_lambda
            let mu_k: Scalar = (self.v() + Scalar::from(self.l() + k) + Scalar::from(1))
                .invert()
                .unwrap();
            mu.push(mu_k);

            let o_mu_k = ((o_a_k + o_c) * mu_k).neg();
            o_mu.push(o_mu_k);

            let lambda_k = (self.t() + Scalar::from(self.l() + k) + Scalar::from(1))
                .invert()
                .unwrap();
            lambda.push(lambda_k);

            let o_lambda_k = ((o_a_k + o_d) * lambda_k).neg();
            o_lambda.push(o_lambda_k);

            // parse the signature associated with value l+k
            let sign_lk = params.get_sign_by_idx(self.l() + k)?;
            // randomise the signature associated with value l+k
            let (sign_lk_prime, r_k) = sign_lk.randomise(grparams);
            sign_lk_prime_vec.push(sign_lk_prime);
            r_k_vec.push(r_k);
            // compute kappa_k
            let kappa_k = grparams.gen2() * r_k
                + params.pkRP().alpha
                + params.pkRP().beta * Scalar::from(self.l() + k);
            kappa_k_vec.push(kappa_k);
        }


        // construct the zkp proof
        let spend_instance = SpendInstance {
            kappa,
            cc,
            dd,
            aa: aa.clone(),
            ss: ss.clone(),
            tt: tt.clone(),
            kappa_k: kappa_k_vec.clone(),
        };
        let spend_witness = SpendWitness {
            attributes,
            r: sign_blinding_factor,
            o_c,
            o_d,
            lk,
            o_a,
            mu,
            lambda,
            o_mu,
            o_lambda,
            r_k: r_k_vec,
        };
        let zk_proof = SpendProof::construct(
            &params,
            &spend_instance,
            &spend_witness,
            &verification_key,
            &rr,
        );

        // output pay and updated wallet
        let pay = Payment {
            kappa,
            sig: signature_prime,
            ss: ss.clone(),
            tt: tt.clone(),
            aa: aa.clone(),
            rr: rr.clone(),
            kappa_k: kappa_k_vec.clone(),
            sig_lk: sign_lk_prime_vec,
            cc,
            dd,
            zk_proof,
            vv: spend_vv,
        };

        // The number of samples collected by the benchmark process is way higher than the
        // MAX_WALLET_VALUE we ever consider. Thus, we would execute the spending too many times
        // and the initial condition at the top of this function will crush. Thus, we need a
        // benchmark flag to signal that we don't want to increase the spending couter but only
        // care about the function performance.
        if !bench_flag {
            self.up();
        }

        Ok((pay, self))
    }
}

pub fn pseudorandom_fgv(params: &GroupParameters, v: Scalar, l: u64) -> G1Projective {
    let pow = (v + Scalar::from(l) + Scalar::from(1)).invert().unwrap();
    params.gen1() * pow
}

pub fn pseudorandom_fgt(params: &GroupParameters, t: Scalar, l: u64) -> G1Projective {
    let pow = (t + Scalar::from(l) + Scalar::from(1)).invert().unwrap();
    params.gen1() * pow
}

pub fn compute_kappa(
    params: &GroupParameters,
    verification_key: &VerificationKeyAuth,
    attributes: &[Attribute],
    blinding_factor: Scalar,
) -> G2Projective {
    params.gen2() * blinding_factor
        + verification_key.alpha
        + attributes
        .iter()
        .zip(verification_key.beta_g2.iter())
        .map(|(priv_attr, beta_i)| beta_i * priv_attr)
        .sum::<G2Projective>()
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct PayInfo {
    pub info: [u8; 32],
}

#[derive(Debug, Clone)]
pub struct Payment {
    pub kappa: G2Projective,
    pub sig: Signature,
    pub ss: Vec<G1Projective>,
    pub tt: Vec<G1Projective>,
    pub aa: Vec<G1Projective>,
    pub rr: Vec<Scalar>,
    pub kappa_k: Vec<G2Projective>,
    pub sig_lk: Vec<Signature>,
    pub cc: G1Projective,
    pub dd: G1Projective,
    pub zk_proof: SpendProof,
    pub vv: u64,
}

impl Payment {
    pub fn spend_verify(
        &self,
        params: &Parameters,
        verification_key: &VerificationKeyAuth,
        pay_info: &PayInfo,
        spend_vv: u64,
    ) -> Result<bool> {
        if bool::from(self.sig.0.is_identity()) {
            return Err(CompactEcashError::Spend(
                "The element h of the signature equals the identity".to_string(),
            ));
        }

        if !check_bilinear_pairing(
            &self.sig.0.to_affine(),
            &G2Prepared::from(self.kappa.to_affine()),
            &self.sig.1.to_affine(),
            params.grp().prepared_miller_g2(),
        ) {
            return Err(CompactEcashError::Spend(
                "The bilinear check for kappa failed".to_string(),
            ));
        }

        for k in 0..spend_vv {
            if bool::from(self.sig_lk[k as usize].0.is_identity()) {
                return Err(CompactEcashError::Spend(
                    "The element h of the signature on l equals the identity".to_string(),
                ));
            }

            if !check_bilinear_pairing(
                &self.sig_lk[k as usize].0.to_affine(),
                &G2Prepared::from(self.kappa_k[k as usize].to_affine()),
                &self.sig_lk[k as usize].1.to_affine(),
                params.grp().prepared_miller_g2(),
            ) {
                return Err(CompactEcashError::Spend(
                    "The bilinear check for kappa_l failed".to_string(),
                ));
            }
            // verify integrity of R_k
            if !(self.rr[k as usize] == hash_to_scalar(pay_info.info)) {
                return Err(CompactEcashError::Spend(
                    "Integrity of R_k does not hold".to_string(),
                ));
            }
        }

        //TODO: verify whether payinfo contains merchent's identifier

        // verify the zk proof
        let instance = SpendInstance {
            kappa: self.kappa,
            aa: self.aa.clone(),
            cc: self.cc,
            dd: self.dd,
            ss: self.ss.clone(),
            tt: self.tt.clone(),
            kappa_k: self.kappa_k.clone(),
        };

        if !self
            .zk_proof
            .verify(&params, &instance, &verification_key, &self.rr)
        {
            return Err(CompactEcashError::Spend(
                "ZkProof verification failed".to_string(),
            ));
        }

        Ok(true)
    }
}
