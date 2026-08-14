// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::client::Client;
use crate::ecash::error::{EcashError, Result};
use crate::ecash::helpers::CachedImmutableEpochItem;
use async_trait::async_trait;
use nym_coconut_dkg_common::types::{Epoch, EpochId};
use nym_dkg::Threshold;
use nym_validator_client::EcashApiClient;
use std::cmp::min;
use time::OffsetDateTime;
use tokio::sync::{RwLock, RwLockWriteGuard};

#[async_trait]
pub trait APICommunicationChannel {
    async fn current_epoch(&self) -> Result<EpochId>;

    async fn ecash_clients(&self, epoch_id: EpochId) -> Result<Vec<EcashApiClient>>;

    async fn ecash_threshold(&self, epoch_id: EpochId) -> Result<Threshold>;

    async fn dkg_in_progress(&self) -> Result<bool>;
}

struct CachedEpoch {
    valid_until: OffsetDateTime,
    current_epoch: Epoch,
}

impl Default for CachedEpoch {
    fn default() -> Self {
        CachedEpoch {
            valid_until: OffsetDateTime::UNIX_EPOCH,
            current_epoch: Epoch::default(),
        }
    }
}

impl CachedEpoch {
    fn is_valid(&self) -> bool {
        self.valid_until > OffsetDateTime::now_utc()
    }

    fn update(&mut self, epoch: Epoch) -> Result<()> {
        let now = OffsetDateTime::now_utc();

        let validity_duration = if let Some(epoch_finish) = epoch.deadline {
            // SAFETY: values set in our contract are valid unix timestamps
            #[allow(clippy::unwrap_used)]
            let state_end =
                OffsetDateTime::from_unix_timestamp(epoch_finish.seconds() as i64).unwrap();
            let until_epoch_state_end = state_end - now;
            // make it valid until the next epoch transition or next 5min, whichever is smaller
            min(until_epoch_state_end, 5 * time::Duration::MINUTE)
        } else {
            5 * time::Duration::MINUTE
        };

        self.valid_until = now + validity_duration;
        self.current_epoch = epoch;

        Ok(())
    }
}

pub(crate) struct QueryCommunicationChannel {
    client: Box<dyn Client + Send + Sync>,

    epoch_clients: CachedImmutableEpochItem<Vec<EcashApiClient>>,
    cached_epoch: RwLock<CachedEpoch>,
    threshold_values: CachedImmutableEpochItem<Threshold>,
}

impl QueryCommunicationChannel {
    pub fn new<C>(client: C) -> Self
    where
        C: Client + Send + Sync + 'static,
    {
        QueryCommunicationChannel {
            client: Box::new(client),
            epoch_clients: Default::default(),
            cached_epoch: Default::default(),
            threshold_values: Default::default(),
        }
    }

    async fn update_epoch_cache(&self) -> Result<RwLockWriteGuard<'_, CachedEpoch>> {
        let mut guard = self.cached_epoch.write().await;

        let epoch = self.client.get_current_epoch().await?;

        guard.update(epoch)?;
        Ok(guard)
    }
}

#[async_trait]
impl APICommunicationChannel for QueryCommunicationChannel {
    async fn current_epoch(&self) -> Result<EpochId> {
        let guard = self.cached_epoch.read().await;
        if guard.is_valid() {
            return Ok(guard.current_epoch.epoch_id);
        }

        // update cache
        drop(guard);
        let guard = self.update_epoch_cache().await?;

        Ok(guard.current_epoch.epoch_id)
    }

    // TODO: perhaps this should be returning a ReadGuard instead?
    async fn ecash_clients(&self, epoch_id: EpochId) -> Result<Vec<EcashApiClient>> {
        self.epoch_clients
            .get_or_init(epoch_id, || async {
                self.client.get_registered_ecash_clients(epoch_id).await
            })
            .await
            .map(|guard| guard.clone())
    }

    async fn ecash_threshold(&self, epoch_id: EpochId) -> Result<Threshold> {
        self.threshold_values
            .get_or_init(epoch_id, || async {
                if let Some(threshold) = self.client.get_epoch_threshold(epoch_id).await? {
                    Ok(threshold)
                } else {
                    Err(EcashError::UnavailableThreshold { epoch_id })
                }
            })
            .await
            .map(|t| *t)
    }

