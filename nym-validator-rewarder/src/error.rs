// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::tx::ErrorReport;
use std::io;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NymRewarderError {
    #[error(
    "failed to load config file using path '{}'. detailed message: {source}", path.display()
    )]
    ConfigLoadFailure {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error(
    "failed to save config file using path '{}'. detailed message: {source}", path.display()
    )]
    ConfigSaveFailure {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to initialise paths")]
    PathInitialisationFailure {
        #[source]
        source: io::Error,
    },

    #[error("there already exists a config file at: {}. if you want to overwrite its content, use --force flag", path.display())]
    ExistingConfig { path: PathBuf },

    // TODO: I think this one should get split into more, explicit, variants
    #[error(transparent)]
    NyxdFailure(#[from] NyxdError),

    #[error("the provided rewarding ratios don't add up to 1. ratios: {ratios:?}")]
    InvalidRewardingRatios { ratios: Vec<f32> },

    #[error("chain scraping failure: {source}")]
    ScraperFailure {
        #[from]
        source: nyxd_scraper::error::ScraperError,
    },

    // this should never happen but unwrapping everywhere was more cumbersome than just propagating the error
    #[error("failed to determine epoch boundaries: {0}")]
    TimeComponentFailure(#[from] time::error::ComponentRange),

    #[error("could not convert validators conesnsus address: {consensus_address} to a nym account address: {source}")]
    MalformedBech32Address {
        consensus_address: String,
        #[source]
        source: ErrorReport,
    },
}
