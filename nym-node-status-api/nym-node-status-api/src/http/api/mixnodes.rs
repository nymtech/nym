use axum::{
    Json, Router,
    extract::{Query, State},
};
use serde::Deserialize;
use tracing::instrument;
use utoipa::IntoParams;

use crate::http::{
    error::{HttpError, HttpResult},
    models::DailyStats,
    state::AppState,
};

pub(crate) fn routes() -> Router<AppState> {
    Router::new().route("/stats", axum::routing::get(get_stats))
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

fn validate_offset(offset: Option<i64>) -> HttpResult<usize> {
    offset
        .unwrap_or(0)
        .try_into()
        .map_err(|_| HttpError::invalid_input("Offset must be non-negative"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::models::DailyStats;

    #[test]
    fn test_routes_construction() {
        let router = routes();
        // Just verify the router builds without panic
        // Actual route testing would require integration tests
        let _routes = router;
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
