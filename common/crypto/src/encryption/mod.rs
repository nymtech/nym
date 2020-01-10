use crate::PemStorable;

pub mod x25519;

pub trait MixnetEncryptionKeyPair<Priv, Pub>
where
    Priv: MixnetEncryptionPrivateKey,
    Pub: MixnetEncryptionPublicKey,
{
    fn new() -> Self;
    fn private_key(&self) -> &Priv;
    fn public_key(&self) -> &Pub;

    // TODO: encryption related methods
}

pub trait MixnetEncryptionPublicKey:
    Sized + PemStorable + for<'a> From<&'a <Self as MixnetEncryptionPublicKey>::PrivateKeyMaterial>
{
    // we need to couple public and private keys together
    type PrivateKeyMaterial: MixnetEncryptionPrivateKey<PublicKeyMaterial = Self>;

    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(b: &[u8]) -> Self;
}

pub trait MixnetEncryptionPrivateKey: Sized + PemStorable {
    // we need to couple public and private keys together
    type PublicKeyMaterial: MixnetEncryptionPublicKey<PrivateKeyMaterial = Self>;

    /// Returns the associated public key
    fn public_key(&self) -> Self::PublicKeyMaterial {
        self.into()
    }

    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(b: &[u8]) -> Self;
}
