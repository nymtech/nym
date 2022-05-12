use std::cell::Cell;

use bls12_381::{G1Projective, G2Prepared, G2Projective, pairing, Scalar};
use group::Curve;

use crate::Attribute;
use crate::constants::L;
use crate::error::{DivisibleEcashError, Result};
use crate::proofs::proof_spend::{SpendInstance, SpendProof, SpendWitness};
use crate::scheme::keygen::{SecretKeyUser, VerificationKeyAuth};
use crate::scheme::setup::{GroupParameters, Parameters};
use crate::utils::{check_bilinear_pairing, hash_to_scalar, Signature, SignerIndex};

pub mod aggregation;
pub mod identify;
pub mod keygen;
pub mod setup;
pub mod structure_preserving_signature;
pub mod withdrawal;

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

#[derive(Debug, Clone, Copy)]
pub struct Phi(pub(crate) G1Projective, pub(crate) G1Projective);

#[derive(Debug, Clone, Copy)]
pub struct VarPhi(pub(crate) G1Projective, pub(crate) G1Projective);


pub struct PayInfo {
    pub info: [u8; 32],
}

#[derive(Debug, Clone)]
pub struct Payment {
    kappa: G2Projective,
    sig: Signature,
    phi: Phi,
    varphi: VarPhi,
    rr: Scalar,
    zkproof: SpendProof,
    vv: u64,
}

pub struct PartialWallet {
    sig: Signature,
    v: Scalar,
    idx: Option<SignerIndex>,
}

pub struct Wallet {
    sig: Signature,
    v: Scalar,
    l: Cell<u64>,
}

impl Wallet {
    pub fn signature(&self) -> &Signature {
        &self.sig
    }

