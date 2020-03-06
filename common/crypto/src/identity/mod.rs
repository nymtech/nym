use crate::{encryption, PemStorableKey, PemStorableKeyPair};
use bs58;
use curve25519_dalek::scalar::Scalar;
use sphinx::route::DestinationAddressBytes;

// for time being define a dummy identity using x25519 encryption keys (as we've done so far)
// and replace it with proper keys, like ed25519 later on
#[derive(Clone)]
pub struct MixIdentityKeyPair {
    pub private_key: MixIdentityPrivateKey,
    pub public_key: MixIdentityPublicKey,
}

impl MixIdentityKeyPair {
    pub fn new() -> Self {
        let keypair = encryption::KeyPair::new();
        MixIdentityKeyPair {
            private_key: MixIdentityPrivateKey(keypair.private_key),
            public_key: MixIdentityPublicKey(keypair.public_key),
        }
    }

    pub fn private_key(&self) -> &MixIdentityPrivateKey {
        &self.private_key
    }

    pub fn public_key(&self) -> &MixIdentityPublicKey {
        &self.public_key
    }

    pub fn from_bytes(priv_bytes: &[u8], pub_bytes: &[u8]) -> Self {
        MixIdentityKeyPair {
            private_key: MixIdentityPrivateKey::from_bytes(priv_bytes),
            public_key: MixIdentityPublicKey::from_bytes(pub_bytes),
        }
    }
}

impl Default for MixIdentityKeyPair {
    fn default() -> Self {
        MixIdentityKeyPair::new()
    }
}

impl PemStorableKeyPair for MixIdentityKeyPair {
    type PrivatePemKey = MixIdentityPrivateKey;
    type PublicPemKey = MixIdentityPublicKey;

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
pub struct MixIdentityPublicKey(encryption::PublicKey);

impl MixIdentityPublicKey {
    pub fn derive_address(&self) -> DestinationAddressBytes {
        let mut temporary_address = [0u8; 32];
        let public_key_bytes = self.to_bytes();
        temporary_address.copy_from_slice(&public_key_bytes[..]);

        DestinationAddressBytes::from_bytes(temporary_address)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes()
    }

    pub fn from_bytes(b: &[u8]) -> Self {
        Self(encryption::PublicKey::from_bytes(b))
    }

    pub fn to_base58_string(&self) -> String {
        bs58::encode(&self.to_bytes()).into_string()
    }

    pub fn from_base58_string(val: String) -> Self {
        Self::from_bytes(&bs58::decode(&val).into_vec().unwrap())
    }
}

impl PemStorableKey for MixIdentityPublicKey {
    fn pem_type(&self) -> String {
        format!("DUMMY KEY BASED ON {}", self.0.pem_type())
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MixIdentityPrivateKey(pub encryption::PrivateKey);

impl<'a> From<&'a MixIdentityPrivateKey> for MixIdentityPublicKey {
    fn from(pk: &'a MixIdentityPrivateKey) -> Self {
        let private_ref = &pk.0;
        let public: encryption::PublicKey = private_ref.into();
        MixIdentityPublicKey(public)
    }
}

impl MixIdentityPrivateKey {
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes()
    }

    pub fn from_bytes(b: &[u8]) -> Self {
        Self(encryption::PrivateKey::from_bytes(b))
    }
}

// TODO: this will be implemented differently by using the proper trait
impl MixIdentityPrivateKey {
    pub fn as_scalar(&self) -> Scalar {
        let encryption_key = &self.0;
        encryption_key.0
    }
}

impl PemStorableKey for MixIdentityPrivateKey {
    fn pem_type(&self) -> String {
        format!("DUMMY KEY BASED ON {}", self.0.pem_type())
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
}
