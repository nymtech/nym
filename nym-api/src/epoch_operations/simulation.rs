// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Performance methodology comparison system
//!
//! This module provides functionality to compare different performance calculation
//! methodologies without affecting actual rewards, enabling analysis of:
//! - Old method: 24-hour cache-based performance calculation  
//! - New method: 1-hour route-based performance calculation
//!
//! The system focuses on performance metrics and rankings rather than reward amounts,
//! as actual rewards are calculated on-chain based on factors not available to the API.

use crate::epoch_operations::error::RewardingError;
use crate::epoch_operations::helpers::RewardedNodeWithParams;
use crate::storage::models::{
    SimulatedNodePerformance, SimulatedPerformanceComparison, SimulatedPerformanceRanking,
    SimulatedRouteAnalysis,
};
use crate::support::storage::NymApiStorage;
use crate::EpochAdvancer;
use nym_contracts_common::types::NaiveFloat;
use nym_mixnet_contract_common::reward_params::Performance;
use nym_mixnet_contract_common::{EpochRewardedSet, NodeId, RewardingParams};
use std::collections::HashMap;
use time::OffsetDateTime;
use tracing::{debug, error, info};

/// Configuration for simulation runs
#[derive(Debug, Clone)]
pub struct SimulationConfig {
    /// Time window in hours for new method calculation (default: 1)
    pub new_method_time_window_hours: u32,
    /// Description for this simulation run
    pub description: Option<String>,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            new_method_time_window_hours: 1,
            description: None,
        }
    }
}

/// Main simulation coordinator
pub struct SimulationCoordinator<'a> {
    storage: &'a NymApiStorage,
    config: SimulationConfig,
}

impl<'a> SimulationCoordinator<'a> {
    pub fn new(storage: &'a NymApiStorage, config: SimulationConfig) -> Self {
        Self { storage, config }
    }

    /// Run simulation using new rewarding method only
    pub async fn run_simulation(
        &self,
        rewarded_set: &EpochRewardedSet,
        reward_params: RewardingParams,
        current_epoch_id: u32,
    ) -> Result<(), RewardingError> {
        let now = OffsetDateTime::now_utc();
        let end_timestamp = now.unix_timestamp();
        let start_timestamp =
            end_timestamp - (self.config.new_method_time_window_hours as i64 * 3600);

        info!(
            "Starting new method simulation for epoch {} with time window {}h",
            current_epoch_id, self.config.new_method_time_window_hours
        );

        // Create simulation epoch record or get existing one
        let (epoch_db_id, is_new) = self
            .storage
            .manager
            .create_or_get_simulated_reward_epoch(
                current_epoch_id,
                "new",
                start_timestamp,
                end_timestamp,
                self.config.description.as_deref(),
            )
            .await
            .map_err(|e| RewardingError::DatabaseError { source: e.into() })?;

        if !is_new {
            info!(
                "Simulation for epoch {} already exists (id: {}), skipping duplicate simulation",
                current_epoch_id, epoch_db_id
            );
            return Ok(());
        }

        // Run new method simulation only
        match self
            .run_new_method_simulation(rewarded_set, reward_params, epoch_db_id, end_timestamp)
            .await
        {
            Ok(_) => {
                info!("New method simulation completed successfully");
            }
            Err(e) => {
                error!("New method simulation failed: {}", e);
                return Err(e);
            }
        }

        info!("Simulation completed for epoch {}", current_epoch_id);
        Ok(())
    }

