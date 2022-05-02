use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum MixnetContractError {
    #[error("Overflow Error")]
    OverflowError(#[from] cosmwasm_std::OverflowError),
    #[error("reward_blockstamp field not set, set_reward_blockstamp must be called before attempting to issue rewards")]
    BlockstampNotSet,
    #[error("{source}")]
    TryFromIntError {
        #[from]
        source: std::num::TryFromIntError,
    },
    #[error("Error casting from U128")]
    CastError,
    #[error("{source}")]
    StdErr {
        #[from]
        source: cosmwasm_std::StdError,
    },
    #[error("Division by zero at {}", line!())]
    DivisionByZero,
}
