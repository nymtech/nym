// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Handlers for simulation API endpoints

use crate::simulation_api::models::{
    ComparisonSummaryStats, ExportFormat, MethodComparisonResponse, NodeComparisonQuery,
    NodeMethodComparison, NodePerformanceData, PerformanceComparisonData, RouteAnalysisComparison,
    RouteAnalysisData, SimulationApiError, SimulationEpochDetails, SimulationEpochSummary,
    SimulationEpochsResponse, SimulationListQuery,
};
use crate::support::http::state::AppState;
use crate::support::storage::NymApiStorage;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Json, Response};
use axum::routing::get;
use axum::Router;
use nym_contracts_common::NaiveFloat as _;
use nym_http_api_common::{FormattedResponse, OutputParams};
use nym_mixnet_contract_common::NodeId;
use serde::Deserialize;
use std::collections::HashMap;
use utoipa::IntoParams;

type SimulationResult<T> = Result<T, SimulationApiError>;
type AxumResult<T> = Result<T, (StatusCode, Json<SimulationApiError>)>;

/// Create the simulation API router
pub(crate) fn simulation_routes() -> Router<AppState> {
    Router::new()
        .route("/epochs", get(list_simulation_epochs))
        .route("/epochs/:epoch_id", get(get_simulation_epoch_details))
        .route("/epochs/:epoch_id/comparison", get(compare_methods))
        .route("/epochs/:epoch_id/export", get(export_simulation_data))
        .route(
            "/nodes/:node_id/performance",
            get(get_node_performance_history),
        )
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Path)]
struct EpochPathParam {
    epoch_id: i64,
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Path)]
struct NodePathParam {
    node_id: NodeId,
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct ExportQuery {
    format: Option<ExportFormat>,
}

/// List all simulation epochs with optional filtering
#[utoipa::path(
    tag = "Simulation",
    get,
    path = "/epochs",
    context_path = "/v1/simulation",
    responses(
        (status = 200, description = "List of simulation epochs", body = SimulationEpochsResponse),
        (status = 500, description = "Internal server error", body = SimulationApiError)
    ),
    params(SimulationListQuery, OutputParams)
)]
async fn list_simulation_epochs(
    Query(params): Query<SimulationListQuery>,
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<SimulationEpochsResponse>> {
    let storage = state.storage();
    let output = output.output.unwrap_or_default();

    // Apply defaults and validation
    let limit = params.limit.unwrap_or(50).min(1000);
    let offset = params.offset.unwrap_or(0);

    let epochs = get_simulation_epochs_with_filters(storage, &params, limit, offset)
        .await
        .map_err(to_axum_error)?;

    // Enhance epochs with additional metadata using batch operations
    let epoch_db_ids: Vec<i64> = epochs.iter().map(|e| e.id).collect();
    let epoch_ids: Vec<u32> = epochs.iter().map(|e| e.epoch_id).collect();

    // Batch fetch node counts and available methods for all epochs
    let node_counts = storage
        .manager
        .count_simulated_node_performance_for_epochs_batch(&epoch_db_ids)
        .await
        .map_err(|e| {
            to_axum_error(SimulationApiError::with_details(
                "Database error",
                &e.to_string(),
            ))
        })?;
    let available_methods = storage
        .manager
        .get_available_calculation_methods_for_epochs_batch(&epoch_ids)
        .await
        .map_err(|e| {
            to_axum_error(SimulationApiError::with_details(
                "Database error",
                &e.to_string(),
            ))
        })?;

    let mut enhanced_epochs = Vec::new();
    for mut epoch in epochs {
        // Get metadata from our batch results
        epoch.nodes_analyzed = node_counts.get(&epoch.id).copied().unwrap_or(0);
        epoch.available_methods = available_methods
            .get(&epoch.epoch_id)
            .cloned()
            .unwrap_or_default();
        enhanced_epochs.push(epoch);
    }

    let total_count = count_total_simulation_epochs(storage, &params)
        .await
        .map_err(to_axum_error)?;

    let response = SimulationEpochsResponse {
        epochs: enhanced_epochs,
        total_count,
    };

    Ok(output.to_response(response))
}

/// Get detailed simulation data for a specific epoch
#[utoipa::path(
    tag = "Simulation",
    get,
    path = "/epochs/{epoch_id}",
    context_path = "/v1/simulation",
    responses(
        (status = 200, description = "Detailed simulation epoch data", body = SimulationEpochDetails),
        (status = 404, description = "Simulation epoch not found", body = SimulationApiError),
        (status = 500, description = "Internal server error", body = SimulationApiError)
    ),
    params(
        ("epoch_id" = i64, Path, description = "Simulation epoch ID"),
        OutputParams
    )
)]
async fn get_simulation_epoch_details(
    Path(params): Path<EpochPathParam>,
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<SimulationEpochDetails>> {
    let storage = state.storage();
    let output = output.output.unwrap_or_default();

    let epoch = get_simulation_epoch_by_id(storage, params.epoch_id)
        .await
        .map_err(to_axum_error)?
        .ok_or_else(|| to_axum_error(SimulationApiError::new("Simulation epoch not found")))?;

    let mut epoch_summary = SimulationEpochSummary::from(epoch);
    epoch_summary.nodes_analyzed = count_nodes_for_epoch(storage, params.epoch_id)
        .await
        .map_err(to_axum_error)?;
    epoch_summary.available_methods =
        get_available_methods_for_epoch(storage, epoch_summary.epoch_id)
            .await
            .map_err(to_axum_error)?;

    let mut node_performance = get_node_performance_for_epoch(storage, params.epoch_id)
        .await
        .map_err(to_axum_error)?;
    
    // Populate production performance from node annotations cache
    if let Some(node_annotations) = state.node_status_cache.node_annotations().await {
        for perf in &mut node_performance {
            if let Some(annotation) = node_annotations.get(&perf.node_id) {
                perf.production_performance = Some(annotation.last_24h_performance.naive_to_f64() * 100.0);
            }
        }
    }

    let performance_comparisons = get_performance_comparisons_for_epoch(storage, params.epoch_id)
        .await
        .map_err(to_axum_error)?;

    let route_analysis = get_route_analysis_for_epoch(storage, params.epoch_id)
        .await
        .map_err(to_axum_error)?;

    let details = SimulationEpochDetails {
        epoch: epoch_summary,
        node_performance,
        performance_comparisons,
        route_analysis,
    };

    Ok(output.to_response(details))
}

/// Compare old vs new methods for a specific epoch
#[utoipa::path(
    tag = "Simulation",
    get,
    path = "/epochs/{epoch_id}/comparison",
    context_path = "/v1/simulation",
    responses(
        (status = 200, description = "Method comparison results", body = MethodComparisonResponse),
        (status = 404, description = "Simulation epoch not found", body = SimulationApiError),
        (status = 500, description = "Internal server error", body = SimulationApiError)
    ),
    params(
        ("epoch_id" = i64, Path, description = "Simulation epoch ID"),
        NodeComparisonQuery,
        OutputParams
    )
)]
async fn compare_methods(
    Path(params): Path<EpochPathParam>,
    Query(query): Query<NodeComparisonQuery>,
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<MethodComparisonResponse>> {
    let storage = state.storage();
    let output = output.output.unwrap_or_default();

    // Get simulation epoch to extract actual epoch_id
    let sim_epoch = get_simulation_epoch_by_id(storage, params.epoch_id)
        .await
        .map_err(to_axum_error)?
        .ok_or_else(|| to_axum_error(SimulationApiError::new("Simulation epoch not found")))?;

    // Get simulation performance data
    let mut performance_data = get_performance_by_method(storage, sim_epoch.epoch_id, "new")
        .await
        .map_err(to_axum_error)?;
    
    // Populate production performance from node annotations cache
    let node_annotations = state
        .node_status_cache
        .node_annotations()
        .await
        .ok_or_else(|| to_axum_error(SimulationApiError::new("Node annotations not available")))?;
    
    for perf in &mut performance_data {
        if let Some(annotation) = node_annotations.get(&perf.node_id) {
            perf.production_performance = Some(annotation.last_24h_performance.naive_to_f64() * 100.0);
        }
    }

    // Build node comparisons from the single performance dataset
    let node_comparisons = build_node_comparisons_from_single_dataset(
        performance_data,
        &query,
    );

    // Calculate summary statistics
    let summary_statistics = calculate_summary_statistics(&node_comparisons);

    // Get route analysis comparison
    let route_analysis_comparison = get_route_analysis_comparison(storage, sim_epoch.epoch_id)
        .await
        .map_err(to_axum_error)?;

    let comparison = MethodComparisonResponse {
        epoch_id: sim_epoch.epoch_id,
        simulation_epoch_id: params.epoch_id,
        node_comparisons,
        summary_statistics,
        route_analysis_comparison,
    };

    Ok(output.to_response(comparison))
}

/// Export simulation data in various formats
#[utoipa::path(
    tag = "Simulation",
    get,
    path = "/epochs/{epoch_id}/export",
    context_path = "/v1/simulation",
    responses(
        (status = 200, description = "Exported simulation data"),
        (status = 404, description = "Simulation epoch not found", body = SimulationApiError),
        (status = 500, description = "Internal server error", body = SimulationApiError)
    ),
    params(
        ("epoch_id" = i64, Path, description = "Simulation epoch ID"),
        ExportQuery
    )
)]
async fn export_simulation_data(
    Path(params): Path<EpochPathParam>,
    Query(query): Query<ExportQuery>,
    State(state): State<AppState>,
) -> Result<Response, (StatusCode, Json<SimulationApiError>)> {
    let storage = state.storage();
    let format = query.format.unwrap_or(ExportFormat::Json);

    // Get detailed simulation data
    let epoch_details = get_simulation_epoch_details_internal(storage, params.epoch_id)
        .await
        .map_err(to_axum_error)?
        .ok_or_else(|| to_axum_error(SimulationApiError::new("Simulation epoch not found")))?;

    match format {
        ExportFormat::Json => {
            let json_data = serde_json::to_string_pretty(&epoch_details).map_err(|e| {
                to_axum_error(SimulationApiError::with_details(
                    "JSON serialization failed",
                    &e.to_string(),
                ))
            })?;

            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .header(
                    "Content-Disposition",
                    format!(
                        "attachment; filename=\"simulation_epoch_{}.json\"",
                        params.epoch_id
                    ),
                )
                .body(json_data.into())
                .map_err(|e| {
                    to_axum_error(SimulationApiError::with_details(
                        "Response building failed",
                        &e.to_string(),
                    ))
                })
        }
        ExportFormat::Csv => {
            let csv_data = convert_to_csv(&epoch_details).map_err(|e| {
                to_axum_error(SimulationApiError::with_details(
                    "CSV conversion failed",
                    &e.to_string(),
                ))
            })?;

            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/csv")
                .header(
                    "Content-Disposition",
                    format!(
                        "attachment; filename=\"simulation_epoch_{}.csv\"",
                        params.epoch_id
                    ),
                )
                .body(csv_data.into())
                .map_err(|e| {
                    to_axum_error(SimulationApiError::with_details(
                        "Response building failed",
                        &e.to_string(),
                    ))
                })
        }
    }
}

