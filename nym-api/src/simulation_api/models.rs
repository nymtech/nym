// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! API models for simulation data responses

use crate::storage::models::{SimulatedNodePerformance, SimulatedPerformanceComparison, SimulatedRewardEpoch, SimulatedRouteAnalysis};
use nym_mixnet_contract_common::NodeId;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;

/// Response for listing simulation epochs
#[derive(Serialize, Deserialize, ToSchema, Debug, Clone)]
pub struct SimulationEpochsResponse {
    pub epochs: Vec<SimulationEpochSummary>,
    pub total_count: usize,
}

/// Summary information about a simulation epoch
#[derive(Serialize, Deserialize, ToSchema, Debug, Clone)]
pub struct SimulationEpochSummary {
    pub id: i64,
    pub epoch_id: u32,
    pub calculation_method: String,
    pub start_timestamp: i64,
    pub end_timestamp: i64,
    pub description: Option<String>,
    pub created_at: i64,
    /// Number of nodes that had performance calculated
    pub nodes_analyzed: usize,
    /// Available calculation methods for this epoch
    pub available_methods: Vec<String>,
}

impl From<SimulatedRewardEpoch> for SimulationEpochSummary {
    fn from(epoch: SimulatedRewardEpoch) -> Self {
        Self {
            id: epoch.id,
            epoch_id: epoch.epoch_id,
            calculation_method: epoch.calculation_method,
            start_timestamp: epoch.start_timestamp,
            end_timestamp: epoch.end_timestamp,
            description: epoch.description,
            created_at: epoch.created_at,
            nodes_analyzed: 0, // Will be populated by handler
            available_methods: vec![], // Will be populated by handler
        }
    }
}

/// Detailed simulation epoch with all data
#[derive(Serialize, Deserialize, ToSchema, Debug, Clone)]
pub struct SimulationEpochDetails {
    pub epoch: SimulationEpochSummary,
    pub node_performance: Vec<NodePerformanceData>,
    pub performance_comparisons: Vec<PerformanceComparisonData>,
    pub route_analysis: Vec<RouteAnalysisData>,
}

/// Node performance data for API responses
#[derive(Serialize, Deserialize, ToSchema, Debug, Clone)]
pub struct NodePerformanceData {
    pub node_id: NodeId,
    pub node_type: String,
    pub identity_key: Option<String>,
    pub reliability_score: f64,
    pub positive_samples: u32,
    pub negative_samples: u32,
    pub work_factor: Option<f64>,
    pub calculation_method: String,
    pub calculated_at: i64,
}

impl From<SimulatedNodePerformance> for NodePerformanceData {
    fn from(perf: SimulatedNodePerformance) -> Self {
        Self {
            node_id: perf.node_id,
            node_type: perf.node_type,
            identity_key: perf.identity_key,
            reliability_score: perf.reliability_score,
            positive_samples: perf.positive_samples,
            negative_samples: perf.negative_samples,
            work_factor: perf.work_factor,
            calculation_method: perf.calculation_method,
            calculated_at: perf.calculated_at,
        }
    }
}

/// Performance comparison data for API responses
#[derive(Serialize, Deserialize, ToSchema, Debug, Clone)]
pub struct PerformanceComparisonData {
    pub node_id: NodeId,
    pub node_type: String,
    pub performance_score: f64,
    pub work_factor: f64,
    pub calculation_method: String,
    pub positive_samples: Option<i64>,
    pub negative_samples: Option<i64>,
    pub route_success_rate: Option<f64>,
    pub calculated_at: i64,
}

impl From<SimulatedPerformanceComparison> for PerformanceComparisonData {
    fn from(comparison: SimulatedPerformanceComparison) -> Self {
        Self {
            node_id: comparison.node_id,
            node_type: comparison.node_type,
            performance_score: comparison.performance_score,
            work_factor: comparison.work_factor,
            calculation_method: comparison.calculation_method,
            positive_samples: comparison.positive_samples,
            negative_samples: comparison.negative_samples,
            route_success_rate: comparison.route_success_rate,
            calculated_at: comparison.calculated_at,
        }
    }
}

/// Route analysis data for API responses
#[derive(Serialize, Deserialize, ToSchema, Debug, Clone)]
pub struct RouteAnalysisData {
    pub calculation_method: String,
    pub total_routes_analyzed: u32,
    pub successful_routes: u32,
    pub failed_routes: u32,
    pub average_route_reliability: Option<f64>,
    pub time_window_hours: u32,
    pub analysis_parameters: Option<String>,
    pub calculated_at: i64,
}

