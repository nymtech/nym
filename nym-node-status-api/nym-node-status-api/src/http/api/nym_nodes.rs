use axum::{
    extract::{Path, Query, State},
    Json, Router,
};
use nym_validator_client::client::NodeId;
use serde::Deserialize;
use tracing::instrument;
use utoipa::IntoParams;

use crate::http::{
    error::{HttpError, HttpResult},
    models::{ExtendedNymNode, NodeDelegation},
    state::AppState,
    PagedResult, Pagination,
};

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/", axum::routing::get(nym_nodes))
        .route(
            "/:node_id/delegations",
            axum::routing::get(node_delegations),
        )
}

#[utoipa::path(
    tag = "Nym Explorer",
    get,
    params(
        Pagination
    ),
    path = "/nym-nodes",
    context_path = "/explorer/v3",
    responses(
        (status = 200, body = PagedResult<ExtendedNymNode>)
    )
)]
#[instrument(level = tracing::Level::INFO, skip_all, fields(page=pagination.page, size=pagination.size))]
async fn nym_nodes(
    Query(pagination): Query<Pagination>,
    State(state): State<AppState>,
) -> HttpResult<Json<PagedResult<ExtendedNymNode>>> {
    let db = state.db_pool();
    let node_geocache = state.node_geocache();

    let nodes = state
        .cache()
        .get_nym_nodes_list(db, node_geocache)
        .await
        .map_err(|e| {
            tracing::error!("{e}");
            HttpError::internal()
        })?;

    Ok(Json(PagedResult::paginate(pagination, nodes)))
}

#[allow(dead_code)] // clippy doesn't detect usage in utoipa macros
#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Path)]
struct NodeIdParam {
    #[param(minimum = 0)]
    node_id: NodeId,
}

#[utoipa::path(
    tag = "Nym Explorer",
    get,
    params(
        NodeIdParam
    ),
    path = "/{node_id}/delegations",
    context_path = "/explorer/v3/nym-nodes",
    responses(
        (status = 200, body = NodeDelegation)
    )
)]
#[instrument(level = tracing::Level::INFO, skip(state))]
async fn node_delegations(
    Path(node_id): Path<NodeId>,
    State(state): State<AppState>,
) -> HttpResult<Json<Vec<NodeDelegation>>> {
    state
        .node_delegations(node_id)
        .await
        .ok_or_else(|| HttpError::no_delegations_for_node(node_id))
        .map(Json)
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
    fn test_node_id_param_deserialization() {
        // Test valid node ID
        let json = r#"{"node_id": 42}"#;
        let param: NodeIdParam = serde_json::from_str(json).unwrap();
        assert_eq!(param.node_id, 42);
        
        // Test zero node ID
        let json_zero = r#"{"node_id": 0}"#;
        let param_zero: NodeIdParam = serde_json::from_str(json_zero).unwrap();
        assert_eq!(param_zero.node_id, 0);
        
        // Test max node ID
        let json_max = format!(r#"{{"node_id": {}}}"#, u32::MAX);
        let param_max: NodeIdParam = serde_json::from_str(&json_max).unwrap();
        assert_eq!(param_max.node_id, u32::MAX);
    }
}
