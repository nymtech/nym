pub mod encryption;
pub mod identity;

// TODO: this trait will need to be moved elsewhere, probably to some 'persistence' crate
// but since it will need to be used by all identities, it's not really appropriate if it lived in nym-client

pub trait PemStorableKey {
    fn pem_type(&self) -> String;
}

pub trait PemStorableKeyPair {
    type PrivatePemKey: PemStorableKey;
    type PublicPemKey: PemStorableKey;

    fn private_pem_key(&self) -> &Self::PrivatePemKey;
    fn public_pem_key(&self) -> &Self::PublicPemKey;
}