impl From<SimulatedRouteAnalysis> for RouteAnalysisData {
    fn from(analysis: SimulatedRouteAnalysis) -> Self {
        Self {
            calculation_method: analysis.calculation_method,
            total_routes_analyzed: analysis.total_routes_analyzed,
            successful_routes: analysis.successful_routes,
            failed_routes: analysis.failed_routes,
            average_route_reliability: analysis.average_route_reliability,
            time_window_hours: analysis.time_window_hours,
            analysis_parameters: analysis.analysis_parameters,
            calculated_at: analysis.calculated_at,
        }
    }
}

/// Comparison between old and new methods for a specific epoch
#[derive(Serialize, Deserialize, ToSchema, Debug, Clone)]
pub struct MethodComparisonResponse {
    pub epoch_id: u32,
    pub simulation_epoch_id: i64,
    pub node_comparisons: Vec<NodeMethodComparison>,
    pub summary_statistics: ComparisonSummaryStats,
    pub route_analysis_comparison: RouteAnalysisComparison,
}

/// Comparison data for a single node between old and new methods
#[derive(Serialize, Deserialize, ToSchema, Debug, Clone)]
pub struct NodeMethodComparison {
    pub node_id: NodeId,
    pub node_type: String,
    pub identity_key: Option<String>,
    pub old_method: Option<NodePerformanceData>,
    pub new_method: Option<NodePerformanceData>,
    pub reliability_difference: Option<f64>, // new - old
    pub performance_delta_percentage: Option<f64>, // (new - old) / old * 100
    pub ranking_old_method: Option<i64>,
    pub ranking_new_method: Option<i64>,
    pub ranking_delta: Option<i64>, // new ranking - old ranking (negative is improvement)
}

/// Summary statistics comparing old vs new methods
#[derive(Serialize, Deserialize, ToSchema, Debug, Clone)]
pub struct ComparisonSummaryStats {
    pub total_nodes_compared: usize,
    pub nodes_improved: usize,      // nodes with better performance in new method
    pub nodes_degraded: usize,      // nodes with worse performance in new method
    pub nodes_unchanged: usize,     // nodes with same performance
    pub average_reliability_old: f64,
    pub average_reliability_new: f64,
    pub median_reliability_old: f64,
    pub median_reliability_new: f64,
    pub reliability_std_dev_old: f64,
    pub reliability_std_dev_new: f64,
    pub max_improvement: f64,       // highest positive delta
    pub max_degradation: f64,       // highest negative delta
}

/// Comparison of route analysis between methods
#[derive(Serialize, Deserialize, ToSchema, Debug, Clone)]
pub struct RouteAnalysisComparison {
    pub old_method: Option<RouteAnalysisData>,
    pub new_method: Option<RouteAnalysisData>,
    pub time_window_difference_hours: i32, // new - old
    pub route_coverage_difference: i32,    // new total routes - old total routes
    pub success_rate_difference: Option<f64>, // new success rate - old success rate
}

/// Export format options
#[derive(Serialize, Deserialize, ToSchema, Debug, Clone)]
pub enum ExportFormat {
    #[serde(rename = "json")]
    Json,
    #[serde(rename = "csv")]
    Csv,
}

/// Query parameters for simulation listings
#[derive(Deserialize, ToSchema, Debug, utoipa::IntoParams)]
pub struct SimulationListQuery {
    /// Limit number of results (default: 50, max: 1000)
    pub limit: Option<usize>,
    /// Offset for pagination (default: 0)
    pub offset: Option<usize>,
}

/// Query parameters for node-specific performance comparison
#[derive(Deserialize, ToSchema, Debug, utoipa::IntoParams)]
pub struct NodeComparisonQuery {
    /// Specific node ID to analyze
    pub node_id: Option<NodeId>,
    /// Node type filter (mixnode, gateway)
    pub node_type: Option<String>,
    /// Minimum reliability difference threshold for filtering
    pub min_delta: Option<f64>,
    /// Maximum reliability difference threshold for filtering
    pub max_delta: Option<f64>,
}

/// Error response for simulation API
#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct SimulationApiError {
    pub error: String,
    pub details: Option<String>,
    pub timestamp: i64,
}

impl SimulationApiError {
    pub fn new(error: &str) -> Self {
        Self {
            error: error.to_string(),
            details: None,
            timestamp: OffsetDateTime::now_utc().unix_timestamp(),
        }
    }

    pub fn with_details(error: &str, details: &str) -> Self {
        Self {
            error: error.to_string(),
            details: Some(details.to_string()),
            timestamp: OffsetDateTime::now_utc().unix_timestamp(),
        }
    }
}