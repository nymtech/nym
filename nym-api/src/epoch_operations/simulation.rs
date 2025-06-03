// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Simulation coordinator for comparing old vs new rewarding methodologies
//! 
//! This module provides functionality to run simulated reward calculations without
//! performing blockchain transactions, enabling safe comparison of:
//! - Old method: 24-hour cache-based performance calculation  
//! - New method: 1-hour route-based performance calculation

use crate::epoch_operations::error::RewardingError;
use crate::epoch_operations::helpers::RewardedNodeWithParams;
use crate::storage::models::{SimulatedNodePerformance, SimulatedReward, SimulatedRouteAnalysis};
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
    /// Whether to run both old and new methods (default: true)
    pub run_both_methods: bool,
    /// Description for this simulation run
    pub description: Option<String>,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            new_method_time_window_hours: 1,
            run_both_methods: true,
            description: None,
        }
    }
}

/// Results from a single calculation method
#[derive(Debug, Clone)]
pub struct MethodResults {
    pub method_name: String,
    pub node_performance: Vec<SimulatedNodePerformance>,
    pub rewards: Vec<SimulatedReward>,
    pub route_analysis: SimulatedRouteAnalysis,
}

/// Complete simulation results containing both methods
#[derive(Debug, Clone)]
pub struct SimulationResults {
    pub epoch_id: i64,
    pub old_method: Option<MethodResults>,
    pub new_method: Option<MethodResults>,
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

    /// Run a complete simulation comparing old vs new rewarding methods
    pub async fn run_simulation(
        &self,
        epoch_advancer: &EpochAdvancer,
        rewarded_set: &EpochRewardedSet,
        reward_params: RewardingParams,
        current_epoch_id: u32,
    ) -> Result<SimulationResults, RewardingError> {
        let now = OffsetDateTime::now_utc();
        let end_timestamp = now.unix_timestamp();
        let start_timestamp = end_timestamp - (24 * 3600); // 24 hours ago for baseline
        
        info!(
            "Starting simulation for epoch {} with time window {}h",
            current_epoch_id, self.config.new_method_time_window_hours
        );

        // Create simulation epoch record
        let epoch_db_id = self.storage.manager
            .create_simulated_reward_epoch(
                current_epoch_id,
                "comparison",
                start_timestamp,
                end_timestamp,
                self.config.description.as_deref(),
            )
            .await
            .map_err(|e| RewardingError::DatabaseError { source: e.into() })?;

        let mut results = SimulationResults {
            epoch_id: epoch_db_id,
            old_method: None,
            new_method: None,
        };

        // Run old method simulation (24h cache-based)
        if self.config.run_both_methods {
            match self.run_old_method_simulation(
                epoch_advancer,
                rewarded_set,
                reward_params,
                epoch_db_id,
                end_timestamp,
            ).await {
                Ok(old_results) => {
                    results.old_method = Some(old_results);
                    info!("Old method simulation completed successfully");
                }
                Err(e) => {
                    error!("Old method simulation failed: {}", e);
                    // Continue with new method even if old fails
                }
            }
        }

        // Run new method simulation (1h route-based)
        match self.run_new_method_simulation(
            rewarded_set,
            reward_params,
            epoch_db_id,
            end_timestamp,
        ).await {
            Ok(new_results) => {
                results.new_method = Some(new_results);
                info!("New method simulation completed successfully");
            }
            Err(e) => {
                error!("New method simulation failed: {}", e);
            }
        }

        info!(
            "Simulation completed for epoch {}. Methods run: old={}, new={}",
            current_epoch_id,
            results.old_method.is_some(),
            results.new_method.is_some()
        );

        Ok(results)
    }

