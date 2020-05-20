use crate::parameters::SomeRngTrait;
use crate::proofs::pi_V;
use crate::{Attribute, Credential, G1Point, G2Point, Params, Result, VerificationKey};

// TODO: it might need to be expanded from the beginning to also contain 'Zeta'
// or have some generic way of doing it
// look: https://github.com/nymtech/nym-validator/blob/develop/crypto/coconut/scheme/tumbler.go#L66
// because presumably we will want to do double spending detection
pub struct Theta {
    kappa: G2Point,
    nu: G1Point,
    proof: pi_V,
}

/// `prove_credential` builds cryptographic material required for blind verification.
/// It returns kappa and nu - group elements needed to perform verification
/// and zero-knowledge proof asserting correctness of the above.
pub fn prove_credential<R: SomeRngTrait>(
    params: &Params<R>,
    verification_key: &VerificationKey,
    credential: &Credential,
    private_attributes: &[&Attribute],
) -> Result<Theta> {
    unimplemented!()
}

/// `blind_verify_credential` verifies the Coconut credential on the private and optional public attributes.
pub fn blind_verify_credential<R: SomeRngTrait>(
    params: &Params<R>,
    verification_key: &VerificationKey,
    credential: &Credential,
    theta: &Theta,
    public_attributes: &[&Attribute],
) -> bool {
    unimplemented!()
}

// TODO: possibly completely get rid of this trivial case as most likely it will
// never be used

/// `verify_credential` verifies the Coconut credential that has been either issued exclusively on public attributes
/// or all private attributes have been publicly revealed
pub fn verify_credential<R: SomeRngTrait>(
    params: &Params<R>,
    verification_key: &VerificationKey,
    public_attributes: &[&Attribute],
    credential: &Credential,
) -> bool {
    unimplemented!()
}
