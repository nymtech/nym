use cosmwasm_std::Decimal;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum MixnetContractError {
    #[error("{source}")]
    StdErr {
        #[from]
        source: cosmwasm_std::StdError,
    },

    #[error("Provided percent value is greater than 100%")]
    InvalidPercent,

    #[error("Attempted to subtract with overflow ({minuend}.sub({subtrahend}))")]
    OverflowSubtraction {
        minuend: Decimal,
        subtrahend: Decimal,
    },
    //
    // #[error("Overflow Error")]
    // OverflowError(#[from] cosmwasm_std::OverflowError),
    // #[error("reward_blockstamp field not set, set_reward_blockstamp must be called before attempting to issue rewards")]
    // BlockstampNotSet,
    // #[error("{source}")]
    // TryFromIntError {
    //     #[from]
    //     source: std::num::TryFromIntError,
    // },
    // #[error("Error casting from U128")]
    // CastError,
    // #[error("{source}")]
    // StdErr {
    //     #[from]
    //     source: cosmwasm_std::StdError,
    // },
    // #[error("Division by zero at {}", line!())]
    // DivisionByZero,
}
