use nym_contracts_common::signing::SigningAlgorithm;
use nym_crypto::asymmetric::ed25519::Ed25519RecoveryError;
use nym_node_requests::api::client::NymNodeApiClientError;
use nym_types::error::TypesError;
use nym_validator_client::signing::direct_wallet::DirectSecp256k1HdWalletError;
use nym_validator_client::{nyxd::error::NyxdError, ValidatorClientError};
use nym_wallet_types::network::Network;
use serde::{Serialize, Serializer};
use std::io;
use std::num::ParseIntError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BackendError {
    #[error(transparent)]
    TypesError {
        source: TypesError,
    },
    #[error(transparent)]
    Bip39Error {
        #[from]
        source: bip39::Error,
    },
    #[error(transparent)]
    TendermintError {
        #[from]
        source: cosmrs::rpc::Error,
    },
    #[error("{pretty_error}")]
    NyxdError {
        pretty_error: String,
        #[source]
        source: Box<NyxdError>,
    },
    #[error(transparent)]
    CosmwasmStd {
        #[from]
        source: cosmwasm_std::StdError,
    },
    #[error(transparent)]
    ErrorReport {
        #[from]
        source: eyre::Report,
    },
    #[error(transparent)]
    NymNodeApiError { source: Box<NymNodeApiClientError> },
    #[error(transparent)]
    IOError {
        #[from]
        source: io::Error,
    },
    #[error(transparent)]
    SerdeJsonError {
        #[from]
        source: serde_json::Error,
    },
    #[error(transparent)]
    MalformedUrlProvided {
        #[from]
        source: url::ParseError,
    },
    #[error(transparent)]
    ReqwestError {
        #[from]
        source: reqwest::Error,
    },
    #[error(transparent)]
    K256Error {
        #[from]
        source: k256::ecdsa::Error,
    },

    #[error(transparent)]
    StoreCipherError {
        #[from]
        source: nym_store_cipher::Error,
    },

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
    #[error("Invalid update pledge request, the new bond amount is the same as the current one")]
    WalletPledgeUpdateNoOp,
    #[error(
        "Invalid update pledge request, the new bond is a different currency from the current one"
    )]
    WalletPledgeUpdateInvalidCurrency,
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
    #[error("Built-in HD derivation path constant failed to parse (internal error)")]
    InvalidInternalDerivationPath,
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
    #[error("Failed to check for application update")]
    CheckAppVersionError,
    #[error("Failed to connect to the provided validator URL")]
    WalletValidatorConnectionFailed,
    #[error("No defined default validator URL")]
    WalletNoDefaultValidator,
    #[error(
        "this vesting operation has been disabled. please use the non-vesting variant instead."
    )]
    UnsupportedVestingOperation,

    #[error(transparent)]
    WalletError {
        #[from]
        source: DirectSecp256k1HdWalletError,
    },

    #[error("received unexpected signing algorithm: {received:?}. Expected to get {expected:?}")]
    UnexpectedSigningAlgorithm {
        received: SigningAlgorithm,
        expected: SigningAlgorithm,
    },
    #[error(transparent)]
    Ed25519Recovery(#[from] Ed25519RecoveryError),

    #[error("This command ({name}) has been removed. Please try to use {alternative} instead.")]
    RemovedCommand { name: String, alternative: String },

    #[error("there aren't any vesting delegations to migrate")]
    NoVestingDelegations,

    /// Vesting contract [`nym_vesting_contract_common::VestingContractError::NoAccountForAddress`].
    #[error("Vesting contract has no account for this address")]
    VestingContractAccountNotFound,

    #[error("this command has been temporarily disabled")]
    Disabled,
    //
    // #[error("this operation is no longer allowed to be performed with vesting tokens. please move them to your liquid balance and try again")]
    // DisabledVestingOperation,
}

impl Serialize for BackendError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

/// Cosmwasm returns vesting [`nym_vesting_contract_common::VestingContractError::NoAccountForAddress`]
/// as `VESTING (...): Account does not exist - ...` in the ABCI log.
fn nyxd_error_is_vesting_contract_no_account(err: &NyxdError) -> bool {
    fn text_matches_strict(text: &str) -> bool {
        text.contains("VESTING") && text.contains("Account does not exist")
    }
    // Prefer strict match; fall back for ABCI text that omits the `VESTING` prefix. Exclude Nyxd
    // `NonExistentAccountError` (`... does not exist on the chain`).
    fn abci_query_vesting_no_account(text: &str) -> bool {
        text_matches_strict(text)
            || (text.contains("Account does not exist")
                && !text.contains("does not exist on the chain"))
    }
    match err {
        NyxdError::AbciError {
            log,
            pretty_log,
            ..
        } => {
            pretty_log
                .as_ref()
                .is_some_and(|s| abci_query_vesting_no_account(s))
                || abci_query_vesting_no_account(log)
        }
        _ => text_matches_strict(&err.to_string()),
    }
}

impl From<TypesError> for BackendError {
    fn from(e: TypesError) -> Self {
        if let TypesError::NyxdError { ref source } = e {
            if nyxd_error_is_vesting_contract_no_account(source) {
                return Self::VestingContractAccountNotFound;
            }
        }
        Self::TypesError { source: e }
    }
}

impl From<NyxdError> for BackendError {
    fn from(source: NyxdError) -> Self {
        if nyxd_error_is_vesting_contract_no_account(&source) {
            return Self::VestingContractAccountNotFound;
        }
        match source {
            NyxdError::AbciError {
                code: _,
                log: _,
                ref pretty_log,
            } => {
                if let Some(pretty_log) = pretty_log {
                    Self::NyxdError {
                        pretty_error: pretty_log.to_string(),
                        source: Box::new(source),
                    }
                } else {
                    Self::NyxdError {
                        pretty_error: source.to_string(),
                        source: Box::new(source),
                    }
                }
            }
            nyxd_error => Self::NyxdError {
                pretty_error: nyxd_error.to_string(),
                source: Box::new(nyxd_error),
            },
        }
    }
}

impl From<ValidatorClientError> for BackendError {
    fn from(e: ValidatorClientError) -> Self {
        TypesError::from(e).into()
    }
}

impl From<NymNodeApiClientError> for BackendError {
    fn from(e: NymNodeApiClientError) -> Self {
        BackendError::NymNodeApiError {
            source: Box::new(e),
        }
    }
}
