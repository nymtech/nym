// Copyright 2025 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::nym_api_helpers::{CachedEpoch, CachedImmutableEpochItem, CachedImmutableItems};
use crate::quorum_checker::QuorumState;
use crate::shared_state::required_deposit_cache::RequiredDepositCache;
use nym_compact_ecash::VerificationKeyAuth;
use nym_credentials::{AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures};
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::EcashApiClient;
use time::Date;
use tokio::sync::RwLock;

pub struct EcashState {
    pub required_deposit_cache: RequiredDepositCache,

    pub quorum_state: QuorumState,

    pub cached_epoch: RwLock<CachedEpoch>,

    pub master_verification_key: CachedImmutableEpochItem<VerificationKeyAuth>,

    pub threshold_values: CachedImmutableEpochItem<u64>,

    pub epoch_clients: CachedImmutableEpochItem<Vec<EcashApiClient>>,

    pub coin_index_signatures: CachedImmutableEpochItem<AggregatedCoinIndicesSignatures>,

    pub expiration_date_signatures:
        CachedImmutableItems<(EpochId, Date), AggregatedExpirationDateSignatures>,
}

impl EcashState {
    pub fn new(
        required_deposit_cache: RequiredDepositCache,
        quorum_state: QuorumState,
    ) -> EcashState {
        EcashState {
            required_deposit_cache,
            quorum_state,
            cached_epoch: Default::default(),
            master_verification_key: Default::default(),
            threshold_values: Default::default(),
            epoch_clients: Default::default(),
            coin_index_signatures: Default::default(),
            expiration_date_signatures: Default::default(),
        }
    }
}
