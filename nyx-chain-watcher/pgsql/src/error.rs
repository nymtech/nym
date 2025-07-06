use std::io;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NyxChainWatcherError {
    // #[error("failed to save config file using path '{}'. detailed message: {source}", path.display())]
    // ConfigSaveFailure {
    //     path: PathBuf,
    //     #[source]
    //     source: io::Error,
    // },
    #[error("failed to save config file using path '{}'. detailed message: {source}", path.display())]
    UnformattedConfigSaveFailure {
        path: PathBuf,
        #[source]
        source: nym_config::error::NymConfigTomlError,
    },

    #[error("could not derive path to data directory of this nyx chain watcher")]
    DataDirDerivationFailure,

    // #[error("could not derive path to config directory of this nyx chain watcher")]
    // ConfigDirDerivationFailure,
    #[error("failed to load config file using path '{}'. detailed message: {source}", path.display())]
    ConfigLoadFailure {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(transparent)]
    FileIoFailure(#[from] io::Error),

    #[error(transparent)]
    AnyhowFailure(#[from] anyhow::Error),

    #[error(transparent)]
    NymConfigTomlE(#[from] nym_config::error::NymConfigTomlError),
}