/// Get performance history for a specific node across simulation epochs
#[utoipa::path(
    tag = "Simulation",
    get,
    path = "/nodes/{node_id}/performance",
    context_path = "/v1/simulation",
    responses(
        (status = 200, description = "Node performance history", body = Vec<NodePerformanceData>),
        (status = 500, description = "Internal server error", body = SimulationApiError)
    ),
    params(
        ("node_id" = NodeId, Path, description = "Node ID"),
        OutputParams
    )
)]
async fn get_node_performance_history(
    Path(params): Path<NodePathParam>,
    Query(output): Query<OutputParams>,
    State(state): State<AppState>,
) -> AxumResult<FormattedResponse<Vec<NodePerformanceData>>> {
    let storage = state.storage();
    let output = output.output.unwrap_or_default();

    let performance_history = get_node_performance_history_internal(storage, params.node_id)
        .await
        .map_err(to_axum_error)?;

    Ok(output.to_response(performance_history))
}

// Helper functions (implementations would be added here)

async fn get_simulation_epochs_with_filters(
    storage: &NymApiStorage,
    _params: &SimulationListQuery,
    limit: usize,
    offset: usize,
) -> SimulationResult<Vec<SimulationEpochSummary>> {
    let limit_i64 = limit as i64;
    let offset_i64 = offset as i64;

    let epochs = sqlx::query_as!(
        crate::support::storage::models::SimulatedRewardEpoch,
        "SELECT id as \"id!\", epoch_id as \"epoch_id!: u32\", calculation_method as \"calculation_method!\", 
               start_timestamp as \"start_timestamp!\", end_timestamp as \"end_timestamp!\", 
               description, created_at as \"created_at!\"
         FROM simulated_reward_epochs 
         ORDER BY created_at DESC 
         LIMIT ? OFFSET ?",
        limit_i64,
        offset_i64
    )
    .fetch_all(&storage.manager.connection_pool)
    .await
    .map_err(|e| SimulationApiError::with_details("Database error", &e.to_string()))?;

    Ok(epochs
        .into_iter()
        .map(SimulationEpochSummary::from)
        .collect())
}

