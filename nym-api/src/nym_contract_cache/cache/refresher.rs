// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::NymContractCache;
use crate::nym_contract_cache::cache::data::{CachedContractInfo, CachedContractsInfo};
use crate::nyxd::Client;
use crate::support::caching::CacheNotification;
use anyhow::Result;
use futures::future::join_all;
use nym_mixnet_contract_common::{MixId, MixNodeDetails, RewardedSetNodeStatus};
use nym_task::TaskClient;
use nym_validator_client::nyxd::contract_traits::{
    MixnetQueryClient, NymContractsProvider, PagedSpDirectoryQueryClient, SpDirectoryQueryClient,
    VestingQueryClient,
};
use nym_validator_client::nyxd::CosmWasmClient;
use std::{collections::HashMap, sync::atomic::Ordering, time::Duration};
use tokio::sync::watch;
use tokio::time;

pub struct NymContractCacheRefresher {
    nyxd_client: Client,
    cache: NymContractCache,
    caching_interval: Duration,

    // Notify listeners that the cache has been updated
    update_notifier: watch::Sender<CacheNotification>,
}

impl NymContractCacheRefresher {
    pub(crate) fn new(
        nyxd_client: Client,
        caching_interval: Duration,
        cache: NymContractCache,
    ) -> Self {
        let (tx, _) = watch::channel(CacheNotification::Start);
        NymContractCacheRefresher {
            nyxd_client,
            cache,
            caching_interval,
            update_notifier: tx,
        }
    }

    pub fn subscribe(&self) -> watch::Receiver<CacheNotification> {
        self.update_notifier.subscribe()
    }

    async fn get_nym_contracts_info(&self) -> Result<CachedContractsInfo> {
        use crate::query_guard;

        let mut updated = HashMap::new();

        let client_guard = self.nyxd_client.read().await;

        let mixnet = query_guard!(client_guard, mixnet_contract_address());
        let vesting = query_guard!(client_guard, vesting_contract_address());
        let service_provider = query_guard!(client_guard, service_provider_contract_address());
        let coconut_bandwidth = query_guard!(client_guard, coconut_bandwidth_contract_address());
        let coconut_dkg = query_guard!(client_guard, dkg_contract_address());
        let group = query_guard!(client_guard, group_contract_address());
        let multisig = query_guard!(client_guard, multisig_contract_address());

        // get cw2 versions
        let mixnet_cw2_future = query_guard!(client_guard, get_mixnet_contract_cw2_version());
        let vesting_cw2_future = query_guard!(client_guard, get_vesting_contract_cw2_version());
        let service_provider_cw2_future = query_guard!(client_guard, get_sp_contract_cw2_version());

        // group and multisig contract save that information in their storage but don't expose it via queries
        // so a temporary workaround...
        let multisig_cw2 = if let Some(multisig_contract) = multisig {
            query_guard!(
                client_guard,
                query_contract_raw(multisig_contract, b"contract_info".to_vec())
                    .await
                    .map(|r| serde_json::from_slice(&r).ok())
                    .ok()
                    .flatten()
            )
        } else {
            None
        };
        let group_cw2 = if let Some(group_contract) = group {
            query_guard!(
                client_guard,
                query_contract_raw(group_contract, b"contract_info".to_vec())
                    .await
                    .map(|r| serde_json::from_slice(&r).ok())
                    .ok()
                    .flatten()
            )
        } else {
            None
        };

        let mut cw2_info = join_all(vec![
            mixnet_cw2_future,
            vesting_cw2_future,
            service_provider_cw2_future,
        ])
        .await;

        // get detailed build info
        let mixnet_detailed_future = query_guard!(client_guard, get_mixnet_contract_version());
        let vesting_detailed_future = query_guard!(client_guard, get_vesting_contract_version());
        let service_provider_detailed_future =
            query_guard!(client_guard, get_sp_contract_version());

        let mut build_info = join_all(vec![
            mixnet_detailed_future,
            vesting_detailed_future,
            service_provider_detailed_future,
        ])
        .await;

        // the below unwraps are fine as we definitely have the specified number of entries
        // Note to whoever updates this code in the future: `pop` removes **LAST** element,
        // so make sure you call them in correct order, depending on what's specified in the `join_all`
        updated.insert(
            "nym-service-provider-directory-contract".to_string(),
            CachedContractInfo::new(
                service_provider,
                cw2_info.pop().unwrap().ok(),
                build_info.pop().unwrap().ok(),
            ),
        );

        updated.insert(
            "nym-vesting-contract".to_string(),
            CachedContractInfo::new(
                vesting,
                cw2_info.pop().unwrap().ok(),
                build_info.pop().unwrap().ok(),
            ),
        );
        updated.insert(
            "nym-mixnet-contract".to_string(),
            CachedContractInfo::new(
                mixnet,
                cw2_info.pop().unwrap().ok(),
                build_info.pop().unwrap().ok(),
            ),
        );

        updated.insert(
            "nym-coconut-bandwidth-contract".to_string(),
            CachedContractInfo::new(coconut_bandwidth, None, None),
        );
        updated.insert(
            "nym-coconut-dkg-contract".to_string(),
            CachedContractInfo::new(coconut_dkg, None, None),
        );
        updated.insert(
            "nym-cw3-multisig-contract".to_string(),
            CachedContractInfo::new(multisig, multisig_cw2, None),
        );
        updated.insert(
            "nym-cw4-group-contract".to_string(),
            CachedContractInfo::new(group, group_cw2, None),
        );

        Ok(updated)
    }

