
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

