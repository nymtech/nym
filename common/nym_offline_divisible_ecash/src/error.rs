use thiserror::Error;

pub type Result<T> = std::result::Result<T, DivisibleEcashError>;

#[derive(Error, Debug)]
pub enum DivisibleEcashError {
    #[error("Setup error: {0}")]
    Setup(String),

    #[error("Aggregation error: {0}")]
    Aggregation(String),

    #[error("Withdrawal Request Verification related error: {0}")]
    WithdrawalRequestVerification(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Interpolation error: {0}")]
    Interpolation(String),

    #[error("Issuance Verification related error: {0}")]
    IssuanceVfy(String),

    #[error("Spend Verification related error: {0}")]
    Spend(String),

    #[error("Identify Verification related error: {0}")]
    Identify(String),

    #[error("Tried to deserialize {object} with bytes of invalid length. Expected {actual} < {} or {modulus_target} % {modulus} == 0")]
    DeserializationInvalidLength {
        actual: usize,
        target: usize,
        modulus_target: usize,
        modulus: usize,
        object: String,
    },
}
