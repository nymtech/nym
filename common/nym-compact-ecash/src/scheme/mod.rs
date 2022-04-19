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
        skUser: &SecretKeyUser,
        payInfo: &PayInfo,
    ) -> Result<(Payment, &Self)> {
        if self.l() > params.L() {
            return Err(CompactEcashError::Spend(
                "The counter l is higher than max L".to_string(),
            ));
        }

        let grparams = params.grp();
        // randomize signature in the wallet
        let (signature_prime, sign_blinding_factor) = self.signature().randomise(grparams);
        // construct kappa i.e., blinded attributes for show
        let attributes = vec![skUser.sk, self.v(), self.t()];
        // compute kappa
        let kappa = compute_kappa(
            &grparams,
            &verification_key,
            &attributes,
            sign_blinding_factor,
        );

        // pick random openings o_a, o_c, o_d
        let o_a = grparams.random_scalar();
        let o_c = grparams.random_scalar();
        let o_d = grparams.random_scalar();

        // compute commitments A, C, D
        let A = grparams.gen1() * o_a + grparams.gamma1() * Scalar::from(self.l());
        let C = grparams.gen1() * o_c + grparams.gamma1() * self.v();
        let D = grparams.gen1() * o_d + grparams.gamma1() * self.t();

        // compute hash of the payment info
        let R = hash_to_scalar(payInfo.info);

        // evaluate the pseudorandom functions
        let S = pseudorandom_fgv(&grparams, self.v(), self.l());
        let T = grparams.gen1() * skUser.sk + pseudorandom_fgt(&grparams, self.t(), self.l()) * R;

        // compute values mu, o_mu, lambda, o_lambda
        let mu: Scalar = (self.v() + Scalar::from(self.l()) + Scalar::from(1))
            .invert()
            .unwrap();
        let o_mu = ((o_a + o_c) * mu).neg();
        let lambda = (self.t() + Scalar::from(self.l()) + Scalar::from(1))
            .invert()
            .unwrap();
        let o_lambda = ((o_a + o_d) * lambda).neg();

        // parse the signature associated with value l
        let sign_l = params.get_sign_by_idx(self.l());
        // randomise the signature associated with value l
        let (sign_l_prime, sign_l_blinding_factor) = sign_l.randomise(grparams);
        // compute kappa_l
        let kappa_l = grparams.gen2() * sign_l_blinding_factor
            + params.pkRP().alpha
            + params.pkRP().beta * Scalar::from(self.l());

        // construct the zkp proof
        let spendInstance = SpendInstance {
            kappa,
            A,
            C,
            D,
            S,
            T,
            kappa_l,
        };
        let spendWitness = SpendWitness {
            attributes,
            r: sign_blinding_factor,
            r_l: sign_l_blinding_factor,
            l: Scalar::from(self.l()),
            o_a,
            o_c,
            o_d,
            mu,
            lambda,
            o_mu,
            o_lambda,
        };
        let zk_proof =
            SpendProof::construct(&params, &spendInstance, &spendWitness, &verification_key, R);

        // output pay and updated wallet
        let pay = Payment {
            kappa,
            sig: signature_prime,
            S,
            T,
            A,
            C,
            D,
            R,
            kappa_l,
            sig_l: sign_l_prime,
            zk_proof,
        };

        self.up();

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

pub struct PayInfo {
    pub(crate) info: [u8; 32],
}

#[derive(Debug, Clone)]
pub struct Payment {
    pub kappa: G2Projective,
    pub sig: Signature,
    pub S: G1Projective,
    pub T: G1Projective,
    pub A: G1Projective,
    pub C: G1Projective,
    pub D: G1Projective,
    pub R: Scalar,
    pub kappa_l: G2Projective,
    pub sig_l: Signature,
    pub zk_proof: SpendProof,
}

impl Payment {
    pub fn spend_verify(
        &self,
        params: &Parameters,
        verification_key: &VerificationKeyAuth,
        payinfo: &PayInfo,
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

        // verify integrity of R
        if !(self.R == hash_to_scalar(payinfo.info)) {
            return Err(CompactEcashError::Spend(
                "Integrity of R does not hold".to_string(),
            ));
        }

        //TODO: verify whether payinfo contains merchent's identifier

        // verify the zk proof
        let instance = SpendInstance {
            kappa: self.kappa,
            A: self.A,
            C: self.C,
            D: self.D,
            S: self.S,
            T: self.T,
            kappa_l: self.kappa_l,
        };

        if !self
            .zk_proof
            .verify(&params, &instance, &verification_key, self.R)
        {
            return Err(CompactEcashError::Spend(
                "ZkProof verification failed".to_string(),
            ));
        }

        Ok(true)
    }
}
