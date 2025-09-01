// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::NodeStatusCache;
use crate::mixnet_contract_cache::cache::data::ConfigScoreData;
use crate::node_describe_cache::cache::DescribedNodes;
use crate::node_performance::provider::{NodePerformanceProvider, NodesRoutingScores};
use crate::node_status_api::cache::config_score::calculate_config_score;
use crate::node_status_api::models::Uptime;
use crate::support::caching::cache::SharedCache;
use crate::support::legacy_helpers::legacy_host_to_ips_and_hostname;
use crate::{
    mixnet_contract_cache::cache::MixnetContractCache,
    node_status_api::cache::NodeStatusCacheError, support::caching::CacheNotification,
};
use ::time::OffsetDateTime;
use cosmwasm_std::Decimal;
use nym_api_requests::legacy::{LegacyGatewayBondWithId, LegacyMixNodeDetailsWithLayer};
use nym_api_requests::models::{
    DetailedNodePerformance, GatewayBondAnnotated, MixNodeBondAnnotated, NodeAnnotation,
    NodePerformance,
};
use nym_mixnet_contract_common::{NodeId, NymNodeDetails, RewardingParams};
use nym_task::ShutdownToken;
use nym_topology::CachedEpochRewardedSet;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::watch;
use tokio::time;
use tracing::{info, trace, warn};

// Long running task responsible for keeping the node status cache up-to-date.
pub struct NodeStatusCacheRefresher {
    // Main stored data
    cache: NodeStatusCache,
    fallback_caching_interval: Duration,

    // Sources for when refreshing data
    mixnet_contract_cache: MixnetContractCache,
    described_cache: SharedCache<DescribedNodes>,
    mixnet_contract_cache_listener: watch::Receiver<CacheNotification>,
    describe_cache_listener: watch::Receiver<CacheNotification>,

    performance_provider: Box<dyn NodePerformanceProvider + Send + Sync>,
}

impl NodeStatusCacheRefresher {
    pub(crate) fn new(
        cache: NodeStatusCache,
        fallback_caching_interval: Duration,
        contract_cache: MixnetContractCache,
        described_cache: SharedCache<DescribedNodes>,
        contract_cache_listener: watch::Receiver<CacheNotification>,
        describe_cache_listener: watch::Receiver<CacheNotification>,
        performance_provider: Box<dyn NodePerformanceProvider + Send + Sync>,
    ) -> Self {
        Self {
            cache,
            fallback_caching_interval,
            mixnet_contract_cache: contract_cache,
            described_cache,
            mixnet_contract_cache_listener: contract_cache_listener,
            describe_cache_listener,
            performance_provider,
        }
    }

    /// Runs the node status cache refresher task.
    pub async fn run(&mut self, shutdown_token: ShutdownToken) {
        let mut last_update = OffsetDateTime::now_utc();
        let mut fallback_interval = time::interval(self.fallback_caching_interval);
        while !shutdown_token.is_cancelled() {
            tokio::select! {
                biased;
                _ = shutdown_token.cancelled() => {
                    trace!("NodeStatusCacheRefresher: Received shutdown");
                }
                // Update node status cache when the contract cache / describe cache is updated
                Ok(_) = self.mixnet_contract_cache_listener.changed() => {
                    tokio::select! {
                        _ = self.maybe_refresh(&mut fallback_interval, &mut last_update) => (),
                        _ = shutdown_token.cancelled() => {
                            trace!("NodeStatusCacheRefresher: Received shutdown");
                        }
                    }
                }
                Ok(_) = self.describe_cache_listener.changed() => {
                    tokio::select! {
                        _ = self.maybe_refresh(&mut fallback_interval, &mut last_update) => (),
                        _ = shutdown_token.cancelled() => {
                            trace!("NodeStatusCacheRefresher: Received shutdown");
                        }
                    }
                }
                // ... however, if we don't receive any notifications we fall back to periodic
                // refreshes
                _ = fallback_interval.tick() => {
                    tokio::select! {
                        _ = self.maybe_refresh(&mut fallback_interval, &mut last_update) => (),
                        _ = shutdown_token.cancelled() => {
                            trace!("NodeStatusCacheRefresher: Received shutdown");
                        }
                    }
                }
            }
        }
        info!("NodeStatusCacheRefresher: Exiting");
    }

    fn caches_available(&self) -> bool {
        let contract_cache =
            *self.mixnet_contract_cache_listener.borrow() != CacheNotification::Start;
        let describe_cache = *self.describe_cache_listener.borrow() != CacheNotification::Start;

        let available = contract_cache && describe_cache;
        if !available {
            warn!(
                contract_cache,
                describe_cache, "auxiliary caches data is not yet available"
            )
        }

        available
    }

