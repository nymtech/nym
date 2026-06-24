// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use nym_ecash_time::OffsetDateTime;
use nym_validator_client::coconut::{all_ecash_api_clients, EcashApiError};
use nym_validator_client::nym_api::EpochId;
use nym_validator_client::nyxd::contract_traits::DkgQueryClient;
use nym_validator_client::EcashApiClient;

pub use nyxd_public_data::NyxdGlobalDataFetcher;

mod nyxd_public_data;

struct EcashApiClientsCache {
    epoch_id: EpochId,
    clients: Vec<EcashApiClient>,
    last_updated_at: OffsetDateTime,
}

impl EcashApiClientsCache {
    const VALIDITY_DURATION: Duration = Duration::from_secs(30 * 60); // 30 minutes

    fn is_stale(&self) -> bool {
        self.last_updated_at + Self::VALIDITY_DURATION < OffsetDateTime::now_utc()
    }

    async fn from_dkg_client<C>(query_client: &C, epoch_id: EpochId) -> Result<Self, EcashApiError>
    where
        C: DkgQueryClient + Send + Sync,
    {
        let clients = all_ecash_api_clients(query_client, epoch_id).await?;
        Ok(EcashApiClientsCache {
            epoch_id,
            clients,
            last_updated_at: OffsetDateTime::now_utc(),
        })
    }
}
