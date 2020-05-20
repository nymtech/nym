use crate::error::Result;
use crate::parameters::SomeRngTrait;
use crate::proofs::pi_S;
use crate::{elgamal, Attribute, G1Point, Params, SecretKey};

pub struct Credential {
    h: G1Point,
    sigma: G1Point,
}

impl Credential {
    fn randomize<R: SomeRngTrait>(&self, params: &Params<R>) -> Self {
        crate::randomize_credential(params, self)
    }
}

pub struct BlindedCredential {
    h: G1Point,
    c_tilde: elgamal::Ciphertext,
}

impl BlindedCredential {
    // TODO: perhaps take self by value
    fn unblind<R: SomeRngTrait>(
        &self,
        params: &Params<R>,
        priv_key: &elgamal::PrivateKey,
    ) -> Credential {
        crate::unblind(params, self, priv_key)
    }
}

pub struct Lambda {
    commitment: G1Point,
    ciphertexts: Vec<elgamal::Ciphertext>,
    proof: pi_S,
}

// TODO: possibly completely get rid of this trivial case as most likely it will
// never be used

/// `sign` creates a Coconut credential under a given secret key on a set of public attributes only.
pub fn sign<R: SomeRngTrait>(
    params: &Params<R>,
    secret_key: &SecretKey,
    public_attributes: &[&Attribute],
) -> Result<Credential> {
    unimplemented!()
}

/// prepare_blind_sign builds cryptographic material for blind sign.
/// It returns commitment to the private and public attributes,
/// encryptions of the private attributes
/// and zero-knowledge proof asserting correctness of the above.
pub fn prepare_blind_sign<R: SomeRngTrait>(
    params: &Params<R>,
    pub_key: &elgamal::PublicKey,
    public_attributes: &[&Attribute],
    private_attributes: &[&Attribute],
) -> Result<Lambda> {
    unimplemented!()
}

/// `blind_sign` creates a blinded Coconut credential on the attributes provided to `prepare_blind_sign`.
pub fn blind_sign<R: SomeRngTrait>(
    params: &Params<R>,
    secret_key: &SecretKey,
    lambda: &Lambda,
    pub_key: &elgamal::PublicKey,
    public_attributes: &[&Attribute],
) -> Result<BlindedCredential> {
    unimplemented!()
}

// TODO: perhaps take blinded_signature by value
/// `unblind` unblinds the blinded Coconut credential.
pub fn unblind<R: SomeRngTrait>(
    params: &Params<R>,
    blinded_credential: &BlindedCredential,
    priv_key: &elgamal::PrivateKey,
) -> Credential {
    unimplemented!()
}
