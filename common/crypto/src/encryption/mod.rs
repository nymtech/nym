use crate::{PemStorableKey, PemStorableKeyPair};
use curve25519_dalek::montgomery::MontgomeryPoint;
use curve25519_dalek::scalar::Scalar;

// TODO: ensure this is a proper name for this considering we are not implementing entire DH here

const CURVE_GENERATOR: MontgomeryPoint = curve25519_dalek::constants::X25519_BASEPOINT;

pub struct KeyPair {
    pub(crate) private_key: PrivateKey,
    pub(crate) public_key: PublicKey,
}

impl KeyPair {
    pub fn new() -> Self {
        let mut rng = rand_os::OsRng::new().unwrap();
        let private_key_value = Scalar::random(&mut rng);
        let public_key_value = CURVE_GENERATOR * private_key_value;

        KeyPair {
            private_key: PrivateKey(private_key_value),
            public_key: PublicKey(public_key_value),
        }
    }

    pub fn private_key(&self) -> &PrivateKey {
        &self.private_key
    }

    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }

    pub fn from_bytes(priv_bytes: &[u8], pub_bytes: &[u8]) -> Self {
        KeyPair {
            private_key: PrivateKey::from_bytes(priv_bytes),
            public_key: PublicKey::from_bytes(pub_bytes),
        }
    }
}

impl Default for KeyPair {
    fn default() -> Self {
        KeyPair::new()
    }
}

impl PemStorableKeyPair for KeyPair {
    type PrivatePemKey = PrivateKey;
    type PublicPemKey = PublicKey;

    fn private_key(&self) -> &Self::PrivatePemKey {
        self.private_key()
    }

    fn public_key(&self) -> &Self::PublicPemKey {
        self.public_key()
    }

    fn from_bytes(priv_bytes: &[u8], pub_bytes: &[u8]) -> Self {
        Self::from_bytes(priv_bytes, pub_bytes)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PrivateKey(pub Scalar);

impl PrivateKey {
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes().to_vec()
    }

    pub fn from_bytes(b: &[u8]) -> Self {
        let mut bytes = [0; 32];
        bytes.copy_from_slice(&b[..]);
        // due to trait restriction we have no choice but to panic if this fails
        let key = Scalar::from_canonical_bytes(bytes).unwrap();
        Self(key)
    }

    pub fn inner(&self) -> Scalar {
        self.0
    }
}

impl PemStorableKey for PrivateKey {
    fn pem_type(&self) -> String {
        String::from("X25519 PRIVATE KEY")
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PublicKey(pub MontgomeryPoint);

impl<'a> From<&'a PrivateKey> for PublicKey {
    fn from(pk: &'a PrivateKey) -> Self {
        PublicKey(CURVE_GENERATOR * pk.0)
    }
}

impl PublicKey {
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes().to_vec()
    }

    pub fn from_bytes(b: &[u8]) -> Self {
        let mut bytes = [0; 32];
        bytes.copy_from_slice(&b[..]);
        let key = MontgomeryPoint(bytes);
        Self(key)
    }

    pub fn inner(&self) -> MontgomeryPoint {
        self.0
    }

    pub fn to_base58_string(&self) -> String {
        bs58::encode(&self.to_bytes()).into_string()
    }

    pub fn from_base58_string(val: String) -> Self {
        Self::from_bytes(&bs58::decode(&val).into_vec().unwrap())
    }
}

impl PemStorableKey for PublicKey {
    fn pem_type(&self) -> String {
        String::from("X25519 PUBLIC KEY")
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
}