async fn count_nodes_for_epoch(storage: &NymApiStorage, epoch_id: i64) -> SimulationResult<usize> {
    storage
        .manager
        .count_simulated_node_performance_for_epoch(epoch_id)
        .await
        .map_err(|e| SimulationApiError::with_details("Database error", &e.to_string()))
}

async fn get_available_methods_for_epoch(
    storage: &NymApiStorage,
    epoch_id: u32,
) -> SimulationResult<Vec<String>> {
    storage
        .manager
        .get_available_calculation_methods_for_epoch(epoch_id)
        .await
        .map_err(|e| SimulationApiError::with_details("Database error", &e.to_string()))
}

async fn count_total_simulation_epochs(
    storage: &NymApiStorage,
    _params: &SimulationListQuery,
) -> SimulationResult<usize> {
    let result = sqlx::query!("SELECT COUNT(*) as count FROM simulated_reward_epochs")
        .fetch_one(&storage.manager.connection_pool)
        .await
        .map_err(|e| SimulationApiError::with_details("Database error", &e.to_string()))?;

    Ok(result.count as usize)
}

async fn get_simulation_epoch_by_id(
    storage: &NymApiStorage,
    id: i64,
) -> SimulationResult<Option<crate::support::storage::models::SimulatedRewardEpoch>> {
    storage
        .manager
        .get_simulated_reward_epoch(id)
        .await
        .map_err(|e| SimulationApiError::with_details("Database error", &e.to_string()))
}

