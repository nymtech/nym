use crate::encryption::{
    MixnetEncryptionKeyPair, MixnetEncryptionPrivateKey, MixnetEncryptionPublicKey,
};
use crate::PemStorable;
use curve25519_dalek::montgomery::MontgomeryPoint;
use curve25519_dalek::scalar::Scalar;

// TODO: ensure this is a proper name for this considering we are not implementing entire DH here

pub const CURVE_GENERATOR: MontgomeryPoint = curve25519_dalek::constants::X25519_BASEPOINT;

pub struct KeyPair {
    pub(crate) private_key: PrivateKey,
    pub(crate) public_key: PublicKey,
}

impl MixnetEncryptionKeyPair<PrivateKey, PublicKey> for KeyPair {
    fn new() -> Self {
        let mut rng = rand_os::OsRng::new().unwrap();
        let private_key_value = Scalar::random(&mut rng);
        let public_key_value = CURVE_GENERATOR * private_key_value;

        KeyPair {
            private_key: PrivateKey(private_key_value),
            public_key: PublicKey(public_key_value),
        }
    }

    fn private_key(&self) -> &PrivateKey {
        &self.private_key
    }

    fn public_key(&self) -> &PublicKey {
        &self.public_key
    }

    fn from_bytes(priv_bytes: &[u8], pub_bytes: &[u8]) -> Self {
        KeyPair {
            private_key: PrivateKey::from_bytes(priv_bytes),
            public_key: PublicKey::from_bytes(pub_bytes),
        }
    }
}

// COPY IS DERIVED ONLY TEMPORARILY UNTIL https://github.com/nymtech/nym/issues/47 is fixed
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct PrivateKey(pub Scalar);

impl MixnetEncryptionPrivateKey for PrivateKey {
    type PublicKeyMaterial = PublicKey;

    fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes().to_vec()
    }

    fn from_bytes(b: &[u8]) -> Self {
        let mut bytes = [0; 32];
        bytes.copy_from_slice(&b[..]);
        let key = Scalar::from_canonical_bytes(bytes).unwrap();
        Self(key)
    }
}

impl PemStorable for PrivateKey {
    fn pem_type(&self) -> String {
        String::from("X25519 PRIVATE KEY")
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PublicKey(pub MontgomeryPoint);

impl<'a> From<&'a PrivateKey> for PublicKey {
    fn from(pk: &'a PrivateKey) -> Self {
        PublicKey(CURVE_GENERATOR * pk.0)
    }
}

impl MixnetEncryptionPublicKey for PublicKey {
    type PrivateKeyMaterial = PrivateKey;

    fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes().to_vec()
    }

    fn from_bytes(b: &[u8]) -> Self {
        let mut bytes = [0; 32];
        bytes.copy_from_slice(&b[..]);
        let key = MontgomeryPoint(bytes);
        Self(key)
    }
}

impl PemStorable for PublicKey {
    fn pem_type(&self) -> String {
        String::from("X25519 PUBLIC KEY")
    }
}
