use crate::http::state::AppState;
use axum::Router;
use axum::{
    extract::{Query, State},
    Json,
};
use semver::Version;
use serde::Deserialize;
use std::sync::LazyLock;
use tracing::instrument;
use utoipa::IntoParams;

use crate::http::error::{HttpError, HttpResult};
use crate::http::models::DVpnGateway;

pub mod country;
pub mod entry;
pub mod exit;

static MIN_SUPPORTED_VERSION: LazyLock<Version> = LazyLock::new(|| Version::new(1, 6, 2));

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/", axum::routing::get(dvpn_gateways))
        .merge(country::routes())
        .merge(entry::routes())
        .merge(exit::routes())
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct MinNodeVersionQuery {
    min_node_version: Option<String>,
}

#[utoipa::path(
    tag = "dVPN Directory Cache",
    get,
    params(
        MinNodeVersionQuery
    ),
    path = "",
    summary = "Gets available entry and exit gateways from the Nym network directory",
    context_path = "/dvpn/v1/directory/gateways",
    operation_id = "getGateways",
    responses(
        (status = 200, body = Vec<DVpnGateway>)
    )
)]
#[instrument(level = tracing::Level::INFO, skip(state))]
pub async fn dvpn_gateways(
    Query(MinNodeVersionQuery { min_node_version }): Query<MinNodeVersionQuery>,
    state: State<AppState>,
) -> HttpResult<Json<Vec<DVpnGateway>>> {
    let min_node_version = match min_node_version {
        Some(min_version) => semver::Version::parse(&min_version)
            .map_err(|_| HttpError::invalid_input("Min version must be valid semver"))?,
        None => MIN_SUPPORTED_VERSION.clone(),
    };

    Ok(Json(
        state
            .cache()
            .get_dvpn_gateway_list(state.db_pool(), &min_node_version)
            .await,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_routes_construction() {
        let router = routes();
        // Verify the router builds without panic
        let _routes = router;
    }
    
    #[test]
    fn test_min_node_version_query_deserialization() {
        // Test with version
        let json = r#"{"min_node_version": "1.2.3"}"#;
        let query: MinNodeVersionQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.min_node_version, Some("1.2.3".to_string()));
        
        // Test without version
        let json_empty = r#"{}"#;
        let query_empty: MinNodeVersionQuery = serde_json::from_str(json_empty).unwrap();
        assert_eq!(query_empty.min_node_version, None);
    }
    
    #[test]
    fn test_min_supported_version() {
        // Test that the lazy static initializes correctly
        assert_eq!(MIN_SUPPORTED_VERSION.major, 1);
        assert_eq!(MIN_SUPPORTED_VERSION.minor, 6);
        assert_eq!(MIN_SUPPORTED_VERSION.patch, 2);
    }
}
