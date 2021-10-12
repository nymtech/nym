use num::rational::Ratio;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum MixnetContractError {
    #[error("Overflow Error")]
    OverflowError(#[from] cosmwasm_std::OverflowError),
    #[error("Could not convert ratio {0} to u128")]
    RatioToU128(Ratio<u128>),
    #[error("reward_blockstamp field not set, set_reward_blockstamp must be called before attempting to issue rewards")]
    BlockstampNotSet,
}
