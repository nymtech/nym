// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Errors {
    #[error("account id does not match")]
    AccountIdError,

    #[error("signature error - {0}")]
    SignatureError(#[from] k256::ecdsa::signature::Error),

    #[error("{0}")]
    CosmrsError(#[from] cosmrs::ErrorReport),
}
