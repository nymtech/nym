use crate::encryption;
use crate::encryption::{
    MixnetEncryptionKeyPair, MixnetEncryptionPrivateKey, MixnetEncryptionPublicKey,
};

pub trait MixnetIdentityKeyPair<Priv, Pub>
where
    Priv: MixnetIdentityPrivateKey,
    Pub: MixnetIdentityPublicKey,
{
    fn new() -> Self;
    fn private_key(&self) -> &Priv;
    fn public_key(&self) -> &Pub;

    // TODO: signing related methods
}

pub trait MixnetIdentityPublicKey:
    Sized + for<'a> From<&'a <Self as MixnetIdentityPublicKey>::PrivateKeyMaterial>
{
    // we need to couple public and private keys together
    type PrivateKeyMaterial: MixnetIdentityPrivateKey<PublicKeyMaterial = Self>;

    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(b: &[u8]) -> Self;
}

pub trait MixnetIdentityPrivateKey: Sized {
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

// TODO: SUPER TEMPORARY:
// for time being define a dummy identity using x25519 encryption keys (as we've done so far)

struct DummyMixIdentityKeyPair {
    private_key: DummyMixIdentityPrivateKey,
    public_key: DummyMixIdentityPublicKey,
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
}

struct DummyMixIdentityPublicKey(encryption::x25519::PublicKey);

impl MixnetIdentityPublicKey for DummyMixIdentityPublicKey {
    type PrivateKeyMaterial = DummyMixIdentityPrivateKey;

    fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes()
    }

    fn from_bytes(b: &[u8]) -> Self {
        Self(encryption::x25519::PublicKey::from_bytes(b))
    }
}

struct DummyMixIdentityPrivateKey(encryption::x25519::PrivateKey);

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
