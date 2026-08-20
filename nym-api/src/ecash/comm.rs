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

    /// Whether this epoch's ceremony has finished, making its signer set final.
    ///
    /// Anything derived from the signer set may only be cached once this is true: until
    /// then the set is still being filled in, and whatever partial view a caller happens
    /// to observe would be pinned for the lifetime of the process.
    async fn ceremony_concluded(&self, epoch_id: EpochId) -> Result<bool>;
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

    /// The current epoch, refreshing the cache if it has gone stale.
    ///
    /// The cached copy expires at the epoch's own state deadline (see
    /// [`CachedEpoch::update`]), so it can never claim a ceremony has finished when it
    /// has not - at worst it is briefly pessimistic, which only costs an extra query.
    async fn current_epoch_data(&self) -> Result<Epoch> {
        let guard = self.cached_epoch.read().await;
        if guard.is_valid() {
            return Ok(guard.current_epoch);
        }

        drop(guard);
        let guard = self.update_epoch_cache().await?;
        Ok(guard.current_epoch)
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
        // gateways poll continuously, so during a ceremony something will ask about the
        // new epoch while its shares are still being submitted. answer, but don't cache:
        // the entry has no expiry, so an empty or partial set would stick for good.
        if !self.ceremony_concluded(epoch_id).await? {
            return self.client.get_registered_ecash_clients(epoch_id).await;
        }

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

    async fn ceremony_concluded(&self, epoch_id: EpochId) -> Result<bool> {
        Ok(self
            .current_epoch_data()
            .await?
            .is_ceremony_concluded(epoch_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecash::tests::contract_chain::{ContractChainClient, SharedContractChain};
    use crate::ecash::tests::contract_harness::{
        cheap, derive_keypairs, exchange_dealings, finalize_except, initialise_controllers,
        initiate_dkg, submit_public_keys, validate_keys,
    };
    use nym_compact_ecash::aggregate_verification_keys;

    /// The ceremony is a precondition here, not the subject, so it runs against the
    /// contract without any DKG cryptography.
    #[tokio::test]
    async fn serves_signer_discovery_from_a_concluded_ceremony() -> anyhow::Result<()> {
        let validators = 3;

        let chain = SharedContractChain::new(validators);
        initiate_dkg(&chain);
        let epoch_id = chain.epoch().epoch_id;

        cheap::run_ceremony(&chain, false);
        cheap::install_real_verification_keys(&chain);

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

    /// One signer dropping out during the finalization window leaves its share
    /// unverified on chain. The epoch still concluded and the remaining shares still
    /// meet the threshold, so signer discovery must keep working for everyone else.
    ///
    /// Guards against the conversion rejecting the whole epoch on the first unusable
    /// share, which used to cost every gateway and client signer discovery for that
    /// epoch entirely.
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

    /// B10: an api that kept serving requests throughout a ceremony must answer
    /// correctly once that ceremony concludes, with no restart.
    ///
    /// Gateways poll continuously, so in production *something* will query the new
    /// epoch while its shares are still being submitted. `epoch_clients` caches
    /// whatever it sees under that epoch id with no expiry and no invalidation, so a
    /// single mid-ceremony query pins an empty signer set for good.
    ///
    /// Currently RED. Note the fix for the unverified-share handling widened this:
    /// mid-ceremony queries used to fail (and errors are not cached), whereas now they
    /// succeed with an empty list, which is exactly what gets cached.
    ///
    /// The ceremony here is a precondition, not the subject, so it runs against the
    /// contract without any DKG cryptography.
    #[tokio::test]
    async fn signer_discovery_recovers_after_a_ceremony_without_a_restart() -> anyhow::Result<()> {
        let validators = 3;

        let chain = SharedContractChain::new(validators);
        initiate_dkg(&chain);
        let epoch_id = chain.epoch().epoch_id;

        let channel =
            QueryCommunicationChannel::new(ContractChainClient::new(chain.admin(), chain.clone()));

        // a gateway hits the api after every phase of the ceremony. none of these are
        // expected to succeed - the point is that asking must not poison later answers.
        cheap::register_dealers(&chain, false);
        let _ = channel.ecash_clients(epoch_id).await;

        cheap::advance(&chain);
        cheap::submit_dealings(&chain, false);
        let _ = channel.ecash_clients(epoch_id).await;

        cheap::advance(&chain);
        cheap::submit_vk_shares(&chain, false);
        let _ = channel.ecash_clients(epoch_id).await;

        cheap::advance(&chain);
        let _ = channel.ecash_clients(epoch_id).await;

        cheap::advance(&chain);
        cheap::verify_vk_shares(&chain, false);
        cheap::advance(&chain);

        cheap::install_real_verification_keys(&chain);

        // the ceremony is over and every dealer is a verified signer on chain
        for member in chain.group_member_addresses() {
            assert!(chain.vk_share_verified(epoch_id, &member));
        }

        // so the same long-lived channel must now discover them, without being restarted
        let clients = channel.ecash_clients(epoch_id).await?;
        assert_eq!(clients.len(), validators);

        Ok(())
    }
}
