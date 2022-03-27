use std::convert::TryInto;

use bls12_381::{G1Projective, G2Projective, Scalar};

use crate::Attribute;
use crate::error::{CompactEcashError, Result};
use crate::proofs::proof_spend::{SpendInstance, SpendProof, SpendWitness};
use crate::scheme::keygen::{SecretKeyUser, VerificationKeyAuth};
use crate::scheme::setup::Parameters;
use crate::scheme::Wallet;
use crate::utils::hash_to_scalar;

pub struct PayInfo {
    pub(crate) info: [u8; 32],
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

pub fn spend(params: &Parameters, wallet: &Wallet, verification_key: &VerificationKeyAuth, skUser: &SecretKeyUser, payInfo: &PayInfo) -> Result<()> {
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
    let zkp = SpendProof::construct(&params, &spendInstance, &spendWitness, &verification_key, R);

    // output pay and updated wallet

    Ok(())
}

pub fn spend_verify() {}
