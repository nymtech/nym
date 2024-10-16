// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::NymContractCache;
use crate::nym_contract_cache::cache::data::{CachedContractInfo, CachedContractsInfo};
use crate::nyxd::Client;
use crate::support::caching::CacheNotification;
use anyhow::Result;
use nym_api_requests::legacy::{
    LegacyGatewayBondWithId, LegacyMixNodeBondWithLayer, LegacyMixNodeDetailsWithLayer,
};
use nym_mixnet_contract_common::{LegacyMixLayer, RewardedSet};
use nym_task::TaskClient;
use nym_validator_client::nyxd::contract_traits::{
    MixnetQueryClient, NymContractsProvider, VestingQueryClient,
};
use rand::prelude::SliceRandom;
use rand::rngs::OsRng;
use std::collections::HashSet;
use std::{collections::HashMap, sync::atomic::Ordering, time::Duration};
use tokio::sync::watch;
use tokio::time;
use tracing::{error, info, trace, warn};

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

        let nym_nodes = self.nyxd_client.get_nymnodes().await?;
        let mixnode_details = self.nyxd_client.get_mixnodes().await?;
        let gateway_bonds = self.nyxd_client.get_gateways().await?;
        let gateway_ids: HashMap<_, _> = self
            .nyxd_client
            .get_gateway_ids()
            .await?
            .into_iter()
            .map(|id| (id.identity, id.node_id))
            .collect();

        let mut gateways = Vec::with_capacity(gateway_bonds.len());
        for bond in gateway_bonds {
            // we explicitly panic here because that value MUST exist.
            // if it doesn't, we messed up the migration and we have big problems
            let node_id = *gateway_ids.get(bond.identity()).unwrap_or_else(|| {
                panic!(
                    "CONTRACT DATA INCONSISTENCY: MISSING GATEWAY ID FOR: {}",
                    bond.identity()
                )
            });
            gateways.push(LegacyGatewayBondWithId { bond, node_id })
        }

        let rewarded_set = self.get_rewarded_set().await;
        let layer1 = rewarded_set.layer1.iter().collect::<HashSet<_>>();
        let layer2 = rewarded_set.layer2.iter().collect::<HashSet<_>>();
        let layer3 = rewarded_set.layer3.iter().collect::<HashSet<_>>();

        let layer_choices = [
            LegacyMixLayer::One,
            LegacyMixLayer::Two,
            LegacyMixLayer::Three,
        ];
        let mut rng = OsRng;
        let mut mixnodes = Vec::with_capacity(mixnode_details.len());
        for detail in mixnode_details {
            // if node is not in the rewarded set, well.
            // slap a random layer on it because legacy clients don't understand a concept of layerless mixnodes
            let layer = if layer1.contains(&detail.mix_id()) {
                LegacyMixLayer::One
            } else if layer2.contains(&detail.mix_id()) {
                LegacyMixLayer::Two
            } else if layer3.contains(&detail.mix_id()) {
                LegacyMixLayer::Three
            } else {
                // SAFETY: the slice is not empty so the unwrap is fine
                #[allow(clippy::unwrap_used)]
                layer_choices.choose(&mut rng).copied().unwrap()
            };

            mixnodes.push(LegacyMixNodeDetailsWithLayer {
                bond_information: LegacyMixNodeBondWithLayer {
                    bond: detail.bond_information,
                    layer,
                },
                rewarding_details: detail.rewarding_details,
                pending_changes: detail.pending_changes,
            })
        }

        let contract_info = self.get_nym_contracts_info().await?;

        info!(
            "Updating validator cache. There are {} [legacy] mixnodes, {} [legacy] gateways and {} nym nodes",
            mixnodes.len(),
            gateways.len(),
            nym_nodes.len(),
        );

        self.cache
            .update(
                mixnodes,
                gateways,
                nym_nodes,
                rewarded_set,
                rewarding_params,
                current_interval,
                contract_info,
            )
            .await;

        if let Err(err) = self.update_notifier.send(CacheNotification::Updated) {
            warn!("Failed to notify validator cache refresh: {err}");
        }

        Ok(())
    }

    async fn get_rewarded_set(&self) -> RewardedSet {
        self.nyxd_client
            .get_rewarded_set_nodes()
            .await
            .unwrap_or_default()
    }

    // fn collect_rewarded_and_active_set_details(
    //     all_mixnodes: &[MixNodeDetails],
    //     rewarded_set_nodes: RewardedSet,
    // ) -> (Vec<MixNodeDetails>, Vec<MixNodeDetails>) {
    //     let mut active_set = Vec::new();
    //     let mut rewarded_set = Vec::new();
    //
    //     for mix in all_mixnodes {
    //         if let Some(status) = rewarded_set_nodes.get(&mix.mix_id()) {
    //             rewarded_set.push(mix.clone());
    //             if status.is_active() {
    //                 active_set.push(mix.clone())
    //             }
    //         }
    //     }
    //
    //     (rewarded_set, active_set)
    // }

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
