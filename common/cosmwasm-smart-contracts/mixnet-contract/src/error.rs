use crate::IdentityKey;
use cosmwasm_std::{Addr, Coin, Decimal};
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

    #[error("Not enough funds sent for node pledge. (received {received}, minimum {minimum})")]
    InsufficientPledge { received: Coin, minimum: Coin },

    #[error("Mixnode ({identity}) does not exist")]
    MixNodeBondNotFound { identity: IdentityKey },

    #[error("{owner} does not seem to own any mixnodes")]
    NoAssociatedMixNodeBond { owner: Addr },

    #[error("{owner} does not seem to own any gateways")]
    NoAssociatedGatewayBond { owner: Addr },

    #[error("This address has already bonded a mixnode")]
    AlreadyOwnsMixnode,

    #[error("This address has already bonded a gateway")]
    AlreadyOwnsGateway,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Wrong coin denomination. Received: {received}, expected: {expected}")]
    WrongDenom { received: String, expected: String },

    #[error("Received multiple coin types during staking")]
    MultipleDenoms,
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
    // #[error("Division by zero at {}")]
    // DivisionByZero,
}