    /// Run simulation using old method (24h cache-based)
    async fn run_old_method_simulation(
        &self,
        _epoch_advancer: &EpochAdvancer,
        rewarded_set: &EpochRewardedSet,
        reward_params: RewardingParams,
        epoch_db_id: i64,
        end_timestamp: i64,
    ) -> Result<MethodResults, RewardingError> {
        debug!("Running old method simulation (24h cache-based)");

        // Get 24h performance data using existing cache-based method
        let mixnode_reliabilities = self.storage
            .get_all_avg_mix_reliability_in_last_24hr(end_timestamp)
            .await
            .map_err(|e| RewardingError::DatabaseError { source: e.into() })?;

        let gateway_reliabilities = self.storage
            .get_all_avg_gateway_reliability_in_last_24hr(end_timestamp)
            .await
            .map_err(|e| RewardingError::DatabaseError { source: e.into() })?;

        // Convert to performance map
        let mut performance_map = HashMap::new();
        
        for mix_reliability in mixnode_reliabilities {
            performance_map.insert(
                mix_reliability.mix_id(), 
                Performance::from_percentage_value(mix_reliability.value() as u64).unwrap_or_default()
            );
        }

        for gateway_reliability in gateway_reliabilities {
            performance_map.insert(
                gateway_reliability.node_id(),
                Performance::from_percentage_value(gateway_reliability.value() as u64).unwrap_or_default()
            );
        }

        // Calculate rewards using old method logic
        let rewarded_nodes = self.calculate_rewards_for_nodes(
            rewarded_set,
            reward_params,
            &performance_map,
        );

        // Convert to simulation data structures
        let node_performance = self.convert_to_simulated_performance(
            &rewarded_nodes,
            epoch_db_id,
            "old",
        );

        let rewards = self.convert_to_simulated_rewards(
            &rewarded_nodes,
            epoch_db_id,
            "old",
        );

        // Create route analysis for old method
        let route_analysis = SimulatedRouteAnalysis {
            id: 0, // Will be set by database
            simulated_epoch_id: epoch_db_id,
            calculation_method: "old".to_string(),
            total_routes_analyzed: 0, // Old method doesn't use route data
            successful_routes: 0,
            failed_routes: 0,
            average_route_reliability: None,
            time_window_hours: 24, // Old method uses 24h
            analysis_parameters: Some("{\"method\":\"cache_based\",\"data_source\":\"status_cache\"}".to_string()),
            calculated_at: OffsetDateTime::now_utc().unix_timestamp(),
        };

        // Store results in database
        self.storage.manager
            .insert_simulated_node_performance(&node_performance)
            .await
            .map_err(|e| RewardingError::DatabaseError { source: e.into() })?;

        self.storage.manager
            .insert_simulated_rewards(&rewards)
            .await
            .map_err(|e| RewardingError::DatabaseError { source: e.into() })?;

        self.storage.manager
            .insert_simulated_route_analysis(&route_analysis)
            .await
            .map_err(|e| RewardingError::DatabaseError { source: e.into() })?;

        Ok(MethodResults {
            method_name: "old".to_string(),
            node_performance,
            rewards,
            route_analysis,
        })
    }

