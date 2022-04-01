use crate::error::DivisibleEcashError;

pub trait Bytable
    where
        Self: Sized,
{
    fn to_byte_vec(&self) -> Vec<u8>;

    fn try_from_byte_slice(slice: &[u8]) -> Result<Self, DivisibleEcashError>;
}

pub trait Base58
    where
        Self: Bytable,
{
    fn try_from_bs58<S: AsRef<str>>(x: S) -> Result<Self, DivisibleEcashError> {
        Self::try_from_byte_slice(&bs58::decode(x.as_ref()).into_vec().unwrap())
    }
    fn to_bs58(&self) -> String {
        bs58::encode(self.to_byte_vec()).into_string()
    }
}
