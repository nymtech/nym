// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::error::Result;
use crate::nyxd;
use crate::support::nyxd::ClientInner;
use nym_coconut_dkg_common::types::{Epoch, EpochId};
use nym_compact_ecash::VerificationKeyAuth;
use nym_credentials::obtain_aggregate_verification_key;
use nym_validator_client::coconut::all_ecash_api_clients;
use nym_validator_client::nyxd::contract_traits::DkgQueryClient;
use std::cmp::min;
use std::collections::HashMap;
use std::ops::Deref;
use time::OffsetDateTime;
use tokio::sync::RwLock;

#[async_trait]
pub trait APICommunicationChannel {
    async fn current_epoch(&self) -> Result<EpochId>;
    async fn aggregated_verification_key(&self, epoch_id: EpochId) -> Result<VerificationKeyAuth>;
}

struct CachedEpoch {
    valid_until: OffsetDateTime,
    current_epoch_id: EpochId,
}

impl Default for CachedEpoch {
    fn default() -> Self {
        CachedEpoch {
            valid_until: OffsetDateTime::UNIX_EPOCH,
            current_epoch_id: 0,
        }
    }
}

impl CachedEpoch {
    fn is_valid(&self) -> bool {
        self.valid_until > OffsetDateTime::now_utc()
    }

    async fn update(&mut self, epoch: Epoch) -> Result<()> {
        let now = OffsetDateTime::now_utc();

        let validity_duration = if let Some(epoch_finish) = epoch.deadline {
            let state_end =
                OffsetDateTime::from_unix_timestamp(epoch_finish.seconds() as i64).unwrap();
            let until_epoch_state_end = state_end - now;
            // make it valid until the next epoch transition or next 5min, whichever is smaller
            min(until_epoch_state_end, 5 * time::Duration::MINUTE)
        } else {
            5 * time::Duration::MINUTE
        };

        self.valid_until = now + validity_duration;
        self.current_epoch_id = epoch.epoch_id;

        Ok(())
    }
}

pub(crate) struct QueryCommunicationChannel {
    nyxd_client: nyxd::Client,

    epoch_keys: RwLock<HashMap<EpochId, VerificationKeyAuth>>,
    cached_epoch: RwLock<CachedEpoch>,
}

impl QueryCommunicationChannel {
    pub fn new(nyxd_client: nyxd::Client) -> Self {
        QueryCommunicationChannel {
            nyxd_client,
            epoch_keys: Default::default(),
            cached_epoch: Default::default(),
        }
    }
}

#[async_trait]
impl APICommunicationChannel for QueryCommunicationChannel {
    async fn current_epoch(&self) -> Result<EpochId> {
        let guard = self.cached_epoch.read().await;
        if guard.is_valid() {
            return Ok(guard.current_epoch_id);
        }

        // update cache
        drop(guard);
        let mut guard = self.cached_epoch.write().await;

        let epoch = match self.nyxd_client.read().await.deref() {
            ClientInner::Query(client) => client.get_current_epoch().await?,
            ClientInner::Signing(client) => client.get_current_epoch().await?,
        };

        guard.update(epoch).await?;

        return Ok(guard.current_epoch_id);
    }

    async fn aggregated_verification_key(&self, epoch_id: EpochId) -> Result<VerificationKeyAuth> {
        if let Some(vk) = self.epoch_keys.read().await.get(&epoch_id) {
            return Ok(vk.clone());
        }

        let mut guard = self.epoch_keys.write().await;
        let ecash_api_clients = match self.nyxd_client.read().await.deref() {
            ClientInner::Query(client) => all_ecash_api_clients(client, epoch_id).await?,
            ClientInner::Signing(client) => all_ecash_api_clients(client, epoch_id).await?,
        };

        let vk = obtain_aggregate_verification_key(&ecash_api_clients)?;

        guard.insert(epoch_id, vk.clone());

        Ok(vk)
    }
}
