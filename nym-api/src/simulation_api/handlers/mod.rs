// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Handlers for simulation API endpoints

use crate::simulation_api::models::{
    SimulationApiError, SimulationEpochDetails, SimulationEpochSummary, SimulationEpochsResponse,
    SimulationListQuery, NodeComparisonQuery, MethodComparisonResponse, NodeMethodComparison,
    ComparisonSummaryStats, RouteAnalysisComparison, NodePerformanceData, PerformanceComparisonData,
    RouteAnalysisData, ExportFormat,
};
use crate::storage::models::SimulatedPerformanceRanking;
use crate::support::http::state::AppState;
use crate::support::storage::NymApiStorage;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Json, Response};
use axum::routing::get;
use axum::Router;
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
        .route("/nodes/:node_id/performance", get(get_node_performance_history))
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
        .map_err(|e| to_axum_error(SimulationApiError::with_details("Database error", &e.to_string())))?;
    let available_methods = storage
        .manager
        .get_available_calculation_methods_for_epochs_batch(&epoch_ids)
        .await
        .map_err(|e| to_axum_error(SimulationApiError::with_details("Database error", &e.to_string())))?;
    
    let mut enhanced_epochs = Vec::new();
    for mut epoch in epochs {
        // Get metadata from our batch results
        epoch.nodes_analyzed = node_counts.get(&epoch.id).copied().unwrap_or(0);
        epoch.available_methods = available_methods.get(&epoch.epoch_id).cloned().unwrap_or_default();
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
    epoch_summary.available_methods = get_available_methods_for_epoch(storage, epoch_summary.epoch_id)
        .await
        .map_err(to_axum_error)?;
    
    let node_performance = get_node_performance_for_epoch(storage, params.epoch_id)
        .await
        .map_err(to_axum_error)?;
    
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
    
    // Get performance data for both methods
    let old_performance = get_performance_by_method(storage, sim_epoch.epoch_id, "old")
        .await
        .map_err(to_axum_error)?;
    let new_performance = get_performance_by_method(storage, sim_epoch.epoch_id, "new")
        .await
        .map_err(to_axum_error)?;
        
    // Get performance rankings for both methods
    let old_rankings = storage.manager
        .get_simulated_performance_rankings(params.epoch_id, Some("old"))
        .await
        .map_err(|e| to_axum_error(SimulationApiError::with_details("Database error", &e.to_string())))?;
    let new_rankings = storage.manager
        .get_simulated_performance_rankings(params.epoch_id, Some("new"))
        .await
        .map_err(|e| to_axum_error(SimulationApiError::with_details("Database error", &e.to_string())))?;
    
    // Build node comparisons
    let node_comparisons = build_node_comparisons_with_rankings(
        old_performance, 
        new_performance, 
        old_rankings,
        new_rankings,
        &query
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
            let json_data = serde_json::to_string_pretty(&epoch_details)
                .map_err(|e| to_axum_error(SimulationApiError::with_details("JSON serialization failed", &e.to_string())))?;
            
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .header("Content-Disposition", format!("attachment; filename=\"simulation_epoch_{}.json\"", params.epoch_id))
                .body(json_data.into())
                .map_err(|e| to_axum_error(SimulationApiError::with_details("Response building failed", &e.to_string())))
        }
        ExportFormat::Csv => {
            let csv_data = convert_to_csv(&epoch_details)
                .map_err(|e| to_axum_error(SimulationApiError::with_details("CSV conversion failed", &e.to_string())))?;
            
            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/csv")
                .header("Content-Disposition", format!("attachment; filename=\"simulation_epoch_{}.csv\"", params.epoch_id))
                .body(csv_data.into())
                .map_err(|e| to_axum_error(SimulationApiError::with_details("Response building failed", &e.to_string())))
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
    
    Ok(epochs.into_iter().map(SimulationEpochSummary::from).collect())
}

async fn count_nodes_for_epoch(storage: &NymApiStorage, epoch_id: i64) -> SimulationResult<usize> {
    storage
        .manager
        .count_simulated_node_performance_for_epoch(epoch_id)
        .await
        .map_err(|e| SimulationApiError::with_details("Database error", &e.to_string()))
}

async fn get_available_methods_for_epoch(storage: &NymApiStorage, epoch_id: u32) -> SimulationResult<Vec<String>> {
    storage
        .manager
        .get_available_calculation_methods_for_epoch(epoch_id)
        .await
        .map_err(|e| SimulationApiError::with_details("Database error", &e.to_string()))
}

async fn count_total_simulation_epochs(storage: &NymApiStorage, _params: &SimulationListQuery) -> SimulationResult<usize> {
    let result = sqlx::query!(
        "SELECT COUNT(*) as count FROM simulated_reward_epochs"
    )
    .fetch_one(&storage.manager.connection_pool)
    .await
    .map_err(|e| SimulationApiError::with_details("Database error", &e.to_string()))?;
    
    Ok(result.count as usize)
}

async fn get_simulation_epoch_by_id(storage: &NymApiStorage, id: i64) -> SimulationResult<Option<crate::support::storage::models::SimulatedRewardEpoch>> {
    storage
        .manager
        .get_simulated_reward_epoch(id)
        .await
        .map_err(|e| SimulationApiError::with_details("Database error", &e.to_string()))
}

async fn get_node_performance_for_epoch(storage: &NymApiStorage, epoch_id: i64) -> SimulationResult<Vec<NodePerformanceData>> {
    let performance = storage
        .manager
        .get_simulated_node_performance_for_epoch(epoch_id)
        .await
        .map_err(|e| SimulationApiError::with_details("Database error", &e.to_string()))?;
    
    Ok(performance.into_iter().map(NodePerformanceData::from).collect())
}

async fn get_performance_comparisons_for_epoch(storage: &NymApiStorage, epoch_id: i64) -> SimulationResult<Vec<PerformanceComparisonData>> {
    let comparisons = storage
        .manager
        .get_simulated_performance_comparisons_for_epoch(epoch_id)
        .await
        .map_err(|e| SimulationApiError::with_details("Database error", &e.to_string()))?;
    
    Ok(comparisons.into_iter().map(PerformanceComparisonData::from).collect())
}

async fn get_route_analysis_for_epoch(storage: &NymApiStorage, epoch_id: i64) -> SimulationResult<Vec<RouteAnalysisData>> {
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
    method: &str
) -> SimulationResult<Vec<NodePerformanceData>> {
    let performance = storage
        .manager
        .get_simulated_node_performance_by_method(epoch_id, method)
        .await
        .map_err(|e| SimulationApiError::with_details("Database error", &e.to_string()))?;
    
    Ok(performance.into_iter().map(NodePerformanceData::from).collect())
}

fn build_node_comparisons_with_rankings(
    old_performance: Vec<NodePerformanceData>,
    new_performance: Vec<NodePerformanceData>,
    old_rankings: Vec<SimulatedPerformanceRanking>,
    new_rankings: Vec<SimulatedPerformanceRanking>,
    query: &NodeComparisonQuery,
) -> Vec<NodeMethodComparison> {
    let mut old_map: HashMap<NodeId, NodePerformanceData> = old_performance
        .into_iter()
        .map(|p| (p.node_id, p))
        .collect();
    
    let mut new_map: HashMap<NodeId, NodePerformanceData> = new_performance
        .into_iter()
        .map(|p| (p.node_id, p))
        .collect();
        
    // Create ranking maps
    let old_ranking_map: HashMap<NodeId, i64> = old_rankings
        .into_iter()
        .map(|r| (r.node_id, r.performance_rank))
        .collect();
        
    let new_ranking_map: HashMap<NodeId, i64> = new_rankings
        .into_iter()
        .map(|r| (r.node_id, r.performance_rank))
        .collect();
    
    let mut comparisons = Vec::new();
    
    // Get all unique node IDs from both methods
    let mut all_node_ids: Vec<_> = old_map.keys().chain(new_map.keys()).cloned().collect();
    all_node_ids.sort();
    all_node_ids.dedup();
    
    for node_id in all_node_ids {
        let old_perf = old_map.remove(&node_id);
        let new_perf = new_map.remove(&node_id);
        
        // Apply filters
        if let Some(filter_node_id) = query.node_id {
            if node_id != filter_node_id {
                continue;
            }
        }
        
        if let Some(ref filter_node_type) = query.node_type {
            let node_type = old_perf.as_ref()
                .or(new_perf.as_ref())
                .map(|p| &p.node_type);
            if node_type != Some(filter_node_type) {
                continue;
            }
        }
        
        // Calculate differences
        let reliability_difference = match (&old_perf, &new_perf) {
            (Some(old), Some(new)) => Some(new.reliability_score - old.reliability_score),
            _ => None,
        };
        
        let performance_delta_percentage = match (&old_perf, &new_perf) {
            (Some(old), Some(new)) if old.reliability_score != 0.0 => {
                Some((new.reliability_score - old.reliability_score) / old.reliability_score * 100.0)
            }
            _ => None,
        };
        
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
        
        let node_type = old_perf.as_ref()
            .or(new_perf.as_ref())
            .map(|p| p.node_type.clone())
            .unwrap_or_else(|| "unknown".to_string());
        
        let identity_key = old_perf.as_ref()
            .or(new_perf.as_ref())
            .and_then(|p| p.identity_key.clone());
        
        // Get rankings for this node
        let ranking_old = old_ranking_map.get(&node_id).copied();
        let ranking_new = new_ranking_map.get(&node_id).copied();
        let ranking_delta = match (ranking_old, ranking_new) {
            (Some(old), Some(new)) => Some(new - old),
            _ => None,
        };
        
        comparisons.push(NodeMethodComparison {
            node_id,
            node_type,
            identity_key,
            old_method: old_perf,
            new_method: new_perf,
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
        if let Some(old) = &comparison.old_method {
            reliabilities_old.push(old.reliability_score);
        }
        if let Some(new) = &comparison.new_method {
            reliabilities_new.push(new.reliability_score);
        }
        
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
    let (median_reliability_old, reliability_std_dev_old) = calculate_median_and_std(&reliabilities_old);
    let (median_reliability_new, reliability_std_dev_new) = calculate_median_and_std(&reliabilities_new);
    
    ComparisonSummaryStats {
        total_nodes_compared: comparisons.len(),
        nodes_improved: improvements,
        nodes_degraded: degradations,
        nodes_unchanged: unchanged,
        average_reliability_old,
        average_reliability_new,
        median_reliability_old,
        median_reliability_new,
        reliability_std_dev_old,
        reliability_std_dev_new,
        max_improvement,
        max_degradation,
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
    let variance = values.iter()
        .map(|x| (x - mean).powi(2))
        .sum::<f64>() / values.len() as f64;
    let std_dev = variance.sqrt();
    
    (median, std_dev)
}

async fn get_route_analysis_comparison(
    storage: &NymApiStorage,
    epoch_id: u32,
) -> SimulationResult<RouteAnalysisComparison> {
    let old_analysis = storage
        .manager
        .get_simulated_route_analysis_by_method(epoch_id, "old")
        .await
        .map_err(|e| SimulationApiError::with_details("Database error", &e.to_string()))?
        .map(RouteAnalysisData::from);
    
    let new_analysis = storage
        .manager
        .get_simulated_route_analysis_by_method(epoch_id, "new")
        .await
        .map_err(|e| SimulationApiError::with_details("Database error", &e.to_string()))?
        .map(RouteAnalysisData::from);
    
    let time_window_difference_hours = match (&old_analysis, &new_analysis) {
        (Some(old), Some(new)) => new.time_window_hours as i32 - old.time_window_hours as i32,
        _ => 0,
    };
    
    let route_coverage_difference = match (&old_analysis, &new_analysis) {
        (Some(old), Some(new)) => new.total_routes_analyzed as i32 - old.total_routes_analyzed as i32,
        _ => 0,
    };
    
    let success_rate_difference = match (&old_analysis, &new_analysis) {
        (Some(old), Some(new)) => {
            let old_rate = old.successful_routes as f64 / old.total_routes_analyzed as f64;
            let new_rate = new.successful_routes as f64 / new.total_routes_analyzed as f64;
            Some(new_rate - old_rate)
        }
        _ => None,
    };
    
    Ok(RouteAnalysisComparison {
        old_method: old_analysis,
        new_method: new_analysis,
        time_window_difference_hours,
        route_coverage_difference,
        success_rate_difference,
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
    epoch_summary.available_methods = get_available_methods_for_epoch(storage, epoch_summary.epoch_id).await?;
    
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
    csv.push_str("data_type,node_id,node_type,reliability_score,reward_amount,calculation_method\n");
    
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
            comparison.node_id, comparison.node_type, "", comparison.performance_score, comparison.calculation_method
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
    
    Ok(performance.into_iter().map(NodePerformanceData::from).collect())
}

fn to_axum_error(error: SimulationApiError) -> (StatusCode, Json<SimulationApiError>) {
    (StatusCode::INTERNAL_SERVER_ERROR, Json(error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation_api::models::{NodePerformanceData, NodeMethodComparison};
    
    fn create_test_performance_data(node_id: NodeId, reliability: f64, method: &str) -> NodePerformanceData {
        NodePerformanceData {
            node_id,
            node_type: "mixnode".to_string(),
            identity_key: Some("test_key".to_string()),
            reliability_score: reliability,
            positive_samples: 100,
            negative_samples: 10,
            work_factor: Some(1.0),
            calculation_method: method.to_string(),
            calculated_at: 1234567890,
        }
    }

    #[test]
    fn test_calculate_summary_statistics_basic() {
        let comparisons = vec![
            NodeMethodComparison {
                node_id: 1,
                node_type: "mixnode".to_string(),
                identity_key: Some("key1".to_string()),
                old_method: Some(create_test_performance_data(1, 80.0, "old")),
                new_method: Some(create_test_performance_data(1, 90.0, "new")),
                reliability_difference: Some(10.0),
                performance_delta_percentage: Some(12.5),
                ranking_old_method: Some(2),
                ranking_new_method: Some(1),
                ranking_delta: Some(-1),
            },
            NodeMethodComparison {
                node_id: 2,
                node_type: "mixnode".to_string(),
                identity_key: Some("key2".to_string()),
                old_method: Some(create_test_performance_data(2, 70.0, "old")),
                new_method: Some(create_test_performance_data(2, 65.0, "new")),
                reliability_difference: Some(-5.0),
                performance_delta_percentage: Some(-7.14),
                ranking_old_method: Some(1),
                ranking_new_method: Some(2),
                ranking_delta: Some(1),
            },
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
        let old_performance = vec![
            create_test_performance_data(1, 80.0, "old"),
            create_test_performance_data(2, 70.0, "old"),
        ];
        
        let new_performance = vec![
            create_test_performance_data(1, 90.0, "new"),
            create_test_performance_data(3, 85.0, "new"), // New node not in old
        ];

        let query = NodeComparisonQuery {
            node_id: None,
            node_type: None,
            min_delta: None,
            max_delta: None,
        };

        // Create test rankings
        let old_rankings = vec![
            SimulatedPerformanceRanking {
                id: 1,
                simulated_epoch_id: 1,
                node_id: 2,
                calculation_method: "old".to_string(),
                performance_rank: 1,
                performance_percentile: 100.0,
                calculated_at: 1234567890,
            },
            SimulatedPerformanceRanking {
                id: 2,
                simulated_epoch_id: 1,
                node_id: 1,
                calculation_method: "old".to_string(),
                performance_rank: 2,
                performance_percentile: 0.0,
                calculated_at: 1234567890,
            },
        ];
        
        let new_rankings = vec![
            SimulatedPerformanceRanking {
                id: 3,
                simulated_epoch_id: 1,
                node_id: 1,
                calculation_method: "new".to_string(),
                performance_rank: 1,
                performance_percentile: 66.67,
                calculated_at: 1234567890,
            },
            SimulatedPerformanceRanking {
                id: 4,
                simulated_epoch_id: 1,
                node_id: 3,
                calculation_method: "new".to_string(),
                performance_rank: 2,
                performance_percentile: 33.33,
                calculated_at: 1234567890,
            },
            SimulatedPerformanceRanking {
                id: 5,
                simulated_epoch_id: 1,
                node_id: 2,
                calculation_method: "new".to_string(),
                performance_rank: 3,
                performance_percentile: 0.0,
                calculated_at: 1234567890,
            },
        ];

        let comparisons = build_node_comparisons_with_rankings(
            old_performance, 
            new_performance, 
            old_rankings,
            new_rankings,
            &query
        );

        assert_eq!(comparisons.len(), 3); // Nodes 1, 2, 3
        
        // Find node 1 comparison
        let node1_comparison = comparisons.iter().find(|c| c.node_id == 1).unwrap();
        assert!(node1_comparison.old_method.is_some());
        assert!(node1_comparison.new_method.is_some());
        assert_eq!(node1_comparison.reliability_difference, Some(10.0));
        assert_eq!(node1_comparison.performance_delta_percentage, Some(12.5));

        // Find node 2 comparison (only in old)
        let node2_comparison = comparisons.iter().find(|c| c.node_id == 2).unwrap();
        assert!(node2_comparison.old_method.is_some());
        assert!(node2_comparison.new_method.is_none());
        assert_eq!(node2_comparison.reliability_difference, None);

        // Find node 3 comparison (only in new)
        let node3_comparison = comparisons.iter().find(|c| c.node_id == 3).unwrap();
        assert!(node3_comparison.old_method.is_none());
        assert!(node3_comparison.new_method.is_some());
        assert_eq!(node3_comparison.reliability_difference, None);
    }

    #[test]
    fn test_build_node_comparisons_with_filters() {
        let old_performance = vec![
            create_test_performance_data(1, 80.0, "old"),
            create_test_performance_data(2, 70.0, "old"),
        ];
        
        let new_performance = vec![
            create_test_performance_data(1, 90.0, "new"),
            create_test_performance_data(2, 75.0, "new"),
        ];

        // Test node_id filter
        let query = NodeComparisonQuery {
            node_id: Some(1),
            node_type: None,
            min_delta: None,
            max_delta: None,
        };

        // Create test rankings for filter test
        let rankings = vec![
            SimulatedPerformanceRanking {
                id: 1,
                simulated_epoch_id: 1,
                node_id: 1,
                calculation_method: "old".to_string(),
                performance_rank: 1,
                performance_percentile: 100.0,
                calculated_at: 1234567890,
            },
        ];

        let comparisons = build_node_comparisons_with_rankings(
            old_performance.clone(), 
            new_performance.clone(), 
            rankings.clone(),
            rankings.clone(),
            &query
        );
        assert_eq!(comparisons.len(), 1);
        assert_eq!(comparisons[0].node_id, 1);

        // Test min_delta filter
        let query = NodeComparisonQuery {
            node_id: None,
            node_type: None,
            min_delta: Some(8.0), // Only node 1 has +10.0 delta, node 2 has +5.0
            max_delta: None,
        };

        let comparisons = build_node_comparisons_with_rankings(
            old_performance, 
            new_performance,
            rankings.clone(),
            rankings,
            &query
        );
        assert_eq!(comparisons.len(), 1);
        assert_eq!(comparisons[0].node_id, 1);
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
                create_test_performance_data(1, 80.0, "old"),
                create_test_performance_data(1, 90.0, "new"),
            ],
            performance_comparisons: vec![
                PerformanceComparisonData {
                    node_id: 1,
                    node_type: "mixnode".to_string(),
                    performance_score: 80.0,
                    work_factor: 10.0,
                    calculation_method: "old".to_string(),
                    positive_samples: Some(100),
                    negative_samples: Some(20),
                    route_success_rate: Some(80.0),
                    calculated_at: 1234567890,
                },
            ],
            route_analysis: vec![],
        };

        let csv = convert_to_csv(&details).unwrap();

        println!("CSV: {}", csv);
        
        assert!(csv.contains("data_type,node_id,node_type,reliability_score,reward_amount,calculation_method"));
        assert!(csv.contains("performance,1,mixnode,80,"));
        assert!(csv.contains("performance,1,mixnode,90,"));
    }
}