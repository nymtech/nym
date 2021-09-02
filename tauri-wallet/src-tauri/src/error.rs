use thiserror::Error;
use validator_client::nymd::error::NymdError;

#[derive(Error, Debug)]
pub enum BackendError {
  #[error("Error parsing bip39 mnemonic")]
  Bip39Error {
    #[from]
    source: bip39::Error,
  },
  #[error("Error parsing into tendermint Url")]
  TendermintError {
    #[from]
    source: tendermint_rpc::Error,
  },
  #[error("Error getting balances")]
  NymdError {
    #[from]
    source: NymdError,
  },
}