    pub fn v(&self) -> Scalar {
        self.v
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

    pub(crate) fn spend(
        &self,
        params: &Parameters,
        verification_key: &VerificationKeyAuth,
        sk_user: &SecretKeyUser,
        pay_info: &PayInfo,
        vv: u64,
    ) -> Result<(Payment, &Self)> {
        if self.l() + vv >= L {
            return Err(DivisibleEcashError::Spend(
                "The counter l is higher than max L".to_string(),
            ));
        }

        let grp = params.get_grp();
        let params_u = params.get_params_u();
        let params_a = params.get_params_a();
        // randomize signature in the wallet
        let (signature_prime, sign_blinding_factor) = self.signature().randomise(grp);
        // construct kappa i.e., blinded attributes for show
        let attributes = vec![sk_user.sk, self.v()];
        // compute kappa
        let kappa = compute_kappa(
            &grp,
            &verification_key,
            &attributes,
            sign_blinding_factor,
        );

        let r1 = grp.random_scalar();
        let r2 = grp.random_scalar();
        let phi = Phi(grp.gen1() * r1, params_u.get_ith_sigma(self.l.get() as usize) * self.v + params_u.get_ith_eta(vv as usize) * r1);

        // compute hash of the payment info
        let rr = hash_to_scalar(pay_info.info);
        let varphi = VarPhi(grp.gen1() * r2, (grp.gen1() * rr) * sk_user.sk + params_u.get_ith_theta(self.l.get() as usize) * self.v + params_u.get_ith_eta(vv as usize) * r2);

        // random value used to compute blinded bases
        let r_varsig1 = grp.random_scalar();
        let r_theta1 = grp.random_scalar();
        let r_varsig2 = grp.random_scalar();
        let r_theta2 = grp.random_scalar();
        let r_rr = grp.random_scalar();
        let r_ss = grp.random_scalar();
        let r_tt = grp.random_scalar();

        // compute blinded bases
        let psi0 = params_u.get_psi0();
        let psi1 = params_u.get_psi1();
        let varsig_prime1 = params_u.get_ith_sigma(self.l() as usize) + (psi0 * r_varsig1);
        let theta_prime1 = params_u.get_ith_theta(self.l() as usize) + (psi0 * r_theta1);
        let varsig_prime2 = params_u.get_ith_sigma(self.l() as usize + vv as usize - 1) + (psi0 * r_varsig2);
        let theta_prime2 = params_u.get_ith_sigma(self.l() as usize + vv as usize - 1) + (psi0 * r_theta2);
        let rr_prime = params_u.get_ith_sps_sign(self.l() as usize + vv as usize - 1).rr + (psi0 * r_rr);
        let ss_prime = params_u.get_ith_sps_sign(self.l() as usize + vv as usize - 1).ss + (psi0 * r_ss);
        let tt_prime = params_u.get_ith_sps_sign(self.l() as usize + vv as usize - 1).tt + (psi1 * r_tt);

        let rho1 = self.v.neg() * r_varsig1;
        let rho2 = self.v.neg() * r_theta1;
        let rho3 = r_rr * r_tt;

        let pg_varsigpr1_delta = pairing(&varsig_prime1.to_affine(), &params_a.get_ith_delta((vv - 1) as usize).to_affine());
        let pg_psi0_delta = pairing(&psi0.to_affine(), &params_a.get_ith_delta((vv - 1) as usize).to_affine());
        let pg_varsigpr2_gen2 = pairing(&varsig_prime2.to_affine(), grp.gen2());
        let pg_psi0_gen2 = pairing(&psi0.to_affine(), grp.gen2());
        let pg_thetapr1_delta = pairing(&theta_prime1.to_affine(), &params_a.get_ith_delta((vv - 1) as usize).to_affine());
        let pg_thetapr2_gen2 = pairing(&theta_prime1.to_affine(), grp.gen2());
        let yy = params_u.get_sps_pk().get_yy();
        let pg_rr_yy = pairing(&rr_prime.to_affine(), &yy.to_affine());
        let pg_psi0_yy = pairing(&psi0.to_affine(), &yy.to_affine());
        let pg_ssprime_gen2 = pairing(&ss_prime.to_affine(), grp.gen2());
        let ww1 = params_u.get_sps_pk().get_ith_ww(0);
        let ww2 = params_u.get_sps_pk().get_ith_ww(1);
        let pg_varsigpr2_ww1 = pairing(&varsig_prime2.to_affine(), &ww1.to_affine());
        let pg_psi0_ww1 = pairing(&psi0.to_affine(), &ww1.to_affine());
        let pg_thetapr2_ww2 = pairing(&theta_prime1.to_affine(), &ww2.to_affine());
        let pg_psi0_ww2 = pairing(&psi0.to_affine(), &ww2.to_affine());
        let pg_gen1_zz = pairing(grp.gen1(), &params_u.get_sps_pk().get_zz().to_affine());
        let pg_rr_tt = pairing(&rr_prime.to_affine(), &tt_prime.to_affine());
        let pg_rr_psi1 = pairing(&rr_prime.to_affine(), &psi1.to_affine());
        let pg_psi0_tt = pairing(&psi0.to_affine(), &tt_prime.to_affine());
        let pg_psi0_psi1 = pairing(&psi0.to_affine(), &psi1.to_affine());
        let pg_gen1_gen2 = pairing(grp.gen1(), grp.gen2());

        let instance = SpendInstance {
            kappa,
            phi,
            varphi,
            rr: rr_prime,
            ss: ss_prime,
            tt: tt_prime,
            pg_varsigpr1_delta,
            pg_psi0_delta,
            pg_varsigpr2_gen2,
            pg_psi0_gen2,
            pg_thetapr1_delta,
            pg_thetapr2_gen2,
            pg_rr_yy,
            pg_psi0_yy,
            pg_ssprime_gen2,
            pg_varsigpr2_ww1,
            pg_psi0_ww1,
            pg_thetapr2_ww2,
            pg_psi0_ww2,
            pg_gen1_zz,
            pg_rr_tt,
            pg_rr_psi1,
            pg_psi0_tt,
            pg_psi0_psi1,
            pg_gen1_gen2,
        };

        let witness = SpendWitness {
            sk_u: sk_user.clone(),
            v: self.v,
            r: sign_blinding_factor,
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
        let zkproof = SpendProof::construct(params, &instance, &witness);

        // output pay and updated wallet
        let pay = Payment {
            kappa,
            sig: signature_prime,
            phi,
            varphi,
            rr,
            zkproof,
            vv,
        };

        self.l.set(self.l.get() + vv);
        Ok((pay, self))
    }
}

impl Payment {
    pub fn spend_verify(
        &self,
        params: &Parameters,
        verification_key: &VerificationKeyAuth,
        pay_info: &PayInfo) -> Result<bool> {
        if bool::from(self.sig.0.is_identity()) {
            return Err(DivisibleEcashError::Spend(
                "The element h of the signature equals the identity".to_string(),
            ));
        }
        let grp = params.get_grp();

        if !check_bilinear_pairing(
            &self.sig.0.to_affine(),
            &G2Prepared::from(self.kappa.to_affine()),
            &self.sig.1.to_affine(),
            grp.prepared_miller_g2(),
        ) {
            return Err(DivisibleEcashError::Spend(
                "The bilinear check for kappa failed".to_string(),
            ));
        }

        if bool::from(self.sig.0.is_identity()) {
            return Err(DivisibleEcashError::Spend(
                "The element h of the signature on l equals the identity".to_string(),
            ));
        }

        // verify integrity of R
        if !(self.rr == hash_to_scalar(pay_info.info)) {
            return Err(DivisibleEcashError::Spend(
                "Integrity of R does not hold".to_string(),
            ));
        }

        //TODO: verify whether payinfo contains merchent's identifier

        // TODO: Add zk proof verification

        Ok(true)
    }
}