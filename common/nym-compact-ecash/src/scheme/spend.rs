use std::cell::Cell;
use std::convert::TryInto;

use bls12_381::{G1Projective, G2Prepared, G2Projective, Scalar};
use group::Curve;

use crate::Attribute;
use crate::error::{CompactEcashError, Result};
use crate::proofs::proof_spend::{SpendInstance, SpendProof, SpendWitness};
use crate::scheme::{Signature, Wallet};
use crate::scheme::keygen::{SecretKeyUser, VerificationKeyAuth};
use crate::scheme::setup::Parameters;
use crate::utils::{check_bilinear_pairing, hash_to_scalar};

pub struct PayInfo {
    pub(crate) info: [u8; 32],
}

pub struct Payment {
    kappa: G2Projective,
    sig: Signature,
    S: G1Projective,
    T: G1Projective,
    A: G1Projective,
    C: G1Projective,
    D: G1Projective,
    R: Scalar,
    zk_proof: SpendProof,
}

pub fn pseudorandom_fgv(params: &Parameters, v: Scalar, l: u64) -> G1Projective {
    let pow = (v + Scalar::from(l) + Scalar::from(1)).neg();
    params.gen1() * pow
}

pub fn pseudorandom_fgt(params: &Parameters, t: Scalar, l: u64) -> G1Projective {
    let pow = (t + Scalar::from(l) + Scalar::from(1)).neg();
    params.gen1() * pow
}

pub fn compute_kappa(
    params: &Parameters,
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

pub fn spend(params: &Parameters, wallet: &Wallet, verification_key: &VerificationKeyAuth, skUser: &SecretKeyUser, payInfo: &PayInfo) -> Result<(Payment, Wallet)> {
    if wallet.l() > params.L() {
        return Err(CompactEcashError::Spend(
            "The counter l is higher than max L".to_string(),
        ));
    }

    // randomize signature in the wallet
    let (signature_prime, sign_blinding_factor) = wallet.signature().randomise(params);
    // construct kappa i.e., blinded attributes for show
    let attributes = vec![skUser.sk, wallet.v(), wallet.t()];
    let kappa = compute_kappa(&params, &verification_key, &attributes, sign_blinding_factor);

    // pick random openings o_a, o_c, o_d
    let o_a = params.random_scalar();
    let o_c = params.random_scalar();
    let o_d = params.random_scalar();

    // compute commitments A, C, D
    let A = params.gen1() * o_a + params.gamma1().unwrap() * Scalar::from(wallet.l());
    let C = params.gen1() * o_c + params.gamma1().unwrap() * wallet.v();
    let D = params.gen1() * o_d + params.gamma1().unwrap() * wallet.t();

    // compute hash of the payment info
    let R = hash_to_scalar(payInfo.info);

    // evaluate the pseudorandom functions
    let S = pseudorandom_fgv(&params, wallet.v(), wallet.l());
    let T = params.gen1() * skUser.sk + pseudorandom_fgt(&params, wallet.t(), wallet.l()) * R;

    // compute values mu, o_mu, lambda, o_lambda
    let mu: Scalar = (wallet.v() + Scalar::from(wallet.l()) + Scalar::from(1)).neg();
    let o_mu = ((o_a + o_c) * mu).neg();
    let lambda = (wallet.t() + Scalar::from(wallet.l()) + Scalar::from(1)).neg();
    let o_lambda = ((o_a + o_d) * lambda).neg();

    // construct the zkp proof
    let spendInstance = SpendInstance {
        kappa,
        A,
        C,
        D,
        S,
        T,
    };
    let spendWitness = SpendWitness {
        attributes,
        r: sign_blinding_factor,
        l: Scalar::from(wallet.l()),
        o_a,
        o_c,
        o_d,
        mu,
        lambda,
        o_mu,
        o_lambda,
    };
    let zk_proof = SpendProof::construct(&params, &spendInstance, &spendWitness, &verification_key, R);

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
        zk_proof,
    };
    let wallet_upd = Wallet {
        sig: wallet.sig,
        v: wallet.v,
        t: wallet.t,
        l: Cell::new(wallet.l.get() + 1),
    };

    Ok((pay, wallet_upd))
}

pub fn spend_verify(params: &Parameters, verification_key: VerificationKeyAuth, pay: Payment, payinfo: PayInfo) -> Result<bool> {
    if bool::from(pay.sig.0.is_identity()) {
        return Err(CompactEcashError::Spend(
            "The element h of the signature equals the identity".to_string(),
        ));
    }

    if !check_bilinear_pairing(
        &pay.sig.0.to_affine(),
        &G2Prepared::from(pay.kappa.to_affine()),
        &pay.sig.1.to_affine(),
        params.prepared_miller_g2(),
    ) {
        return Err(CompactEcashError::Spend(
            "The bilinear check for kappa failed".to_string(),
        ));
    }

    // verify integrity of R
    if !(pay.R == hash_to_scalar(payinfo.info)) {
        return Err(CompactEcashError::Spend(
            "Integrity of R does not hold".to_string(),
        ));
    }

    //TODO: verify whether payinfo contains merchent's identifier 

    // verify the zk proof
    let instance = SpendInstance {
        kappa: pay.kappa,
        A: pay.A,
        C: pay.C,
        D: pay.D,
        S: pay.S,
        T: pay.T,
    };

    if !pay.zk_proof.verify(&params, &instance, &verification_key, pay.R) {
        return Err(CompactEcashError::Spend(
            "ZkProof verification failed".to_string(),
        ));
    }

    Ok(true)
}
