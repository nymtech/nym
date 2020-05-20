use crate::scheme::issue_credential::Lambda;
use crate::scheme::show_credential::Theta;
use crate::{elgamal, Attribute, Credential, G1Point, Params, Result, Scalar, VerificationKey};
use crate::parameters::SomeRngTrait;

pub struct pi_S {
    challenge: Scalar,
    rr: Scalar,
    rk: Vec<Scalar>,
    rm: Vec<Scalar>,
}

// I'm almost certain gamma can be replaced with `elgamal::PublicKey`
impl pi_S {
    // I *think* ciphertexts and k can be combined together via `elgamal::EncryptionResult`
    fn construct<R: SomeRngTrait>(
        params: &Params<R>,
        gamma: &G1Point,
        ciphertexts: &[&elgamal::Ciphertext],
        commitment: &G1Point,
        k: &[&Scalar],
        r: &Scalar,
        public_attributes: &[&Attribute],
        private_attributes: &[&Attribute],
    ) -> Result<Self> {
        unimplemented!()
    }

    // pi_S is part of Lambda
    fn verify<R: SomeRngTrait>(params: &Params<R>, gamma: &G1Point, lambda: &Lambda) -> bool {
        unimplemented!()
    }
}

pub struct pi_V {
    challenge: Scalar,
    rm: Vec<Scalar>,
    rt: Scalar,
}

impl pi_V {
    fn construct<R: SomeRngTrait>(
        params: &Params<R>,
        verification_key: &VerificationKey,
        credential: &Credential,
        private_attributes: &[&Attribute],
        t: &Scalar,
    ) -> Result<Self> {
        unimplemented!()
    }

    // pi_V is part of Theta
    fn verify<R: SomeRngTrait>(
        params: &Params<R>,
        verification_key: &VerificationKey,
        credential: &Credential,
        theta: &Theta,
    ) -> bool {
        unimplemented!()
    }
}
