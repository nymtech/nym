// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::block_processor::pruning::{
    EVERYTHING_PRUNING_INTERVAL, EVERYTHING_PRUNING_KEEP_RECENT,
};
use tendermint::Hash;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

#[derive(Debug, Error)]
pub enum ScraperError {
    #[error("experienced internal database error: {0}")]
    InternalDatabaseError(#[from] sqlx::Error),

    #[error("failed to perform startup SQL migration: {0}")]
    StartupMigrationFailure(#[from] sqlx::migrate::MigrateError),

    #[error("can't add any modules to the scraper as it's already running")]
    ScraperAlreadyRunning,

    #[error("failed to establish websocket connection to {url}: {source}")]
    WebSocketConnectionFailure {
        url: String,
        #[source]
        source: tendermint_rpc::Error,
    },

    #[error("failed to establish rpc connection to {url}: {source}")]
    HttpConnectionFailure {
        url: String,
        #[source]
        source: tendermint_rpc::Error,
    },

    #[error("failed to create chain subscription: {source}")]
    ChainSubscriptionFailure {
        #[source]
        source: tendermint_rpc::Error,
    },

    #[error("could not obtain basic block information at height: {height}: {source}")]
    BlockQueryFailure {
        height: u32,
        #[source]
        source: tendermint_rpc::Error,
    },

    #[error("could not obtain block results information at height: {height}: {source}")]
    BlockResultsQueryFailure {
        height: u32,
        #[source]
        source: tendermint_rpc::Error,
    },

    #[error("could not obtain validators information at height: {height}: {source}")]
    ValidatorsQueryFailure {
        height: u32,
        #[source]
        source: tendermint_rpc::Error,
    },

    #[error("could not obtain tx results for tx: {hash}: {source}")]
    TxResultsQueryFailure {
        hash: Hash,
        #[source]
        source: tendermint_rpc::Error,
    },

    #[error("could not obtain current abci info: {source}")]
    AbciInfoQueryFailure {
        #[source]
        source: tendermint_rpc::Error,
    },

    #[error("could not parse tx {hash}: {source}")]
    TxParseFailure {
        hash: Hash,
        #[source]
        source: cosmrs::ErrorReport,
    },

    #[error("received an invalid chain subscription event of kind {kind} while we were waiting for new block data (query: '{query}')")]
    InvalidSubscriptionEvent { query: String, kind: String },

    #[error("received block data was empty (query: '{query}')")]
    EmptyBlockData { query: String },

    #[error("reached maximum number of allowed errors for subscription events")]
    MaximumWebSocketFailures,

    #[error("failed to begin storage tx: {source}")]
    StorageTxBeginFailure {
        #[source]
        source: sqlx::Error,
    },

    #[error("failed to commit storage tx: {source}")]
    StorageTxCommitFailure {
        #[source]
        source: sqlx::Error,
    },

    #[error("failed to send on a closed channel")]
    ClosedChannelError,

    #[error("failed to parse validator's address: {source}")]
    MalformedValidatorAddress {
        #[source]
        source: eyre::Report,
    },

    #[error("failed to parse validator's address: {source}")]
    MalformedValidatorPubkey {
        #[source]
        source: eyre::Report,
    },

    #[error(
        "could not find the block proposer ('{proposer}') for height {height} in the validator set"
    )]
    BlockProposerNotInValidatorSet { height: u32, proposer: String },

    #[error(
        "could not find validator information for {address}; the validator has signed a commit"
    )]
    MissingValidatorInfoCommitted { address: String },

    #[error("pruning.interval must not be set to 0. If you want to disable pruning, select pruning.strategy = \"nothing\"")]
    ZeroPruningInterval,

    #[error("pruning.interval must not be smaller than {}. got: {interval}. for most aggressive pruning, select pruning.strategy = \"everything\"", EVERYTHING_PRUNING_INTERVAL)]
    TooSmallPruningInterval { interval: u32 },

    #[error("pruning.keep_recent must not be smaller than {}. got: {keep_recent}. for most aggressive pruning, select pruning.strategy = \"everything\"", EVERYTHING_PRUNING_KEEP_RECENT)]
    TooSmallKeepRecent { keep_recent: usize },
}

impl<T> From<SendError<T>> for ScraperError {
    fn from(_: SendError<T>) -> Self {
        ScraperError::ClosedChannelError
    }
}
