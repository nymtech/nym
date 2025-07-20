use axum::{
    extract::{Path, Query, State},
    Json, Router,
};
use serde::Deserialize;
use tracing::instrument;
use utoipa::IntoParams;

use crate::http::{
    error::{HttpError, HttpResult},
    models::{DailyStats, Mixnode},
    state::AppState,
    PagedResult, Pagination,
};

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/", axum::routing::get(mixnodes))
        .route("/:mix_id", axum::routing::get(get_mixnodes))
        .route("/stats", axum::routing::get(get_stats))
}

#[utoipa::path(
    tag = "Mixnodes",
    get,
    params(
        Pagination
    ),
    path = "/v2/mixnodes",
    responses(
        (status = 200, body = PagedResult<Mixnode>)
    )
)]
#[instrument(level = tracing::Level::DEBUG, skip_all, fields(page=pagination.page, size=pagination.size))]
async fn mixnodes(
    Query(pagination): Query<Pagination>,
    State(state): State<AppState>,
) -> HttpResult<Json<PagedResult<Mixnode>>> {
    let db = state.db_pool();
    let res = state.cache().get_mixnodes_list(db).await;

    Ok(Json(PagedResult::paginate(pagination, res)))
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Path)]
struct MixIdParam {
    mix_id: String,
}

#[utoipa::path(
    tag = "Mixnodes",
    get,
    params(
        MixIdParam
    ),
    path = "/v2/mixnodes/{mix_id}",
    responses(
        (status = 200, body = Mixnode)
    )
)]
#[instrument(level = tracing::Level::DEBUG, skip_all, fields(mix_id = mix_id))]
async fn get_mixnodes(
    Path(MixIdParam { mix_id }): Path<MixIdParam>,
    State(state): State<AppState>,
) -> HttpResult<Json<Mixnode>> {
    find_mixnode_by_id(&mix_id, state.cache().get_mixnodes_list(state.db_pool()).await)
        .map(Json)
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct MixStatsQueryParams {
    offset: Option<i64>,
}

#[utoipa::path(
    tag = "Mixnodes",
    get,
    path = "/v2/mixnodes/stats",
    params(
        MixStatsQueryParams
    ),
    responses(
        (status = 200, body = Vec<DailyStats>)
    )
)]
#[instrument(level = "debug", skip(state))]
async fn get_stats(
    Query(MixStatsQueryParams { offset }): Query<MixStatsQueryParams>,
    State(state): State<AppState>,
) -> HttpResult<Json<Vec<DailyStats>>> {
    let offset = validate_offset(offset)?;
    let last_30_days = state
        .cache()
        .get_mixnode_stats(state.db_pool(), offset)
        .await;

    Ok(Json(last_30_days))
}

// Extract business logic for testing
fn find_mixnode_by_id(mix_id: &str, mixnodes: Vec<Mixnode>) -> HttpResult<Mixnode> {
    match mix_id.parse::<u32>() {
        Ok(parsed_mix_id) => {
            mixnodes
                .into_iter()
                .find(|item| item.mix_id == parsed_mix_id)
                .ok_or_else(|| HttpError::invalid_input(mix_id))
        }
        Err(_e) => Err(HttpError::invalid_input(mix_id)),
    }
}

fn validate_offset(offset: Option<i64>) -> HttpResult<usize> {
    offset
        .unwrap_or(0)
        .try_into()
        .map_err(|_| HttpError::invalid_input("Offset must be non-negative"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::models::{DailyStats, Mixnode};
    use nym_node_requests::api::v1::node::models::NodeDescription;
    
    fn create_test_mixnode(mix_id: u32, is_dp_delegatee: bool) -> Mixnode {
        Mixnode {
            mix_id,
            bonded: true,
            is_dp_delegatee,
            total_stake: 100000,
            full_details: Some(serde_json::json!({"test": "data"})),
            self_described: Some(serde_json::json!({"version": "1.0"})),
            description: NodeDescription {
                moniker: format!("Mixnode {}", mix_id),
                website: "".to_string(),
                security_contact: "".to_string(),
                details: "".to_string(),
            },
            last_updated_utc: "2024-01-20T10:00:00Z".to_string(),
        }
    }
    
    #[test]
    fn test_routes_construction() {
        let router = routes();
        // Just verify the router builds without panic
        // Actual route testing would require integration tests
        let _routes = router;
    }
    
    #[test]
    fn test_find_mixnode_by_id_success() {
        let mixnodes = vec![
            create_test_mixnode(1, false),
            create_test_mixnode(42, true),
            create_test_mixnode(100, false),
        ];
        
        let result = find_mixnode_by_id("42", mixnodes).unwrap();
        assert_eq!(result.mix_id, 42);
        assert!(result.is_dp_delegatee);
    }
    
    #[test]
    fn test_find_mixnode_by_id_not_found() {
        let mixnodes = vec![
            create_test_mixnode(1, false),
            create_test_mixnode(2, false),
        ];
        
        let result = find_mixnode_by_id("99", mixnodes);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_find_mixnode_by_id_invalid_format() {
        let mixnodes = vec![create_test_mixnode(1, false)];
        
        // Test various invalid formats
        assert!(find_mixnode_by_id("abc", mixnodes.clone()).is_err());
        assert!(find_mixnode_by_id("", mixnodes.clone()).is_err());
        assert!(find_mixnode_by_id("12.34", mixnodes.clone()).is_err());
        assert!(find_mixnode_by_id("-1", mixnodes).is_err());
    }
    
    #[test]
    fn test_find_mixnode_by_id_edge_cases() {
        let mixnodes = vec![
            create_test_mixnode(0, false),
            create_test_mixnode(u32::MAX, false),
        ];
        
        assert!(find_mixnode_by_id("0", mixnodes.clone()).is_ok());
        assert!(find_mixnode_by_id(&u32::MAX.to_string(), mixnodes).is_ok());
    }
    
    #[test]
    fn test_validate_offset_valid() {
        assert_eq!(validate_offset(None).unwrap(), 0);
        assert_eq!(validate_offset(Some(0)).unwrap(), 0);
        assert_eq!(validate_offset(Some(10)).unwrap(), 10);
        assert_eq!(validate_offset(Some(1000)).unwrap(), 1000);
    }
    
    #[test]
    fn test_validate_offset_invalid() {
        assert!(validate_offset(Some(-1)).is_err());
        assert!(validate_offset(Some(-100)).is_err());
        assert!(validate_offset(Some(i64::MIN)).is_err());
    }
    
    #[test]
    fn test_mix_id_param_deserialization() {
        let json = r#"{"mix_id": "123"}"#;
        let param: MixIdParam = serde_json::from_str(json).unwrap();
        assert_eq!(param.mix_id, "123");
    }
    
    #[test]
    fn test_mix_stats_query_params_deserialization() {
        let json = r#"{"offset": 50}"#;
        let params: MixStatsQueryParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.offset, Some(50));
        
        let json_empty = r#"{}"#;
        let params_empty: MixStatsQueryParams = serde_json::from_str(json_empty).unwrap();
        assert_eq!(params_empty.offset, None);
    }
    
    #[test]
    fn test_daily_stats_creation() {
        let stats = DailyStats {
            date_utc: "2024-01-20".to_string(),
            total_packets_received: 1000000,
            total_packets_sent: 999000,
            total_packets_dropped: 1000,
            total_stake: 5000000,
        };
        
        assert_eq!(stats.total_packets_received, 1000000);
        assert_eq!(stats.total_packets_sent, 999000);
        assert_eq!(stats.total_packets_dropped, 1000);
        assert_eq!(stats.total_stake, 5000000);
    }
}
