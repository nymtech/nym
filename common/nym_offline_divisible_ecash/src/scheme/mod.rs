use std::cell::Cell;

use bls12_381::{G2Projective, Scalar};

use crate::Attribute;
use crate::constants::L;
use crate::error::{DivisibleEcashError, Result};
use crate::scheme::keygen::{SecretKeyUser, VerificationKeyAuth};
use crate::scheme::setup::{GroupParameters, Parameters};
use crate::utils::{hash_to_scalar, Signature, SignerIndex};

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

pub struct PayInfo {
    pub info: [u8; 32],
}

#[derive(Debug, Clone)]
pub struct Payment {}

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
        V: u64,
    ) -> Result<(Payment, &Self)> {
        if self.l() + V > L {
            return Err(DivisibleEcashError::Spend(
                "The counter l is higher than max L".to_string(),
            ));
        }

        let grp = params.get_grp();
        let paramsU = params.get_paramsUser();
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
        let phi1 = grp.gen1() * r1;
        let phi2 = paramsU.get_ith_sigma(self.l.get() as usize) * self.v + paramsU.get_ith_eta(V as usize) * r1;

        // compute hash of the payment info
        let rr = hash_to_scalar(pay_info.info);
        let rho1 = grp.gen1() * r2;
        let rho2 = (grp.gen1() * rr) * sk_user.sk + paramsU.get_ith_theta(self.l.get() as usize) * self.v + paramsU.get_ith_eta(V as usize) * r2;

        // compute the zk proof

        // output pay and updated wallet
        let pay = Payment {};

        Ok((pay, self))
    }
}