    /// Run simulation using new method (1h route-based)
    async fn run_new_method_simulation(
        &self,
        rewarded_set: &EpochRewardedSet,
        reward_params: RewardingParams,
        epoch_db_id: i64,
        end_timestamp: i64,
    ) -> Result<(), RewardingError> {
        debug!(
            "Running new method simulation ({}h route-based)",
            self.config.new_method_time_window_hours
        );

        let time_window_secs = (self.config.new_method_time_window_hours as i64) * 3600;
        let start_timestamp = end_timestamp - time_window_secs;

        // Get route-based performance data using new method
        let corrected_reliabilities = self
            .storage
            .manager
            .calculate_corrected_node_reliabilities_for_interval(start_timestamp, end_timestamp)
            .await
            .map_err(|e| RewardingError::DatabaseError { source: e })?;

        // Convert to performance map and build route reliability data
        let mut performance_map = HashMap::new();
        let mut route_reliability_map = HashMap::new();
        let mut total_routes = 0u32;
        let mut successful_routes = 0u32;

        for node_reliability in &corrected_reliabilities {
            let total_samples =
                node_reliability.pos_samples_in_interval + node_reliability.neg_samples_in_interval;
            total_routes += total_samples;
            successful_routes += node_reliability.pos_samples_in_interval;
            performance_map.insert(
                node_reliability.node_id,
                Performance::naive_try_from_f64(node_reliability.reliability / 100.0)
                    .unwrap_or_default(),
            );

            // Store sample counts for detailed performance records
            route_reliability_map.insert(
                node_reliability.node_id,
                (
                    node_reliability.pos_samples_in_interval,
                    node_reliability.neg_samples_in_interval,
                ),
            );
        }

        // Calculate rewards using new method logic
        let rewarded_nodes =
            self.calculate_rewards_for_nodes(rewarded_set, reward_params, &performance_map);

        // Convert to simulation data structures
        let node_performance = self
            .convert_to_simulated_performance(
                &rewarded_nodes,
                rewarded_set,
                reward_params,
                epoch_db_id,
                "new",
                Some(&route_reliability_map), // Pass route sample data for new method
            )
            .await;

        let performance_comparisons = self
            .convert_to_performance_comparisons(
                &rewarded_nodes,
                rewarded_set,
                reward_params,
                epoch_db_id,
                "new",
            )
            .await;

        // Calculate average reliability for new method (mean of all node reliabilities)
        let node_reliabilities: Vec<f64> = corrected_reliabilities
            .iter()
            .filter(|n| n.pos_samples_in_interval + n.neg_samples_in_interval > 0)
            .map(|n| n.reliability)
            .collect();

        let (mean_reliability, median_reliability) = if !node_reliabilities.is_empty() {
            let mean = node_reliabilities.iter().sum::<f64>() / node_reliabilities.len() as f64;
            let median = calculate_median(&node_reliabilities);
            (
                Some((mean * 100.0).round() / 100.0),
                Some((median * 100.0).round() / 100.0),
            )
        } else {
            (None, None)
        };

        // Create route analysis for new method
        let route_analysis = SimulatedRouteAnalysis {
            id: 0, // Will be set by database
            simulated_epoch_id: epoch_db_id,
            calculation_method: "new".to_string(),
            total_routes_analyzed: total_routes,
            successful_routes,
            failed_routes: total_routes - successful_routes,
            average_route_reliability: mean_reliability,
            time_window_hours: self.config.new_method_time_window_hours,
            analysis_parameters: Some(format!(
                "{{\"method\":\"route_based\",\"time_window_hours\":{},\"corrected_routes\":{},\"median_reliability\":{},\"nodes_analyzed\":{}}}",
                self.config.new_method_time_window_hours,
                corrected_reliabilities.len(),
                median_reliability.unwrap_or(0.0),
                node_reliabilities.len()
            )),
            calculated_at: OffsetDateTime::now_utc().unix_timestamp(),
        };

        // Store results in database
        self.storage
            .manager
            .insert_simulated_node_performance(&node_performance)
            .await
            .map_err(|e| RewardingError::DatabaseError { source: e.into() })?;

        self.storage
            .manager
            .insert_simulated_performance_comparisons(&performance_comparisons)
            .await
            .map_err(|e| RewardingError::DatabaseError { source: e.into() })?;

        // Calculate and store performance rankings
        let rankings =
            self.calculate_performance_rankings(&performance_comparisons, epoch_db_id, "new");
        self.storage
            .manager
            .insert_simulated_performance_rankings(&rankings)
            .await
            .map_err(|e| RewardingError::DatabaseError { source: e.into() })?;

        self.storage
            .manager
            .insert_simulated_route_analysis(&route_analysis)
            .await
            .map_err(|e| RewardingError::DatabaseError { source: e.into() })?;

        Ok(())
    }

