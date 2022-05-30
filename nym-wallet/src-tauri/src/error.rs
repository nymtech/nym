use serde::{Serialize, Serializer};
use std::io;
use thiserror::Error;
use validator_client::validator_api::error::ValidatorAPIError;
use validator_client::{nymd::error::NymdError, ValidatorClientError};

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
    #[error("{source}")]
    MalformedUrlProvided {
        #[from]
        source: url::ParseError,
    },
    #[error("{source}")]
    ReqwestError {
        #[from]
        source: reqwest::Error,
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
    #[error("{0} is not a valid network denomination string")]
    InvalidNetworkDenom(String),
    #[error("The provided network is not supported (yet)")]
    NetworkNotSupported(config::defaults::all::Network),
    #[error("Could not access the local data storage directory")]
    UnknownStorageDirectory,
    #[error("No validator API URL configured")]
    NoValidatorApiUrlConfigured,
    #[error("The wallet file already exists")]
    WalletFileAlreadyExists,
    #[error("The wallet file is not found")]
    WalletFileNotFound,
    #[error("Login ID not found in wallet")]
    WalletNoSuchLoginId,
    #[error("Account ID not found in wallet login")]
    WalletNoSuchAccountIdInWalletLogin,
    #[error("Login ID already found in wallet")]
    WalletLoginIdAlreadyExists,
    #[error("Account ID already found in wallet login")]
    WalletAccountIdAlreadyExistsInWalletLogin,
    #[error("Mnemonic already found in wallet login, was it already imported?")]
    WalletMnemonicAlreadyExistsInWalletLogin,
    #[error("Adding a different password to the wallet not currently supported")]
    WalletDifferentPasswordDetected,
    #[error("Unexpted mnemonic account for login")]
    WalletUnexpectedMnemonicAccount,
    #[error("Unexpted multiple account entry for login")]
    WalletUnexpectedMultipleAccounts,
    #[error("Failed to derive address from mnemonic")]
    FailedToDeriveAddress,
}

impl Serialize for BackendError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl From<ValidatorClientError> for BackendError {
    fn from(e: ValidatorClientError) -> Self {
        match e {
            ValidatorClientError::ValidatorAPIError { source } => source.into(),
            ValidatorClientError::MalformedUrlProvided(e) => e.into(),
            ValidatorClientError::NymdError(e) => e.into(),
        }
    }
}
