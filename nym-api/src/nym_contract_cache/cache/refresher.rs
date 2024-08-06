// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::NymContractCache;
use crate::nym_contract_cache::cache::data::{CachedContractInfo, CachedContractsInfo};
use crate::nyxd::Client;
use crate::support::caching::CacheNotification;
use anyhow::Result;
use nym_mixnet_contract_common::{MixId, MixNodeDetails, RewardedSetNodeStatus};
use nym_task::TaskClient;
use nym_validator_client::nyxd::contract_traits::{
    MixnetQueryClient, NymContractsProvider, VestingQueryClient,
};
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
        let coconut_dkg = query_guard!(client_guard, dkg_contract_address());
        let group = query_guard!(client_guard, group_contract_address());
        let multisig = query_guard!(client_guard, multisig_contract_address());
        let ecash = query_guard!(client_guard, ecash_contract_address());

        for (address, name) in [
            (mixnet, "nym-mixnet-contract"),
            (vesting, "nym-vesting-contract"),
            (coconut_dkg, "nym-coconut-dkg-contract"),
            (group, "nym-cw4-group-contract"),
            (multisig, "nym-cw3-multisig-contract"),
            (ecash, "nym-ecash-contract"),
        ] {
            let (cw2, build_info) = if let Some(address) = address {
                let cw2 = query_guard!(client_guard, try_get_cw2_contract_version(address).await);
                let mut build_info = query_guard!(
                    client_guard,
                    try_get_contract_build_information(address).await
                );

                // for backwards compatibility until we migrate the contracts
                if build_info.is_none() {
                    match name {
                        "nym-mixnet-contract" => {
                            build_info = Some(query_guard!(
                                client_guard,
                                get_mixnet_contract_version().await
                            )?)
                        }
                        "nym-vesting-contract" => {
                            build_info = Some(query_guard!(
                                client_guard,
                                get_vesting_contract_version().await
                            )?)
                        }
                        _ => (),
                    }
                }

                (cw2, build_info)
            } else {
                (None, None)
            };

            updated.insert(
                name.to_string(),
                CachedContractInfo::new(address, cw2, build_info),
            );
        }

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