    async fn dkg_in_progress(&self) -> Result<bool> {
        let guard = self.cached_epoch.read().await;
        if guard.is_valid() {
            return Ok(!guard.current_epoch.state.is_in_progress());
        }

        // update cache
        drop(guard);
        let guard = self.update_epoch_cache().await?;

        return Ok(!guard.current_epoch.state.is_in_progress());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecash::tests::contract_chain::{ContractChainClient, SharedContractChain};
    use crate::ecash::tests::contract_harness::{
        derive_keypairs, exchange_dealings, finalize_except, initialise_controllers, initiate_dkg,
        run_full_ceremony, submit_public_keys, validate_keys,
    };
    use nym_compact_ecash::aggregate_verification_keys;

    #[tokio::test]
    #[ignore] // expensive test
    async fn serves_signer_discovery_from_a_concluded_ceremony() -> anyhow::Result<()> {
        let validators = 3;

        let chain = SharedContractChain::new(validators);
        let mut controllers = initialise_controllers(&chain);
        initiate_dkg(&chain);
        let epoch_id = chain.epoch().epoch_id;

        run_full_ceremony(&mut controllers, false).await;

        let channel =
            QueryCommunicationChannel::new(ContractChainClient::new(chain.admin(), chain.clone()));

        assert_eq!(channel.current_epoch().await?, epoch_id);
        assert!(!channel.dkg_in_progress().await?);

        // every dealer that finished the ceremony is discoverable as a signer
        let clients = channel.ecash_clients(epoch_id).await?;
        assert_eq!(clients.len(), validators);

        // and the threshold is the contract's own ceil(2n/3)
        assert_eq!(channel.ecash_threshold(epoch_id).await?, 2);

        Ok(())
    }

    /// B7: one signer dropping out during the finalization window leaves its share
    /// unverified on chain. The epoch still concluded and the remaining shares still
    /// meet the threshold, so signer discovery must keep working for everyone else.
    ///
    /// Currently RED: the conversion rejects the whole epoch on the first unverified
    /// share, before any threshold is considered, so every gateway and client loses
    /// signer discovery for that epoch entirely.
    #[tokio::test]
    #[ignore] // expensive test
    async fn one_unverified_share_does_not_brick_the_epoch() -> anyhow::Result<()> {
        let validators = 3;

        let chain = SharedContractChain::new(validators);
        let mut controllers = initialise_controllers(&chain);
        initiate_dkg(&chain);
        let epoch_id = chain.epoch().epoch_id;

        submit_public_keys(&mut controllers, false).await;
        exchange_dealings(&mut controllers, false).await;
        derive_keypairs(&mut controllers, false).await;
        validate_keys(&mut controllers, false).await;

        // the first dealer never executes its own verification proposal
        finalize_except(&mut controllers, 0).await;

        // precondition: the contract really is in the state we are testing against -
        // the epoch concluded, with exactly one share left unverified
        let dropped = controllers[0].address().await;
        assert!(!chain.vk_share_verified(epoch_id, &dropped));
        for controller in controllers.iter().skip(1) {
            assert!(chain.vk_share_verified(epoch_id, &controller.address().await));
        }

        // ... and the survivors still meet the threshold
        let threshold = chain
            .epoch_threshold(epoch_id)
            .expect("no threshold was set");
        assert_eq!(threshold, 2);

        let channel =
            QueryCommunicationChannel::new(ContractChainClient::new(chain.admin(), chain.clone()));

        let clients = channel.ecash_clients(epoch_id).await?;
        assert_eq!(clients.len() as u64, threshold);

        // the surviving shares must still reconstruct the epoch's master key, otherwise
        // "discovery works" would be hollow
        let mut expected = Vec::new();
        let mut expected_indices = Vec::new();
        for controller in controllers.iter() {
            expected.push(controller.unchecked_coconut_vk().await);
            expected_indices.push(controller.state.assigned_index(epoch_id)?);
        }
        let expected_master = aggregate_verification_keys(&expected, Some(&expected_indices))?;

        let recovered = clients
            .iter()
            .map(|client| client.verification_key.clone())
            .collect::<Vec<_>>();
        let recovered_indices = clients
            .iter()
            .map(|client| client.node_id)
            .collect::<Vec<_>>();
        let recovered_master = aggregate_verification_keys(&recovered, Some(&recovered_indices))?;

        assert_eq!(expected_master, recovered_master);

        Ok(())
    }
}
