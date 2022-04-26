// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dkg::networking::message::InvalidDkgMessageType;
use std::io;
use thiserror::Error;
use validator_client::ValidatorClientError;

#[derive(Error, Debug)]
pub enum DkgError {
    #[error("Internal error - {0}")]
    Internal(#[from] ::dkg::error::DkgError),

    #[error("{0}")]
    ContractClient(#[from] ValidatorClientError),

    #[error("Networking error - {0}")]
    Networking(#[from] io::Error),

    #[error("todo")]
    DeserializationError,
}

impl From<InvalidDkgMessageType> for DkgError {
    fn from(err: InvalidDkgMessageType) -> Self {
        todo!("figure out how it would fit in the DeserializationError")
    }
}