async fn get_node_performance_for_epoch(
    storage: &NymApiStorage,
    epoch_id: i64,
) -> SimulationResult<Vec<NodePerformanceData>> {
    let performance = storage
        .manager
        .get_simulated_node_performance_for_epoch(epoch_id)
        .await
        .map_err(|e| SimulationApiError::with_details("Database error", &e.to_string()))?;

    Ok(performance
        .into_iter()
        .map(NodePerformanceData::from)
        .collect())
}

async fn get_performance_comparisons_for_epoch(
    storage: &NymApiStorage,
    epoch_id: i64,
) -> SimulationResult<Vec<PerformanceComparisonData>> {
    let comparisons = storage
        .manager
        .get_simulated_performance_comparisons_for_epoch(epoch_id)
        .await
        .map_err(|e| SimulationApiError::with_details("Database error", &e.to_string()))?;

    Ok(comparisons
        .into_iter()
        .map(PerformanceComparisonData::from)
        .collect())
}

async fn get_route_analysis_for_epoch(
    storage: &NymApiStorage,
    epoch_id: i64,
) -> SimulationResult<Vec<RouteAnalysisData>> {
    let analysis = storage
        .manager
        .get_simulated_route_analysis_for_epoch(epoch_id)
        .await
        .map_err(|e| SimulationApiError::with_details("Database error", &e.to_string()))?;

    Ok(analysis.into_iter().map(RouteAnalysisData::from).collect())
}

async fn get_performance_by_method(
    storage: &NymApiStorage,
    epoch_id: u32,
    method: &str,
) -> SimulationResult<Vec<NodePerformanceData>> {
    let performance = storage
        .manager
        .get_simulated_node_performance_by_method(epoch_id, method)
        .await
        .map_err(|e| SimulationApiError::with_details("Database error", &e.to_string()))?;

    Ok(performance
        .into_iter()
        .map(NodePerformanceData::from)
        .collect())
}

