use std::cell::Cell;

use bls12_381::{G1Projective, G2Prepared, G2Projective, Scalar};
use group::Curve;

use crate::Attribute;
use crate::constants::L;
use crate::error::{DivisibleEcashError, Result};
use crate::proofs::proof_spend::SpendProof;
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

#[derive(Debug, Clone)]
pub struct Phi(pub(crate) G1Projective, pub(crate) G1Projective);

#[derive(Debug, Clone)]
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
        // compute the zk proof
        let zkproof = SpendProof {};

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