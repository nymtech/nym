use crate::issue_credential::{BlindedCredential, Lambda};
use crate::parameters::SomeRngTrait;
use crate::show_credential::Theta;
use crate::{elgamal, Attribute, Credential, G2Point, Params, Result, Scalar};

pub struct SecretKey {
    x: Scalar,
    ys: Vec<Scalar>,
}

impl SecretKey {
    pub fn sign<R: SomeRngTrait>(
        &self,
        params: &Params<R>,
        public_attributes: &[&Attribute],
    ) -> Result<Credential> {
        crate::sign(params, self, public_attributes)
    }

    pub fn blind_sign<R: SomeRngTrait>(
        &self,
        params: &Params<R>,
        lambda: &Lambda,
        pub_key: &elgamal::PublicKey,
        public_attributes: &[&Attribute],
    ) -> Result<BlindedCredential> {
        crate::issue_credential::blind_sign(params, self, lambda, pub_key, public_attributes)
    }
}

pub struct VerificationKey {
    gen2: G2Point,
    alpha: G2Point,
    beta: Vec<G2Point>,
}

impl VerificationKey {
    pub fn verify<R: SomeRngTrait>(
        &self,
        params: &Params<R>,
        public_attributes: &[&Attribute],
        credential: &Credential,
    ) -> bool {
        crate::verify_credential(params, self, public_attributes, credential)
    }
    pub fn blind_verify<R: SomeRngTrait>(
        &self,
        params: &Params<R>,
        credential: &Credential,
        theta: &Theta,
        public_attributes: &[&Attribute],
    ) -> bool {
        crate::blind_verify_credential(params, self, credential, theta, public_attributes)
    }
}

pub struct Keypair {
    secret_key: SecretKey,
    verification_key: VerificationKey,
}

/// `keygen` generates a single Coconut keypair ((x, y1, y2...), (g2, g2^x, g2^y1, ...)).
/// It is not suitable for threshold credentials as all generated keys
/// are independent of each other.
pub fn keygen<R: SomeRngTrait>(params: &Params<R>) -> Result<Keypair> {
    unimplemented!()
}

/// `trusted_third_party_keygen` generates a set of `num_authorities` Coconut keypairs
/// [((x, y1, y2...), (g2, g2^x, g2^y1, ...)), ...],
/// such that they support threshold aggregation of `threshold` parties.
/// It is expected that this procedure is executed by a Trusted Third Party.
pub fn trusted_third_party_keygen<R: SomeRngTrait>(
    params: &Params<R>,
    threshold: u32,
    num_authorities: u32,
) -> Result<Vec<Keypair>> {
    unimplemented!()
}