    async fn maybe_refresh(
        &self,
        fallback_interval: &mut time::Interval,
        last_updated: &mut OffsetDateTime,
    ) {
        if !self.caches_available() {
            trace!("not updating the cache since the auxiliary data is not yet available");
            return;
        }

        if OffsetDateTime::now_utc() - *last_updated < self.fallback_caching_interval {
            // don't update too often
            trace!("not updating the cache since they've been updated recently");
            return;
        }

        let _ = self.refresh().await;
        *last_updated = OffsetDateTime::now_utc();
        fallback_interval.reset();
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn produce_node_annotations(
        &self,
        config_score_data: &ConfigScoreData,
        routing_scores: &NodesRoutingScores,
        legacy_mixnodes: &[LegacyMixNodeDetailsWithLayer],
        legacy_gateways: &[LegacyGatewayBondWithId],
        nym_nodes: &[NymNodeDetails],
        rewarded_set: &CachedEpochRewardedSet,
        described_nodes: &DescribedNodes,
    ) -> HashMap<NodeId, NodeAnnotation> {
        let mut annotations = HashMap::new();

        for legacy_mix in legacy_mixnodes {
            let node_id = legacy_mix.mix_id();
            let routing_score = routing_scores.get_or_log(node_id);

            let config_score =
                calculate_config_score(config_score_data, described_nodes.get_node(&node_id));

            let performance = routing_score.score * config_score.score;
            // map it from 0-1 range into 0-100
            let scaled_performance = performance * 100.;
            let legacy_performance = Uptime::new(scaled_performance as f32).into();

            annotations.insert(
                legacy_mix.mix_id(),
                NodeAnnotation {
                    last_24h_performance: legacy_performance,
                    current_role: rewarded_set.role(legacy_mix.mix_id()).map(|r| r.into()),
                    detailed_performance: DetailedNodePerformance::new(
                        performance,
                        routing_score,
                        config_score,
                    ),
                },
            );
        }

        for legacy_gateway in legacy_gateways {
            let node_id = legacy_gateway.node_id;
            let routing_score = routing_scores.get_or_log(node_id);
            let config_score =
                calculate_config_score(config_score_data, described_nodes.get_node(&node_id));

            let performance = routing_score.score * config_score.score;
            // map it from 0-1 range into 0-100
            let scaled_performance = performance * 100.;
            let legacy_performance = Uptime::new(scaled_performance as f32).into();

            annotations.insert(
                legacy_gateway.node_id,
                NodeAnnotation {
                    last_24h_performance: legacy_performance,
                    current_role: rewarded_set.role(legacy_gateway.node_id).map(|r| r.into()),
                    detailed_performance: DetailedNodePerformance::new(
                        performance,
                        routing_score,
                        config_score,
                    ),
                },
            );
        }

        for nym_node in nym_nodes {
            let node_id = nym_node.node_id();
            let routing_score = routing_scores.get_or_log(node_id);
            let config_score =
                calculate_config_score(config_score_data, described_nodes.get_node(&node_id));

            let performance = routing_score.score * config_score.score;
            // map it from 0-1 range into 0-100
            let scaled_performance = performance * 100.;
            let legacy_performance = Uptime::new(scaled_performance as f32).into();

            annotations.insert(
                nym_node.node_id(),
                NodeAnnotation {
                    last_24h_performance: legacy_performance,
                    current_role: rewarded_set.role(nym_node.node_id()).map(|r| r.into()),
                    detailed_performance: DetailedNodePerformance::new(
                        performance,
                        routing_score,
                        config_score,
                    ),
                },
            );
        }

        annotations
    }

    #[deprecated]
    pub(super) async fn annotate_legacy_mixnodes_nodes_with_details(
        &self,
        mixnodes: Vec<LegacyMixNodeDetailsWithLayer>,
        routing_scores: &NodesRoutingScores,
        interval_reward_params: RewardingParams,
    ) -> HashMap<NodeId, MixNodeBondAnnotated> {
        let mut annotated = HashMap::new();
        for mixnode in mixnodes {
            let stake_saturation = mixnode
                .rewarding_details
                .bond_saturation(&interval_reward_params);

            let uncapped_stake_saturation = mixnode
                .rewarding_details
                .uncapped_bond_saturation(&interval_reward_params);

            let score = routing_scores.get_or_log(mixnode.mix_id());
            let legacy_report = NodePerformance {
                most_recent: score.legacy_performance(),
                last_hour: score.legacy_performance(),
                last_24h: score.legacy_performance(),
            };

            let Some((ip_addresses, _)) =
                legacy_host_to_ips_and_hostname(&mixnode.bond_information.mix_node.host)
            else {
                continue;
            };

            // legacy node will never get rewarded
            let estimated_operator_apy = Decimal::zero();
            let estimated_delegators_apy = Decimal::zero();

            annotated.insert(
                mixnode.mix_id(),
                MixNodeBondAnnotated {
                    // all legacy nodes are always blacklisted
                    blacklisted: true,
                    mixnode_details: mixnode,
                    stake_saturation,
                    uncapped_stake_saturation,
                    performance: score.legacy_performance(),
                    node_performance: legacy_report,
                    estimated_operator_apy,
                    estimated_delegators_apy,
                    ip_addresses,
                },
            );
        }
        annotated
    }

    #[deprecated]
    pub(crate) async fn annotate_legacy_gateways_with_details(
        &self,
        gateway_bonds: Vec<LegacyGatewayBondWithId>,
        routing_scores: &NodesRoutingScores,
    ) -> HashMap<NodeId, GatewayBondAnnotated> {
        let mut annotated = HashMap::new();
        for gateway_bond in gateway_bonds {
            let score = routing_scores.get_or_log(gateway_bond.node_id);
            let legacy_report = NodePerformance {
                most_recent: score.legacy_performance(),
                last_hour: score.legacy_performance(),
                last_24h: score.legacy_performance(),
            };

            let Some((ip_addresses, _)) =
                legacy_host_to_ips_and_hostname(&gateway_bond.bond.gateway.host)
            else {
                continue;
            };

            annotated.insert(
                gateway_bond.node_id,
                GatewayBondAnnotated {
                    // all legacy nodes are always blacklisted
                    blacklisted: true,
                    gateway_bond,
                    self_described: None,
                    performance: score.legacy_performance(),
                    node_performance: legacy_report,
                    ip_addresses,
                },
            );
        }
        annotated
    }

    /// Refreshes the node status cache by fetching the latest data from the contract cache
    #[allow(deprecated)]
    async fn refresh(&self) -> Result<(), NodeStatusCacheError> {
        info!("Updating node status cache");

        // Fetch contract cache data to work with
        let mixnode_details = self.mixnet_contract_cache.legacy_mixnodes_all().await;
        let interval_reward_params = self.mixnet_contract_cache.interval_reward_params().await?;
        let current_interval = self.mixnet_contract_cache.current_interval().await?;
        let rewarded_set = self.mixnet_contract_cache.rewarded_set_owned().await?;
        let gateway_bonds = self.mixnet_contract_cache.legacy_gateways_all().await;
        let nym_nodes = self.mixnet_contract_cache.nym_nodes().await;
        let config_score_data = self.mixnet_contract_cache.maybe_config_score_data().await?;

        // Compute inclusion probabilities
        // (all legacy mixnodes have 0% chance of being selected)
        let inclusion_probabilities = crate::node_status_api::cache::inclusion_probabilities::InclusionProbabilities::legacy_zero(&mixnode_details);

        let Ok(described) = self.described_cache.get().await else {
            return Err(NodeStatusCacheError::UnavailableDescribedCache);
        };

        let mut legacy_gateway_mapping = HashMap::new();
        for gateway in &gateway_bonds {
            legacy_gateway_mapping.insert(gateway.identity().clone(), gateway.node_id);
        }

        let all_ids = mixnode_details
            .iter()
            .map(|m| m.bond_information.mix_id)
            .chain(
                gateway_bonds
                    .iter()
                    .map(|g| g.node_id)
                    .chain(nym_nodes.iter().map(|n| n.bond_information.node_id)),
            )
            .collect::<Vec<_>>();

        // note: any internal errors imply failures for that node in particular
        let routing_scores = self
            .performance_provider
            .get_batch_node_scores(all_ids, current_interval.current_epoch_absolute_id())
            .await?;

        // Create annotated data
        let node_annotations = self
            .produce_node_annotations(
                &config_score_data,
                &routing_scores,
                &mixnode_details,
                &gateway_bonds,
                &nym_nodes,
                &rewarded_set,
                &described,
            )
            .await;

        let mixnodes_annotated = self
            .annotate_legacy_mixnodes_nodes_with_details(
                mixnode_details,
                &routing_scores,
                interval_reward_params,
            )
            .await;

        let gateways_annotated = self
            .annotate_legacy_gateways_with_details(gateway_bonds, &routing_scores)
            .await;

        // Update the cache
        self.cache
            .update(
                legacy_gateway_mapping,
                node_annotations,
                mixnodes_annotated,
                gateways_annotated,
                inclusion_probabilities,
            )
            .await;
        Ok(())
    }
}
