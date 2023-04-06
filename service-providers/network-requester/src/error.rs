use nym_client_core::error::ClientCoreError;
use nym_socks5_requests::Socks5RequestError;

#[derive(thiserror::Error, Debug)]
pub enum NetworkRequesterError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("client-core error: {0}")]
    ClientCoreError(#[from] ClientCoreError),

    #[error("encountered an error while trying to handle a provider request: {source}")]
    ProviderRequestError {
        #[from]
        source: Socks5RequestError,
    },

    #[error("failed to setup gateway: {source}")]
    FailedToSetupGateway { source: ClientCoreError },

    #[error("failed to load configuration file: {0}")]
    FailedToLoadConfig(String),

    #[error("failed local version check, client and config mismatch")]
    FailedLocalVersionCheck,

    #[error("failed to setup mixnet client: {source}")]
    FailedToSetupMixnetClient { source: nym_sdk::Error },

    #[error("failed to connect to mixnet: {source}")]
    FailedToConnectToMixnet { source: nym_sdk::Error },
}
