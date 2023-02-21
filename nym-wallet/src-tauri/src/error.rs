use nym_types::error::TypesError;
use nym_wallet_types::network::Network;
use serde::{Serialize, Serializer};
use std::io;
use std::num::ParseIntError;
use thiserror::Error;
use validator_client::nym_api::error::NymAPIError;
use validator_client::{nyxd::error::NyxdError, ValidatorClientError};

#[derive(Error, Debug)]
pub enum BackendError {
    #[error("{source}")]
    TypesError {
        #[from]
        source: TypesError,
    },
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
    #[error("{pretty_error}")]
    NyxdError {
        pretty_error: String,
        #[source]
        source: NyxdError,
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
    NymApiError {
        #[from]
        source: NymAPIError,
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
    #[error("{source}")]
    K256Error {
        #[from]
        source: k256::ecdsa::Error,
    },
    #[error("failed to encrypt the given data with the provided password")]
    EncryptionError,
    #[error("failed to decrypt the given data with the provided password")]
    DecryptionError,
    #[error("Client has not been initialized yet, connect with mnemonic to initialize")]
    ClientNotInitialized,
    #[error("No balance available for address {0}")]
    NoBalance(String),
    #[error("The provided network is not supported (yet)")]
    NetworkNotSupported,
    #[error("Could not access the local data storage directory")]
    UnknownStorageDirectory,
    #[error("The wallet file already exists")]
    WalletFileAlreadyExists,
    #[error("The wallet file is not found")]
    WalletFileNotFound,
    #[error("The wallet file has a malformed name")]
    WalletFileMalformedFilename,
    #[error("Unable to archive wallet file")]
    WalletFileUnableToArchive,
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
    #[error("Unexpected mnemonic account for login")]
    WalletUnexpectedMnemonicAccount,
    #[error("Failed to derive address from mnemonic")]
    FailedToDeriveAddress,
    #[error(transparent)]
    ValueParseError(#[from] ParseIntError),
    #[error("The provided coin has an unknown denomination - {0}")]
    UnknownCoinDenom(String),
    #[error("Network {network} doesn't have any associated registered coin denoms")]
    NoCoinsRegistered { network: Network },
    #[error("Signature error {0}")]
    SignatureError(String),
    #[error("Unable to open a new window")]
    NewWindowError,

    #[error("This command ({name}) has been removed. Please try to use {alternative} instead.")]
    RemovedCommand { name: String, alternative: String },
}

impl Serialize for BackendError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl From<NyxdError> for BackendError {
    fn from(source: NyxdError) -> Self {
        match source {
            NyxdError::AbciError {
                code: _,
                log: _,
                ref pretty_log,
            } => {
                if let Some(pretty_log) = pretty_log {
                    Self::NyxdError {
                        pretty_error: pretty_log.to_string(),
                        source,
                    }
                } else {
                    Self::NyxdError {
                        pretty_error: source.to_string(),
                        source,
                    }
                }
            }
            nyxd_error => Self::NyxdError {
                pretty_error: nyxd_error.to_string(),
                source: nyxd_error,
            },
        }
    }
}

impl From<ValidatorClientError> for BackendError {
    fn from(e: ValidatorClientError) -> Self {
        match e {
            ValidatorClientError::NymAPIError { source } => source.into(),
            ValidatorClientError::MalformedUrlProvided(e) => e.into(),
            ValidatorClientError::NyxdError(e) => e.into(),
            ValidatorClientError::NoAPIUrlAvailable => TypesError::NoNymApiUrlConfigured.into(),
        }
    }
}
