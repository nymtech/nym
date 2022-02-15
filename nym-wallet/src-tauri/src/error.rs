use serde::{Serialize, Serializer};
use std::io;
use thiserror::Error;
use validator_client::nymd::error::NymdError;
use validator_client::validator_api::error::ValidatorAPIError;

#[derive(Error, Debug)]
pub enum BackendError {
  #[error("{source}")]
  Bip39Error {
    #[from]
    source: bip39::Error,
  },
  #[error("{source}")]
  TendermintError {
    #[from]
    source: tendermint_rpc::Error,
  },
  #[error("{source}")]
  NymdError {
    #[from]
    source: NymdError,
  },
  #[error("{source}")]
  CosmwasmStd {
    #[from]
    source: cosmwasm_std::StdError,
  },
  #[error("{source}")]
  ErrorReport {
    #[from]
    source: eyre::Report,
  },
  #[error("{source}")]
  ValidatorApiError {
    #[from]
    source: ValidatorAPIError,
  },
  #[error("{source}")]
  KeyDerivationError {
    #[from]
    source: argon2::Error,
  },
  #[error("{source}")]
  IOError {
    #[from]
    source: io::Error,
  },
  #[error("{source}")]
  SerdeJsonError {
    #[from]
    source: serde_json::Error,
  },
  #[error("failed to encrypt the given data with the provided password")]
  EncryptionError,
  #[error("failed to decrypt the given data with the provided password")]
  DecryptionError,
  #[error("Client has not been initialized yet, connect with mnemonic to initialize")]
  ClientNotInitialized,
  #[error("No balance available for address {0}")]
  NoBalance(String),
  #[error("{0} is not a valid denomination string")]
  InvalidDenom(String),
  #[error("The provided network is not supported (yet)")]
  NetworkNotSupported(config::defaults::all::Network),
  #[error("Could not access the local data storage directory")]
  UnknownStorageDirectory,
}

impl Serialize for BackendError {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    serializer.collect_str(self)
  }
}
