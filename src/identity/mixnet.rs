use curve25519_dalek::montgomery::MontgomeryPoint;
use curve25519_dalek::scalar::Scalar;

// This keypair serves as the user's identity within the Mixnet
pub struct KeyPair {
    pub private: Scalar,
    pub public: MontgomeryPoint,
}

impl KeyPair {
    pub fn new() -> KeyPair {
        let (private, public) = sphinx::crypto::keygen();
        KeyPair { private, public }
    }

    pub fn from_bytes(private_bytes: Vec<u8>, public_bytes: Vec<u8>) -> KeyPair {
        let mut bytes = [0; 32];
        bytes.copy_from_slice(&private_bytes[..]);
        let private = Scalar::from_canonical_bytes(bytes).unwrap();

        let mut bytes = [0; 32];
        bytes.copy_from_slice(&public_bytes[..]);
        let public = MontgomeryPoint(bytes);

        KeyPair { private, public }
    }

    pub fn private_bytes(&self) -> Vec<u8> {
        self.private.to_bytes().to_vec()
    }

    pub fn public_bytes(&self) -> Vec<u8> {
        self.public.to_bytes().to_vec()
    }
}