    /// Run simulation using new method (1h route-based)
    async fn run_new_method_simulation(
        &self,
        rewarded_set: &EpochRewardedSet,
        reward_params: RewardingParams,
        epoch_db_id: i64,
        end_timestamp: i64,
    ) -> Result<MethodResults, RewardingError> {
        debug!("Running new method simulation ({}h route-based)", self.config.new_method_time_window_hours);

        let time_window_secs = (self.config.new_method_time_window_hours as i64) * 3600;
        let start_timestamp = end_timestamp - time_window_secs;

        // Get route-based performance data using new method
        let corrected_reliabilities = self.storage.manager
            .calculate_corrected_node_reliabilities_for_interval(start_timestamp, end_timestamp)
            .await
            .map_err(|e| RewardingError::DatabaseError { source: e })?;

        // Convert to performance map
        let mut performance_map = HashMap::new();
        let mut total_routes = 0u32;
        let mut successful_routes = 0u32;
        let mut reliability_sum = 0.0;
        let mut reliability_count = 0u32;

        for node_reliability in &corrected_reliabilities {
            let total_samples = node_reliability.pos_samples_in_interval + node_reliability.neg_samples_in_interval;
            total_routes += total_samples;
            successful_routes += node_reliability.pos_samples_in_interval;
            
            if total_samples > 0 {
                reliability_sum += node_reliability.reliability;
                reliability_count += 1;
            }

            performance_map.insert(
                node_reliability.node_id,
                Performance::naive_try_from_f64(node_reliability.reliability / 100.0).unwrap_or_default()
            );
        }

        // Calculate rewards using new method logic
        let rewarded_nodes = self.calculate_rewards_for_nodes(
            rewarded_set,
            reward_params,
            &performance_map,
        );

        // Convert to simulation data structures  
        let node_performance = self.convert_to_simulated_performance(
            &rewarded_nodes,
            epoch_db_id,
            "new",
        );

        let rewards = self.convert_to_simulated_rewards(
            &rewarded_nodes,
            epoch_db_id,
            "new",
        );

        // Create route analysis for new method
        let route_analysis = SimulatedRouteAnalysis {
            id: 0, // Will be set by database
            simulated_epoch_id: epoch_db_id,
            calculation_method: "new".to_string(),
            total_routes_analyzed: total_routes,
            successful_routes,
            failed_routes: total_routes - successful_routes,
            average_route_reliability: if reliability_count > 0 {
                Some(reliability_sum / reliability_count as f64)
            } else {
                None
            },
            time_window_hours: self.config.new_method_time_window_hours,
            analysis_parameters: Some(format!(
                "{{\"method\":\"route_based\",\"time_window_hours\":{},\"corrected_routes\":{}}}",
                self.config.new_method_time_window_hours,
                corrected_reliabilities.len()
            )),
            calculated_at: OffsetDateTime::now_utc().unix_timestamp(),
        };

        // Store results in database
        self.storage.manager
            .insert_simulated_node_performance(&node_performance)
            .await
            .map_err(|e| RewardingError::DatabaseError { source: e.into() })?;

        self.storage.manager
            .insert_simulated_rewards(&rewards)
            .await
            .map_err(|e| RewardingError::DatabaseError { source: e.into() })?;

        self.storage.manager
            .insert_simulated_route_analysis(&route_analysis)
            .await
            .map_err(|e| RewardingError::DatabaseError { source: e.into() })?;

        Ok(MethodResults {
            method_name: "new".to_string(),
            node_performance,
            rewards,
            route_analysis,
        })
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

    /// Convert rewarded nodes to simulated performance records
    fn convert_to_simulated_performance(
        &self,
        rewarded_nodes: &[RewardedNodeWithParams],
        epoch_db_id: i64,
        method: &str,
    ) -> Vec<SimulatedNodePerformance> {
        let now = OffsetDateTime::now_utc().unix_timestamp();
        
        rewarded_nodes
            .iter()
            .map(|node| SimulatedNodePerformance {
                id: 0, // Will be set by database
                simulated_epoch_id: epoch_db_id,
                node_id: node.node_id,
                node_type: "unknown".to_string(), // TODO: Determine from rewarded set position
                identity_key: None, // TODO: Look up from storage if needed
                reliability_score: node.params.performance.naive_to_f64() * 100.0,
                positive_samples: 0, // TODO: Extract from calculation if available
                negative_samples: 0, // TODO: Extract from calculation if available  
                final_fail_sequence: 0, // TODO: Extract from calculation if available
                work_factor: Some(node.params.work_factor.naive_to_f64()),
                calculation_method: method.to_string(),
                calculated_at: now,
            })
            .collect()
    }

    /// Convert rewarded nodes to simulated reward records
    fn convert_to_simulated_rewards(
        &self,
        rewarded_nodes: &[RewardedNodeWithParams],
        epoch_db_id: i64,
        method: &str,
    ) -> Vec<SimulatedReward> {
        let now = OffsetDateTime::now_utc().unix_timestamp();
        
        rewarded_nodes
            .iter()
            .map(|node| SimulatedReward {
                id: 0, // Will be set by database
                simulated_epoch_id: epoch_db_id,
                node_id: node.node_id,
                node_type: "unknown".to_string(), // TODO: Determine from rewarded set position
                calculated_reward_amount: 0.0, // TODO: Calculate actual reward amount
                reward_currency: "nym".to_string(),
                performance_component: node.params.performance.naive_to_f64() * 100.0,
                work_component: node.params.work_factor.naive_to_f64(),
                calculation_method: method.to_string(),
                calculated_at: now,
            })
            .collect()
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
    ) -> Result<Option<SimulationResults>, RewardingError> {
        let coordinator = SimulationCoordinator::new(&self.storage, simulation_config);
        
        match coordinator.run_simulation(self, rewarded_set, reward_params, current_epoch_id).await {
            Ok(results) => {
                info!("Simulation completed successfully for epoch {}", current_epoch_id);
                Ok(Some(results))
            }
            Err(e) => {
                error!("Simulation failed for epoch {}: {}", current_epoch_id, e);
                // Don't fail the entire epoch operation due to simulation failure
                Ok(None)
            }
        }
    }
}