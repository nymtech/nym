// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_credentials::ecash::bandwidth::serialiser::keys::EpochVerificationKey;
use nym_credentials::ecash::bandwidth::serialiser::signatures::{
    AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures,
};
use nym_ecash_time::Date;
use nym_validator_client::nym_api::EpochId;

use crate::traits::{CredentialFetcherError, CredentialPublicDataFetcher};

/// A piece of global ecash signing data to fetch. Doubles as the de-dup key for an in-flight
/// global-data fetch (hence `Hash`/`Eq`).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum GlobalDataRequest {
    MasterVerificationKey(EpochId),
    CoinIndexSignatures(EpochId),
    ExpirationDateSignatures {
        epoch_id: EpochId,
        expiration_date: Date,
    },
}

impl GlobalDataRequest {
    /// The pieces of global data needed to spend a ticketbook of the given epoch and expiration.
    pub(crate) fn for_ticketbook(
        epoch_id: EpochId,
        expiration_date: Date,
    ) -> [GlobalDataRequest; 3] {
        [
            GlobalDataRequest::MasterVerificationKey(epoch_id),
            GlobalDataRequest::CoinIndexSignatures(epoch_id),
            GlobalDataRequest::ExpirationDateSignatures {
                epoch_id,
                expiration_date,
            },
        ]
    }

    /// Fetches the requested piece via the public-data fetcher, tagging the result with its variant
    /// so the caller can persist it without having to remember what was asked for.
    pub(crate) async fn fetch(
        &self,
        fetcher: &dyn CredentialPublicDataFetcher,
    ) -> Result<GlobalData, CredentialFetcherError> {
        match *self {
            GlobalDataRequest::MasterVerificationKey(epoch_id) => fetcher
                .fetch_master_verification_key(epoch_id)
                .await
                .map(Box::new)
                .map(GlobalData::MasterVerificationKey),
            GlobalDataRequest::CoinIndexSignatures(epoch_id) => fetcher
                .fetch_coin_index_signatures(epoch_id)
                .await
                .map(GlobalData::CoinIndexSignatures),
            GlobalDataRequest::ExpirationDateSignatures {
                epoch_id,
                expiration_date,
            } => fetcher
                .fetch_expiration_date_signatures(expiration_date, epoch_id)
                .await
                .map(GlobalData::ExpirationDateSignatures),
        }
    }
}

/// Global ecash signing data returned by a completed [`GlobalDataRequest`] fetch, ready to persist.
#[derive(Clone)]
pub(crate) enum GlobalData {
    MasterVerificationKey(Box<EpochVerificationKey>),
    CoinIndexSignatures(AggregatedCoinIndicesSignatures),
    ExpirationDateSignatures(AggregatedExpirationDateSignatures),
}