    /// Calculate rewards for nodes using the provided performance data
    /// This mirrors the logic from helpers.rs but uses simulation performance data
    fn calculate_rewards_for_nodes(
        &self,
        rewarded_set: &EpochRewardedSet,
        reward_params: RewardingParams,
        performance_map: &HashMap<NodeId, Performance>,
    ) -> Vec<RewardedNodeWithParams> {
        let nodes = &rewarded_set.assignment;
        let active_node_work_factor = reward_params.active_node_work();
        let standby_node_work_factor = reward_params.standby_node_work();

        let mut rewarded_nodes = Vec::with_capacity(nodes.rewarded_set_size());

        // Process active set mixnodes (layers 1, 2, 3)
        for &node_id in nodes
            .layer1
            .iter()
            .chain(nodes.layer2.iter())
            .chain(nodes.layer3.iter())
        {
            let performance = performance_map.get(&node_id).copied().unwrap_or_default();
            rewarded_nodes.push(RewardedNodeWithParams {
                node_id,
                params: nym_mixnet_contract_common::reward_params::NodeRewardingParameters {
                    performance,
                    work_factor: active_node_work_factor,
                },
            });
        }

        // Process active set gateways
        for &node_id in nodes
            .entry_gateways
            .iter()
            .chain(nodes.exit_gateways.iter())
        {
            let performance = performance_map.get(&node_id).copied().unwrap_or_default();
            rewarded_nodes.push(RewardedNodeWithParams {
                node_id,
                params: nym_mixnet_contract_common::reward_params::NodeRewardingParameters {
                    performance,
                    work_factor: active_node_work_factor,
                },
            });
        }

        // Process standby nodes
        for &node_id in &nodes.standby {
            let performance = performance_map.get(&node_id).copied().unwrap_or_default();
            rewarded_nodes.push(RewardedNodeWithParams {
                node_id,
                params: nym_mixnet_contract_common::reward_params::NodeRewardingParameters {
                    performance,
                    work_factor: standby_node_work_factor,
                },
            });
        }

        rewarded_nodes
    }

    /// Determine node type and work factor from rewarded set position
    fn determine_node_info(
        &self,
        node_id: NodeId,
        rewarded_set: &EpochRewardedSet,
        reward_params: RewardingParams,
    ) -> (String, f64) {
        let nodes = &rewarded_set.assignment;

        // Check if node is in active mixnode layers
        if nodes.layer1.contains(&node_id)
            || nodes.layer2.contains(&node_id)
            || nodes.layer3.contains(&node_id)
        {
            return (
                "mixnode".to_string(),
                reward_params.active_node_work().naive_to_f64(),
            );
        }

        // Check if node is in active gateways
        if nodes.entry_gateways.contains(&node_id) || nodes.exit_gateways.contains(&node_id) {
            return (
                "gateway".to_string(),
                reward_params.active_node_work().naive_to_f64(),
            );
        }

        // Check if node is in standby (could be mixnode or gateway)
        if nodes.standby.contains(&node_id) {
            // Note: We cannot determine if standby nodes are mixnodes or gateways from the
            // rewarded set data alone. This limitation exists in both old and new calculation
            // methods and doesn't significantly impact the simulation since:
            // 1. All standby nodes receive the same work factor regardless of type
            // 2. The gateway 3-sample rule can't be applied to standby nodes in either method
            // For consistency, we label all standby nodes as "mixnode" in the simulation data
            return (
                "mixnode".to_string(),
                reward_params.standby_node_work().naive_to_f64(),
            );
        }

        // Default case (shouldn't happen)
        ("unknown".to_string(), 0.0)
    }

    /// Convert rewarded nodes to simulated performance records
    async fn convert_to_simulated_performance(
        &self,
        rewarded_nodes: &[RewardedNodeWithParams],
        rewarded_set: &EpochRewardedSet,
        reward_params: RewardingParams,
        epoch_db_id: i64,
        method: &str,
        route_reliability_map: Option<&HashMap<NodeId, (u32, u32)>>, // (positive_samples, negative_samples)
    ) -> Vec<SimulatedNodePerformance> {
        let now = OffsetDateTime::now_utc().unix_timestamp();

        let mut performance_records = Vec::with_capacity(rewarded_nodes.len());

        // First collect all node IDs for batch identity key lookups
        let all_node_ids: Vec<NodeId> = rewarded_nodes.iter().map(|node| node.node_id).collect();

        // Batch fetch identity keys for both mixnodes and gateways
        let mixnode_identities = self
            .storage
            .manager
            .get_mixnode_identity_keys_batch(&all_node_ids)
            .await
            .unwrap_or_default();
        let gateway_identities = self
            .storage
            .manager
            .get_gateway_identity_keys_batch(&all_node_ids)
            .await
            .unwrap_or_default();

        for node in rewarded_nodes {
            let (node_type, _) =
                self.determine_node_info(node.node_id, rewarded_set, reward_params);

            // Get identity key from our batch results
            let identity_key = match node_type.as_str() {
                "mixnode" => mixnode_identities.get(&node.node_id).cloned(),
                "gateway" => gateway_identities.get(&node.node_id).cloned(),
                _ => None,
            };

            // Extract sample counts from route reliability if available
            let (positive_samples, negative_samples) = route_reliability_map
                .and_then(|map| map.get(&node.node_id))
                .copied()
                .unwrap_or((0, 0));

            performance_records.push(SimulatedNodePerformance {
                id: 0, // Will be set by database
                simulated_epoch_id: epoch_db_id,
                node_id: node.node_id,
                node_type,
                identity_key,
                reliability_score: node.params.performance.naive_to_f64() * 100.0,
                positive_samples,
                negative_samples,
                work_factor: Some(node.params.work_factor.naive_to_f64()),
                calculation_method: method.to_string(),
                calculated_at: now,
            });
        }

        performance_records
    }

