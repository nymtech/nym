// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContextError {
    #[error("mnemonic was not provided, pass as an argument or an env var called MNEMONIC")]
    MnemonicNotProvided,

    #[error("failed to parse mnemonic - {0}")]
    Bip39Error(#[from] bip39::Error),

    // there are lots of error that can occur in the nyxd client, so just pass through their display details
    // TODO: improve this to return known errors
    #[error("failed to create client - {0}")]
    NyxdError(String),

    #[error(transparent)]
    NyxdErrorPassthrough(#[from] nym_validator_client::nyxd::error::NyxdError),

    #[error(transparent)]
    ValidatorClientError(#[from] nym_validator_client::ValidatorClientError),
}
