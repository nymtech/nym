use thiserror::Error;

pub type Result<T> = std::result::Result<T, CompactEcashError>;

#[derive(Error, Debug)]
pub enum CompactEcashError {
    #[error("Setup error: {0}")]
    Setup(String),

    #[error("Withdrawal Request Verification related error: {0}")]
    WithdrawalRequestVerification(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Interpolation error: {0}")]
    Interpolation(String),

    #[error("Tried to deserialize {object} with bytes of invalid length. Expected {actual} < {} or {modulus_target} % {modulus} == 0")]
    DeserializationInvalidLength {
        actual: usize,
        target: usize,
        modulus_target: usize,
        modulus: usize,
        object: String,
    },
}
