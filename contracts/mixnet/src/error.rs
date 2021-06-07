use crate::contract::DENOM;
use cosmwasm_std::{HumanAddr, StdError};
use thiserror::Error;

/// Custom errors for contract failure conditions.
///
/// Add any other custom errors you like here.
/// Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error(
        "Not enough funds sent for mixnode bond. (received {received:?}, minimum {minimum:?})"
    )]
    InsufficientMixNodeBond { received: u128, minimum: u128 },

    #[error("Account does not own any mixnode bonds")]
    MixNodeBondNotFound {},

    #[error(
        "Not enough funds sent for gateway bond. (received {received:?}, minimum {minimum:?})"
    )]
    InsufficientGatewayBond { received: u128, minimum: u128 },

    #[error("Account ({account:?}) does not own any gateway bonds")]
    GatewayBondNotFound { account: HumanAddr },

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Wrong coin denomination, you must send {}", DENOM)]
    WrongDenom {},

    #[error("Received multiple coin types during bond")]
    MultipleDenoms,

    #[error("No coin was sent for the bonding, you must send {}", DENOM)]
    NoBondFound,

    #[error("The bond reward rate for mixnode was set to be lower than 1")]
    DecreasingMixnodeBondReward,

    #[error("The bond reward rate for gateway was set to be lower than 1")]
    DecreasingGatewayBondReward,

    #[error("The node had uptime larger than 100%")]
    UnexpectedUptime,

    #[error("This address has already bonded a mixnode")]
    AlreadyOwnsMixnode,

    #[error("This address has already bonded a gateway")]
    AlreadyOwnsGateway,

    #[error("Mixnode with this identity already exists. Its owner is {owner:?}")]
    DuplicateMixnode { owner: HumanAddr },

    #[error("Gateway with this identity already exists. Its owner is {owner:?}")]
    DuplicateGateway { owner: HumanAddr },
}
