// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_crypto::asymmetric::ed25519::Ed25519RecoveryError;
use nym_validator_client::nyxd::error::NyxdError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SignerCheckError {
    #[error("failed to connect to nyxd chain due to invalid connection details: {source}")]
    InvalidNyxdConnectionDetails { source: NyxdError },

    #[error("failed to query the DKG contract: {source}")]
    DKGContractQueryFailure { source: NyxdError },

    #[error("dealer at {dealer_url} has provided invalid ed25519 pubkey: {source}")]
    InvalidDealerPubkey {
        dealer_url: String,
        source: Ed25519RecoveryError,
    },

    #[error("dealer at {dealer_url} has provided invalid announce url: {source}")]
    InvalidDealerAddress {
        dealer_url: String,
        source: url::ParseError,
    },
}

impl SignerCheckError {
    pub fn invalid_nyxd_connection_details(source: NyxdError) -> Self {
        Self::InvalidNyxdConnectionDetails { source }
    }

    pub fn dkg_contract_query_failure(source: NyxdError) -> Self {
        Self::DKGContractQueryFailure { source }
    }
}