fn build_node_comparisons_from_single_dataset(
    performance_data: Vec<NodePerformanceData>,
    query: &NodeComparisonQuery,
) -> Vec<NodeMethodComparison> {
    let mut comparisons = Vec::new();
    
    // Calculate rankings
    let mut sorted_by_production: Vec<_> = performance_data.iter()
        .filter(|p| p.production_performance.is_some())
        .map(|p| (p.node_id, p.production_performance))
        .collect();
    sorted_by_production.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    
    let mut sorted_by_simulation: Vec<_> = performance_data.iter()
        .map(|p| (p.node_id, p.reliability_score))
        .collect();
    sorted_by_simulation.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    
    // Create ranking maps
    let production_rankings: HashMap<NodeId, i64> = sorted_by_production.iter()
        .enumerate()
        .map(|(rank, (node_id, _))| (*node_id, (rank + 1) as i64))
        .collect();
        
    let simulation_rankings: HashMap<NodeId, i64> = sorted_by_simulation.iter()
        .enumerate()
        .map(|(rank, (node_id, _))| (*node_id, (rank + 1) as i64))
        .collect();

    for perf in performance_data {
        // Apply filters
        if let Some(filter_node_id) = query.node_id {
            if perf.node_id != filter_node_id {
                continue;
            }
        }

        if let Some(ref filter_node_type) = query.node_type {
            if &perf.node_type != filter_node_type {
                continue;
            }
        }

        // Calculate differences
        let reliability_difference = perf.production_performance.map(|prod| {
            ((perf.reliability_score - prod) * 100.0).round() / 100.0
        });

        let performance_delta_percentage = perf.production_performance.and_then(|prod| {
            if prod != 0.0 {
                Some((((perf.reliability_score - prod) / prod * 100.0) * 100.0).round() / 100.0)
            } else {
                None
            }
        });

        // Apply delta filters
        if let Some(min_delta) = query.min_delta {
            if reliability_difference.map_or(true, |d| d < min_delta) {
                continue;
            }
        }

        if let Some(max_delta) = query.max_delta {
            if reliability_difference.map_or(true, |d| d > max_delta) {
                continue;
            }
        }

        // Get rankings
        let ranking_old = production_rankings.get(&perf.node_id).copied();
        let ranking_new = simulation_rankings.get(&perf.node_id).copied();
        let ranking_delta = match (ranking_old, ranking_new) {
            (Some(old), Some(new)) => Some(new - old),
            _ => None,
        };

        comparisons.push(NodeMethodComparison {
            node_id: perf.node_id,
            node_type: perf.node_type,
            identity_key: perf.identity_key,
            production_performance: perf.production_performance,
            simulated_performance: perf.reliability_score,
            positive_samples: perf.positive_samples,
            negative_samples: perf.negative_samples,
            work_factor: perf.work_factor,
            reliability_difference,
            performance_delta_percentage,
            ranking_old_method: ranking_old,
            ranking_new_method: ranking_new,
            ranking_delta,
        });
    }

    comparisons
}

fn calculate_summary_statistics(comparisons: &[NodeMethodComparison]) -> ComparisonSummaryStats {
    let mut reliabilities_old = Vec::new();
    let mut reliabilities_new = Vec::new();
    let mut improvements = 0;
    let mut degradations = 0;
    let mut unchanged = 0;
    let mut max_improvement: f64 = 0.0;
    let mut max_degradation: f64 = 0.0;

    for comparison in comparisons {
        // Collect production (old method) reliability values
        if let Some(old) = comparison.production_performance {
            reliabilities_old.push(old);
        }
        
        // Collect simulated (new method) reliability values
        reliabilities_new.push(comparison.simulated_performance);

        if let Some(diff) = comparison.reliability_difference {
            if diff > 0.001 {
                improvements += 1;
                max_improvement = max_improvement.max(diff);
            } else if diff < -0.001 {
                degradations += 1;
                max_degradation = max_degradation.min(diff); // This will be negative
            } else {
                unchanged += 1;
            }
        }
    }

    let average_reliability_old = if reliabilities_old.is_empty() {
        0.0
    } else {
        reliabilities_old.iter().sum::<f64>() / reliabilities_old.len() as f64
    };

    let average_reliability_new = if reliabilities_new.is_empty() {
        0.0
    } else {
        reliabilities_new.iter().sum::<f64>() / reliabilities_new.len() as f64
    };

    // Calculate medians and standard deviations
    let (median_reliability_old, reliability_std_dev_old) =
        calculate_median_and_std(&reliabilities_old);
    let (median_reliability_new, reliability_std_dev_new) =
        calculate_median_and_std(&reliabilities_new);

    // Calculate distribution categories
    let distribution_old = calculate_reliability_distribution(&reliabilities_old);
    let distribution_new = calculate_reliability_distribution(&reliabilities_new);

    ComparisonSummaryStats {
        total_nodes_compared: comparisons.len(),
        nodes_improved: improvements,
        nodes_degraded: degradations,
        nodes_unchanged: unchanged,
        average_reliability_old: (average_reliability_old * 100.0).round() / 100.0,
        average_reliability_new: (average_reliability_new * 100.0).round() / 100.0,
        median_reliability_old: (median_reliability_old * 100.0).round() / 100.0,
        median_reliability_new: (median_reliability_new * 100.0).round() / 100.0,
        reliability_std_dev_old: (reliability_std_dev_old * 100.0).round() / 100.0,
        reliability_std_dev_new: (reliability_std_dev_new * 100.0).round() / 100.0,
        max_improvement: (max_improvement * 100.0).round() / 100.0,
        max_degradation: (max_degradation * 100.0).round() / 100.0,
        distribution_old,
        distribution_new,
    }
}

