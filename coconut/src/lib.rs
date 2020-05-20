use crate::error::Result;
pub use parameters::Params;
// so that you could make a direct call like `coconut::keygen(params)`
pub use scheme::{
    issue_credential::{self, blind_sign, prepare_blind_sign, sign, unblind, Credential},
    keygen::{keygen, trusted_third_party_keygen, Keypair, SecretKey, VerificationKey},
    randomize_credential,
    show_credential::{self, blind_verify_credential, prove_credential, verify_credential},
    Attribute,
};

pub mod elgamal;
pub mod error;
pub mod parameters;
pub mod proofs;
pub mod scheme;
mod utils;

// Those will obviously be from an external library
type Scalar = ();
type G1Point = ();
type G2Point = ();

// Note: where I couldn't immediately think of better "human" name for variables
// I've kept paper names. Perhaps Ania or Claudia (or George/Alberto or even Alfredo) could help with those

// Note2: all placeholder types are denoted as `()`

// Note3: I've went ahead and kept borrowing data and passing it by value, however
// in further development, once things start to more closely interact with each other
// it might make sense to take ownership over certain structs.

// FLOW:
/*
    // A: represents step done by signing authority
    // U: represents step done by user
    // V: represents step done by verifier
    // TTP is a trusted third party for the keygen (this will need to be replaced by some distributed key generation algorithm)

   1) Only public attributes, single authority
        - U+A+V:    coconut::setup(n_attr) -> Params
        - A:        coconut::keygen(params) -> (sk, vk)
        - A:        coconut::sign(params, sk, public_attrs) OR sk.sign(params, public_attrs) -> Credential
        - V:        coconut::verify_credential(params, vk, public_attrs, credential) OR vk.verify(params, public_attrs, credential) -> bool

   2) 'Normal' coconut with mix of public and private attributes and multiple authorities:
        - U+A+V:    coconut::setup(n_attr) -> Params
        - TTP:      coconut::trusted_third_party_keygen(params, threshold, authorities) -> Vec<(sk, vk)>
        - U:        coconut::elgamal::keygen() -> (Epriv, Epub)
        - U:        coconut::prepare_blind_sign(params, Epub, public_attrs, private_attrs) -> Lambda
        - A:        coconut::blind_sign(params, sk, lambda, Epub, public_attrs) OR sk.blind_sign(params, lambda, Epub, public_attrs) -> BlindedCredential
        - U:        coconut::unblind(params, blinded_cred, Epriv) -> Credential
        - U:        coconut::randomize(params, credential) -> Credential OR credential.randomize(params)
        - U:        coconut::aggregate_credentials(params, Vec<Credential>, Vec<Index>) -> Credential
        - U+V:      coconut::aggregate_verification_keys(params, Vec<VerificationKey>, Vec<Index>) -> VerificationKey
        // Index has to do with shares
        - U:        coconut::prove_credential(params, vk, credential, private_attributes) -> Theta
        - V:        coconut::blind_verify_credential(params, vk, credential, theta, public_attributes) OR vk.blind_verify_credential(params, credential, theta, public_attributes) -> bool
*/

// only public:
// keygen -> sign -> verify -> success

// public + private:
