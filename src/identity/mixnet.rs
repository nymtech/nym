use curve25519_dalek::montgomery::MontgomeryPoint;
use curve25519_dalek::scalar::Scalar;

pub struct KeyPair {
    pub private: Scalar,
    pub public: MontgomeryPoint,
}

impl KeyPair {
    pub fn new() -> KeyPair {
        let (private, public) = sphinx::crypto::keygen();
        KeyPair { private, public }
    }

    pub fn private_bytes(&self) -> Vec<u8> {
        self.private.to_bytes().to_vec()
    }

    pub fn public_bytes(&self) -> Vec<u8> {
        self.public.to_bytes().to_vec()
    }
}