    async fn refresh(&self) -> Result<()> {
        let rewarding_params = self.nyxd_client.get_current_rewarding_parameters().await?;
        let current_interval = self.nyxd_client.get_current_interval().await?.interval;

        let mixnodes = self.nyxd_client.get_mixnodes().await?;
        let gateways = self.nyxd_client.get_gateways().await?;

        let mix_to_family = self.nyxd_client.get_all_family_members().await?;

        let rewarded_set_map = self.get_rewarded_set_map().await;

        let (rewarded_set, active_set) =
            Self::collect_rewarded_and_active_set_details(&mixnodes, &rewarded_set_map);

        // The service providers and names are optional
        let services = self.nyxd_client.get_all_services().await.ok();
        let contract_info = self.get_nym_contracts_info().await?;

        info!(
            "Updating validator cache. There are {} mixnodes and {} gateways",
            mixnodes.len(),
            gateways.len(),
        );

        self.cache
            .update(
                mixnodes,
                gateways,
                rewarded_set,
                active_set,
                rewarding_params,
                current_interval,
                mix_to_family,
                services,
                contract_info,
            )
            .await;

        if let Err(err) = self.update_notifier.send(CacheNotification::Updated) {
            warn!("Failed to notify validator cache refresh: {err}");
        }

        Ok(())
    }

    async fn get_rewarded_set_map(&self) -> HashMap<MixId, RewardedSetNodeStatus> {
        self.nyxd_client
            .get_rewarded_set_mixnodes()
            .await
            .map(|nodes| nodes.into_iter().collect())
            .unwrap_or_default()
    }

    fn collect_rewarded_and_active_set_details(
        all_mixnodes: &[MixNodeDetails],
        rewarded_set_nodes: &HashMap<MixId, RewardedSetNodeStatus>,
    ) -> (Vec<MixNodeDetails>, Vec<MixNodeDetails>) {
        let mut active_set = Vec::new();
        let mut rewarded_set = Vec::new();

        for mix in all_mixnodes {
            if let Some(status) = rewarded_set_nodes.get(&mix.mix_id()) {
                rewarded_set.push(mix.clone());
                if status.is_active() {
                    active_set.push(mix.clone())
                }
            }
        }

        (rewarded_set, active_set)
    }

    pub(crate) async fn run(&self, mut shutdown: TaskClient) {
        let mut interval = time::interval(self.caching_interval);
        while !shutdown.is_shutdown() {
            tokio::select! {
                _ = interval.tick() => {
                    tokio::select! {
                        biased;
                        _ = shutdown.recv() => {
                            trace!("ValidatorCacheRefresher: Received shutdown");
                        }
                        ret = self.refresh() => {
                            if let Err(err) = ret {
                                error!("Failed to refresh validator cache - {err}");
                            } else {
                                // relaxed memory ordering is fine here. worst case scenario network monitor
                                // will just have to wait for an additional backoff to see the change.
                                // And so this will not really incur any performance penalties by setting it every loop iteration
                                self.cache.initialised.store(true, Ordering::Relaxed)
                            }
                        }
                    }
                }
                _ = shutdown.recv() => {
                    trace!("ValidatorCacheRefresher: Received shutdown");
                }
            }
        }
    }
}
