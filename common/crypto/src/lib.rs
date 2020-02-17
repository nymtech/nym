pub mod encryption;
pub mod identity;

// TODO: ideally those trait should be moved to 'pemstore' crate, however, that would cause
// circular dependency. The best solution would be to remove dependency on 'crypto' from
// pemstore by using either dynamic dispatch or generics - perhaps this should be done
// at some point during one of refactors.

pub trait PemStorableKey {
    fn pem_type(&self) -> String;
    fn to_bytes(&self) -> Vec<u8>;
}

pub trait PemStorableKeyPair {
    type PrivatePemKey: PemStorableKey;
    type PublicPemKey: PemStorableKey;

    fn private_key(&self) -> &Self::PrivatePemKey;
    fn public_key(&self) -> &Self::PublicPemKey;

    fn from_bytes(priv_bytes: &[u8], pub_bytes: &[u8]) -> Self;
}
