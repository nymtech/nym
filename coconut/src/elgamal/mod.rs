use crate::error::Result;
use crate::parameters::SomeRngTrait;
use crate::Params;

// Note2: all placeholder types are denoted as `()`

// Those will obviously be from an external library
type Scalar = ();
type G1Point = ();

pub struct Ciphertext(G1Point, G1Point);

pub struct EncryptionResult {
    ciphertext: Ciphertext,
    k: Scalar,
}

impl EncryptionResult {}

pub struct PrivateKey {
    d: Scalar,
}

impl PrivateKey {
    /// Decrypt takes the ElGamal encryption of a message and returns a point on the G1 curve
    /// that represents original h^m.
    pub fn decrypt(&self, ciphertext: Ciphertext) -> G1Point {
        unimplemented!()
    }

    pub fn public_key<R: SomeRngTrait>(&self, params: &Params<R>) -> PublicKey {
        unimplemented!()
    }
}

pub struct PublicKey {
    group_order: (), // presumably Scalar?
    gen1: G1Point,
    gamma: G1Point, // g1^d
}

impl PublicKey {
    /// Encrypt encrypts the given message in the form of h^m,
    /// where h is a point on the G1 curve using the given public key.
    /// The random k is returned alongside the encryption
    /// as it is required by the Coconut Scheme to create proofs of knowledge.
    pub fn encrypt<R: SomeRngTrait>(
        &self,
        params: &Params<R>,
        h: G1Point,
        m: Scalar,
    ) -> EncryptionResult {
        unimplemented!()
    }
}

pub struct Keypair {
    private_key: PrivateKey,
    public_key: PublicKey,
}

pub fn keygen<R: SomeRngTrait>(params: &Params<R>) -> Keypair {
    unimplemented!()
}
