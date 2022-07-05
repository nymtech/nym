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

    #[error("Attempted to subtract decimals with overflow ({minuend}.sub({subtrahend}))")]
    OverflowDecimalSubtraction {
        minuend: Decimal,
        subtrahend: Decimal,
    },

    #[error("Attempted to subtract with overflow ({minuend}.sub({subtrahend}))")]
    OverflowSubtraction { minuend: u64, subtrahend: u64 },

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

    #[error("No tokens were sent for the bonding")]
    NoBondFound,

    #[error("Wrong coin denomination. Received: {received}, expected: {expected}")]
    WrongDenom { received: String, expected: String },

    #[error("Received multiple coin types during staking")]
    MultipleDenoms,

    #[error("Proxy address mismatch, expected {existing}, got {incoming}")]
    ProxyMismatch { existing: String, incoming: String },

    #[error("Failed to recover ed25519 public key from its base58 representation - {0}")]
    MalformedEd25519IdentityKey(String),

    #[error("Failed to recover ed25519 signature from its base58 representation - {0}")]
    MalformedEd25519Signature(String),

    #[error("Provided ed25519 signature did not verify correctly")]
    InvalidEd25519Signature,

    #[error("Can't advance the epoch as the current one is still in progress. It started at {epoch_start} and finishes at {epoch_end}, while the current block time is {current_block_time}")]
    EpochInProgress {
        current_block_time: u64,
        epoch_start: i64,
        epoch_end: i64,
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
    // #[error("Division by zero at {}")]
    // DivisionByZero,
}
