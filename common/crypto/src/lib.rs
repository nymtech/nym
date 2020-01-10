pub mod encryption;
pub mod identity;

// TODO: this trait will need to be moved elsewhere, probably to some 'persistence' crate
// but since it will need to be used by all identities, it's not really appropriate if it lived in nym-client

pub trait PemStorable {
    fn pem_type(&self) -> String;
}
