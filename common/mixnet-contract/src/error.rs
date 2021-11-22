use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum MixnetContractError {
    #[error("Overflow Error")]
    OverflowError(#[from] cosmwasm_std::OverflowError),
    #[error("reward_blockstamp field not set, set_reward_blockstamp must be called before attempting to issue rewards")]
    BlockstampNotSet,
}
