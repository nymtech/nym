use crate::encryption::{
    MixnetEncryptionKeyPair, MixnetEncryptionPrivateKey, MixnetEncryptionPublicKey,
};
use crate::{encryption, PemStorable};
use bs58;
use curve25519_dalek::scalar::Scalar;
use sphinx::route::DestinationAddressBytes;

pub trait MixnetIdentityKeyPair<Priv, Pub>: Clone
where
    Priv: MixnetIdentityPrivateKey,
    Pub: MixnetIdentityPublicKey,
{
    fn new() -> Self;
    fn private_key(&self) -> &Priv;
    fn public_key(&self) -> &Pub;
    fn from_bytes(priv_bytes: &[u8], pub_bytes: &[u8]) -> Self;

    // TODO: signing related methods
}

pub trait MixnetIdentityPublicKey:
    Sized
    + PemStorable
    + Clone
    + for<'a> From<&'a <Self as MixnetIdentityPublicKey>::PrivateKeyMaterial>
{
    // we need to couple public and private keys together
    type PrivateKeyMaterial: MixnetIdentityPrivateKey<PublicKeyMaterial = Self>;

    fn derive_address(&self) -> DestinationAddressBytes;
    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(b: &[u8]) -> Self;
}

pub trait MixnetIdentityPrivateKey: Sized + PemStorable + Clone {
    // we need to couple public and private keys together
    type PublicKeyMaterial: MixnetIdentityPublicKey<PrivateKeyMaterial = Self>;

    /// Returns the associated public key
    fn public_key(&self) -> Self::PublicKeyMaterial {
        self.into()
    }

    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(b: &[u8]) -> Self;
}

// same for validator

// for time being define a dummy identity using x25519 encryption keys (as we've done so far)
// and replace it with proper keys, like ed25519 later on
#[derive(Clone)]
pub struct DummyMixIdentityKeyPair {
    pub private_key: DummyMixIdentityPrivateKey,
    pub public_key: DummyMixIdentityPublicKey,
}

impl MixnetIdentityKeyPair<DummyMixIdentityPrivateKey, DummyMixIdentityPublicKey>
    for DummyMixIdentityKeyPair
{
    fn new() -> Self {
        let keypair = encryption::x25519::KeyPair::new();
        DummyMixIdentityKeyPair {
            private_key: DummyMixIdentityPrivateKey(keypair.private_key),
            public_key: DummyMixIdentityPublicKey(keypair.public_key),
        }
    }

    fn private_key(&self) -> &DummyMixIdentityPrivateKey {
        &self.private_key
    }

    fn public_key(&self) -> &DummyMixIdentityPublicKey {
        &self.public_key
    }

    fn from_bytes(priv_bytes: &[u8], pub_bytes: &[u8]) -> Self {
        DummyMixIdentityKeyPair {
            private_key: DummyMixIdentityPrivateKey::from_bytes(priv_bytes),
            public_key: DummyMixIdentityPublicKey::from_bytes(pub_bytes),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DummyMixIdentityPublicKey(encryption::x25519::PublicKey);

impl MixnetIdentityPublicKey for DummyMixIdentityPublicKey {
    type PrivateKeyMaterial = DummyMixIdentityPrivateKey;

    fn derive_address(&self) -> DestinationAddressBytes {
        let mut temporary_address = [0u8; 32];
        let public_key_bytes = self.to_bytes();
        temporary_address.copy_from_slice(&public_key_bytes[..]);

        temporary_address
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes()
    }

    fn from_bytes(b: &[u8]) -> Self {
        Self(encryption::x25519::PublicKey::from_bytes(b))
    }
}

impl PemStorable for DummyMixIdentityPublicKey {
    fn pem_type(&self) -> String {
        format!("DUMMY KEY BASED ON {}", self.0.pem_type())
    }
}

impl DummyMixIdentityPublicKey {
    pub fn to_base58_string(&self) -> String {
        bs58::encode(&self.to_bytes()).into_string()
    }

    #[allow(dead_code)]
    fn from_base58_string(val: String) -> Self {
        Self::from_bytes(&bs58::decode(&val).into_vec().unwrap())
    }
}

// COPY IS DERIVED ONLY TEMPORARILY UNTIL https://github.com/nymtech/nym/issues/47 is fixed
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct DummyMixIdentityPrivateKey(pub encryption::x25519::PrivateKey);

impl<'a> From<&'a DummyMixIdentityPrivateKey> for DummyMixIdentityPublicKey {
    fn from(pk: &'a DummyMixIdentityPrivateKey) -> Self {
        let private_ref = &pk.0;
        let public: encryption::x25519::PublicKey = private_ref.into();
        DummyMixIdentityPublicKey(public)
    }
}

impl MixnetIdentityPrivateKey for DummyMixIdentityPrivateKey {
    type PublicKeyMaterial = DummyMixIdentityPublicKey;

    fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes()
    }

    fn from_bytes(b: &[u8]) -> Self {
        Self(encryption::x25519::PrivateKey::from_bytes(b))
    }
}

// TODO: this will be implemented differently by using the proper trait
impl DummyMixIdentityPrivateKey {
    pub fn as_scalar(self) -> Scalar {
        let encryption_key = self.0;
        encryption_key.0
    }
}

impl PemStorable for DummyMixIdentityPrivateKey {
    fn pem_type(&self) -> String {
        format!("DUMMY KEY BASED ON {}", self.0.pem_type())
    }
}