fn calculate_median_and_std(values: &[f64]) -> (f64, f64) {
    if values.is_empty() {
        return (0.0, 0.0);
    }

    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let median = if sorted.len() % 2 == 0 {
        (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
    } else {
        sorted[sorted.len() / 2]
    };

    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / values.len() as f64;
    let std_dev = variance.sqrt();

    (median, std_dev)
}

fn calculate_reliability_distribution(reliabilities: &[f64]) -> crate::simulation_api::models::ReliabilityDistribution {
    let mut distribution = crate::simulation_api::models::ReliabilityDistribution {
        excellent: 0,
        very_good: 0,
        good: 0,
        moderate: 0,
        poor: 0,
        very_poor: 0,
    };
    
    for &reliability in reliabilities {
        match reliability {
            r if r > 95.0 => distribution.excellent += 1,
            r if r > 90.0 => distribution.very_good += 1,
            r if r > 75.0 => distribution.good += 1,
            r if r > 50.0 => distribution.moderate += 1,
            r if r > 25.0 => distribution.poor += 1,
            _ => distribution.very_poor += 1,
        }
    }
    
    distribution
}

async fn get_route_analysis_comparison(
    storage: &NymApiStorage,
    epoch_id: u32,
) -> SimulationResult<RouteAnalysisComparison> {
    // Old method (production) doesn't have route analysis data
    // Only the new method has route-level analysis
    let old_analysis = None;

    let new_analysis = storage
        .manager
        .get_simulated_route_analysis_by_method(epoch_id, "new")
        .await
        .map_err(|e| SimulationApiError::with_details("Database error", &e.to_string()))?
        .map(RouteAnalysisData::from);

    // Since old method doesn't have route analysis, we can only compare against new method data
    let time_window_difference_hours = match &new_analysis {
        Some(new) => new.time_window_hours as i32 - 24, // Old method always uses 24h
        None => 0,
    };

    let route_coverage_difference = 0; // Cannot compare route coverage
    let reliability_difference = None; // Cannot compare route reliability

    Ok(RouteAnalysisComparison {
        old_method: old_analysis,
        new_method: new_analysis,
        time_window_difference_hours,
        route_coverage_difference,
        success_rate_difference: reliability_difference,
    })
}

async fn get_simulation_epoch_details_internal(
    storage: &NymApiStorage,
    epoch_id: i64,
) -> SimulationResult<Option<SimulationEpochDetails>> {
    let epoch = match get_simulation_epoch_by_id(storage, epoch_id).await? {
        Some(epoch) => epoch,
        None => return Ok(None),
    };

    let mut epoch_summary = SimulationEpochSummary::from(epoch);
    epoch_summary.nodes_analyzed = count_nodes_for_epoch(storage, epoch_id).await?;
    epoch_summary.available_methods =
        get_available_methods_for_epoch(storage, epoch_summary.epoch_id).await?;

    let node_performance = get_node_performance_for_epoch(storage, epoch_id).await?;
    let performance_comparisons = get_performance_comparisons_for_epoch(storage, epoch_id).await?;
    let route_analysis = get_route_analysis_for_epoch(storage, epoch_id).await?;

    Ok(Some(SimulationEpochDetails {
        epoch: epoch_summary,
        node_performance,
        performance_comparisons,
        route_analysis,
    }))
}

fn convert_to_csv(details: &SimulationEpochDetails) -> Result<String, Box<dyn std::error::Error>> {
    // Simple CSV conversion
    let mut csv = String::new();

    // Header
    csv.push_str(
        "data_type,node_id,node_type,reliability_score,reward_amount,calculation_method\n",
    );

    // Performance data
    for perf in &details.node_performance {
        csv.push_str(&format!(
            "performance,{},{},{},{},{}\n",
            perf.node_id, perf.node_type, perf.reliability_score, "", perf.calculation_method
        ));
    }

    // Performance comparison data
    for comparison in &details.performance_comparisons {
        csv.push_str(&format!(
            "performance_comparison,{},{},{},{},{}\n",
            comparison.node_id,
            comparison.node_type,
            "",
            comparison.performance_score,
            comparison.calculation_method
        ));
    }

    Ok(csv)
}

async fn get_node_performance_history_internal(
    storage: &NymApiStorage,
    node_id: NodeId,
) -> SimulationResult<Vec<NodePerformanceData>> {
    let performance = storage
        .manager
        .get_simulated_node_performance_history(node_id)
        .await
        .map_err(|e| SimulationApiError::with_details("Database error", &e.to_string()))?;

    Ok(performance
        .into_iter()
        .map(NodePerformanceData::from)
        .collect())
}

fn to_axum_error(error: SimulationApiError) -> (StatusCode, Json<SimulationApiError>) {
    (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{simulation_api::models::{NodeMethodComparison, NodePerformanceData}};

    fn create_test_comparison(
        node_id: NodeId,
        production_perf: Option<f64>,
        simulated_perf: f64,
        reliability_diff: Option<f64>,
        perf_delta_pct: Option<f64>,
    ) -> NodeMethodComparison {
        NodeMethodComparison {
            node_id,
            node_type: "mixnode".to_string(),
            identity_key: Some(format!("key{}", node_id)),
            production_performance: production_perf,
            simulated_performance: simulated_perf,
            positive_samples: 100,
            negative_samples: 10,
            work_factor: Some(1.0),
            reliability_difference: reliability_diff,
            performance_delta_percentage: perf_delta_pct,
            ranking_old_method: None,
            ranking_new_method: None,
            ranking_delta: None,
        }
    }

    #[test]
    fn test_calculate_summary_statistics_basic() {
        let comparisons = vec![
            create_test_comparison(1, Some(80.0), 90.0, Some(10.0), Some(12.5)),
            create_test_comparison(2, Some(70.0), 65.0, Some(-5.0), Some(-7.14)),
        ];

        let stats = calculate_summary_statistics(&comparisons);

        assert_eq!(stats.total_nodes_compared, 2);
        assert_eq!(stats.nodes_improved, 1);
        assert_eq!(stats.nodes_degraded, 1);
        assert_eq!(stats.nodes_unchanged, 0);
        assert_eq!(stats.average_reliability_old, 75.0);
        assert_eq!(stats.average_reliability_new, 77.5);
        assert_eq!(stats.max_improvement, 10.0);
        assert_eq!(stats.max_degradation, -5.0);
        
        // Check distribution for old method (80.0 and 70.0)
        assert_eq!(stats.distribution_old.excellent, 0);
        assert_eq!(stats.distribution_old.very_good, 0);
        assert_eq!(stats.distribution_old.good, 1); // 80.0 falls in 75-90
        assert_eq!(stats.distribution_old.moderate, 1); // 70.0 falls in 50-75
        assert_eq!(stats.distribution_old.poor, 0);
        assert_eq!(stats.distribution_old.very_poor, 0);
        
        // Check distribution for new method (90.0 and 65.0)
        assert_eq!(stats.distribution_new.excellent, 0);
        assert_eq!(stats.distribution_new.very_good, 0);
        assert_eq!(stats.distribution_new.good, 1); // 90.0 falls in 75-90 (not >90)
        assert_eq!(stats.distribution_new.moderate, 1); // 65.0 falls in 50-75
        assert_eq!(stats.distribution_new.poor, 0);
        assert_eq!(stats.distribution_new.very_poor, 0);
    }

    #[test]
    fn test_calculate_summary_statistics_empty() {
        let comparisons = vec![];
        let stats = calculate_summary_statistics(&comparisons);

        assert_eq!(stats.total_nodes_compared, 0);
        assert_eq!(stats.nodes_improved, 0);
        assert_eq!(stats.nodes_degraded, 0);
        assert_eq!(stats.nodes_unchanged, 0);
        assert_eq!(stats.average_reliability_old, 0.0);
        assert_eq!(stats.average_reliability_new, 0.0);
        
        // Check empty distributions
        assert_eq!(stats.distribution_old.excellent, 0);
        assert_eq!(stats.distribution_old.very_good, 0);
        assert_eq!(stats.distribution_old.good, 0);
        assert_eq!(stats.distribution_old.moderate, 0);
        assert_eq!(stats.distribution_old.poor, 0);
        assert_eq!(stats.distribution_old.very_poor, 0);
    }

    #[test]
    fn test_calculate_median_and_std() {
        // Test with odd number of values
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let (median, std_dev) = calculate_median_and_std(&values);
        assert_eq!(median, 3.0);
        assert_eq!(std_dev, 1.4142135623730951); // √2.5

        // Test with even number of values
        let values = vec![1.0, 2.0, 3.0, 4.0];
        let (median, _) = calculate_median_and_std(&values);
        assert_eq!(median, 2.5);

        // Test with empty values
        let values = vec![];
        let (median, std_dev) = calculate_median_and_std(&values);
        assert_eq!(median, 0.0);
        assert_eq!(std_dev, 0.0);
    }

    #[test]
    fn test_build_node_comparisons() {
        let performance_data = vec![
            NodePerformanceData {
                node_id: 1,
                node_type: "mixnode".to_string(),
                identity_key: Some("test_key".to_string()),
                reliability_score: 90.0, // New method
                positive_samples: 100,
                negative_samples: 10,
                work_factor: Some(1.0),
                calculation_method: "new".to_string(),
                calculated_at: 1234567890,
                production_performance: Some(80.0), // Old method
            },
            NodePerformanceData {
                node_id: 2,
                node_type: "mixnode".to_string(),
                identity_key: Some("test_key2".to_string()),
                reliability_score: 65.0, // New method
                positive_samples: 80,
                negative_samples: 20,
                work_factor: Some(1.0),
                calculation_method: "new".to_string(),
                calculated_at: 1234567890,
                production_performance: Some(70.0), // Old method
            },
            NodePerformanceData {
                node_id: 3,
                node_type: "gateway".to_string(),
                identity_key: Some("test_key3".to_string()),
                reliability_score: 85.0, // New method
                positive_samples: 90,
                negative_samples: 15,
                work_factor: Some(1.0),
                calculation_method: "new".to_string(),
                calculated_at: 1234567890,
                production_performance: None, // No production data
            },
        ];

        let query = NodeComparisonQuery {
            node_id: None,
            node_type: None,
            min_delta: None,
            max_delta: None,
        };

        let comparisons = build_node_comparisons_from_single_dataset(
            performance_data,
            &query,
        );

        assert_eq!(comparisons.len(), 3);

        // Find node 1 comparison
        let node1_comparison = comparisons.iter().find(|c| c.node_id == 1).unwrap();
        assert_eq!(node1_comparison.production_performance, Some(80.0));
        assert_eq!(node1_comparison.simulated_performance, 90.0);
        assert_eq!(node1_comparison.reliability_difference, Some(10.0)); // 90 - 80
        assert_eq!(node1_comparison.performance_delta_percentage, Some(12.5)); // (90-80)/80 * 100
        assert_eq!(node1_comparison.ranking_old_method, Some(1)); // 80% is best among nodes with production data
        assert_eq!(node1_comparison.ranking_new_method, Some(1)); // 90% is best overall

        // Find node 2 comparison
        let node2_comparison = comparisons.iter().find(|c| c.node_id == 2).unwrap();
        assert_eq!(node2_comparison.production_performance, Some(70.0));
        assert_eq!(node2_comparison.simulated_performance, 65.0);
        assert_eq!(node2_comparison.reliability_difference, Some(-5.0)); // 65 - 70
        assert_eq!(node2_comparison.ranking_old_method, Some(2)); // 70% is second
        assert_eq!(node2_comparison.ranking_new_method, Some(3)); // 65% is worst

        // Find node 3 comparison (no production data)
        let node3_comparison = comparisons.iter().find(|c| c.node_id == 3).unwrap();
        assert_eq!(node3_comparison.production_performance, None);
        assert_eq!(node3_comparison.simulated_performance, 85.0);
        assert_eq!(node3_comparison.reliability_difference, None);
        assert_eq!(node3_comparison.ranking_old_method, None); // No production ranking
        assert_eq!(node3_comparison.ranking_new_method, Some(2)); // 85% is second best
    }


    #[test]
    fn test_convert_to_csv() {
        let details = SimulationEpochDetails {
            epoch: SimulationEpochSummary {
                id: 1,
                epoch_id: 100,
                calculation_method: "comparison".to_string(),
                start_timestamp: 1234567890,
                end_timestamp: 1234571490,
                description: Some("Test simulation".to_string()),
                created_at: 1234567890,
                nodes_analyzed: 2,
                available_methods: vec!["old".to_string(), "new".to_string()],
            },
            node_performance: vec![
                NodePerformanceData {
                    node_id: 1,
                    node_type: "mixnode".to_string(),
                    identity_key: Some("test_key".to_string()),
                    reliability_score: 90.0,
                    positive_samples: 100,
                    negative_samples: 10,
                    work_factor: Some(1.0),
                    calculation_method: "new".to_string(),
                    calculated_at: 1234567890,
                    production_performance: Some(80.0),
                },
            ],
            performance_comparisons: vec![PerformanceComparisonData {
                node_id: 1,
                node_type: "mixnode".to_string(),
                performance_score: 80.0,
                work_factor: 10.0,
                calculation_method: "old".to_string(),
                positive_samples: Some(100),
                negative_samples: Some(20),
                route_success_rate: Some(80.0),
                calculated_at: 1234567890,
            }],
            route_analysis: vec![],
        };

        let csv = convert_to_csv(&details).unwrap();

        println!("CSV: {}", csv);

        assert!(csv.contains(
            "data_type,node_id,node_type,reliability_score,reward_amount,calculation_method"
        ));
        assert!(csv.contains("performance,1,mixnode,90,")); // New method score
        assert!(csv.contains("performance_comparison,1,mixnode,,80,")); // Performance comparison score
    }
}
