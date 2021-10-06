use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum MixnetContractError {
    #[error("Could not convert ration to f64: {0} / {1}")]
    InvalidRatio(u128, u128),
    #[error("reward_blockstamp field not set, set_reward_blockstamp must be called before attempting to issue rewards")]
    BlockstampNotSet,
}