    /// Convert rewarded nodes to performance comparison records
    async fn convert_to_performance_comparisons(
        &self,
        rewarded_nodes: &[RewardedNodeWithParams],
        rewarded_set: &EpochRewardedSet,
        reward_params: RewardingParams,
        epoch_db_id: i64,
        method: &str,
    ) -> Vec<SimulatedPerformanceComparison> {
        let now = OffsetDateTime::now_utc().unix_timestamp();

        let mut performance_records = Vec::with_capacity(rewarded_nodes.len());

        for node in rewarded_nodes {
            let (node_type, _) =
                self.determine_node_info(node.node_id, rewarded_set, reward_params);

            performance_records.push(SimulatedPerformanceComparison {
                id: 0, // Will be set by database
                simulated_epoch_id: epoch_db_id,
                node_id: node.node_id,
                node_type,
                performance_score: node.params.performance.naive_to_f64() * 100.0,
                work_factor: node.params.work_factor.naive_to_f64(),
                calculation_method: method.to_string(),
                positive_samples: None, // Will be populated from node performance data
                negative_samples: None,
                route_success_rate: None,
                calculated_at: now,
            });
        }

        performance_records
    }

    /// Calculate performance rankings for a set of performance comparisons
    fn calculate_performance_rankings(
        &self,
        performance_comparisons: &[SimulatedPerformanceComparison],
        epoch_db_id: i64,
        method: &str,
    ) -> Vec<SimulatedPerformanceRanking> {
        let now = OffsetDateTime::now_utc().unix_timestamp();
        let mut rankings = Vec::with_capacity(performance_comparisons.len());

        // Sort by performance score descending
        let mut sorted_comparisons: Vec<_> = performance_comparisons
            .iter()
            .enumerate()
            .map(|(idx, comp)| (idx, comp.performance_score))
            .collect();
        sorted_comparisons
            .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let total_nodes = sorted_comparisons.len() as f64;

        for (rank, (original_idx, _score)) in sorted_comparisons.iter().enumerate() {
            let comparison = &performance_comparisons[*original_idx];
            let percentile = ((total_nodes - rank as f64 - 1.0) / total_nodes) * 100.0;

            rankings.push(SimulatedPerformanceRanking {
                id: 0, // Will be set by database
                simulated_epoch_id: epoch_db_id,
                node_id: comparison.node_id,
                calculation_method: method.to_string(),
                performance_rank: (rank + 1) as i64,
                performance_percentile: percentile,
                calculated_at: now,
            });
        }

        rankings
    }
}

impl EpochAdvancer {
    /// Run simulation during epoch operations if simulation mode is enabled
    pub async fn run_simulation_if_enabled(
        &self,
        rewarded_set: &EpochRewardedSet,
        reward_params: RewardingParams,
        current_epoch_id: u32,
        simulation_config: SimulationConfig,
    ) -> Result<(), RewardingError> {
        let coordinator = SimulationCoordinator::new(&self.storage, simulation_config);

        match coordinator
            .run_simulation(rewarded_set, reward_params, current_epoch_id)
            .await
        {
            Ok(()) => {
                info!(
                    "Simulation completed successfully for epoch {}",
                    current_epoch_id
                );
                Ok(())
            }
            Err(e) => {
                error!("Simulation failed for epoch {}: {}", current_epoch_id, e);
                // Don't fail the entire epoch operation due to simulation failure
                Ok(())
            }
        }
    }
}

/// Calculate median of a vector of f64 values
fn calculate_median(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let len = sorted.len();
    if len % 2 == 0 {
        (sorted[len / 2 - 1] + sorted[len / 2]) / 2.0
    } else {
        sorted[len / 2]
    }
}
