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
    models::ExtendedNymNode,
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
#[instrument(level = tracing::Level::DEBUG, skip_all, fields(page=pagination.page, size=pagination.size))]
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

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Path)]
struct NodeIdParam {
    #[param(minimum = 0)]
    node_id: NodeId,
}

#[utoipa::path(
    tag = "Nym Nodes",
    get,
    params(
        NodeIdParam
    ),
    path = "/{node_id}/delegations",
    context_path = "/v3/nym-nodes",
    responses(
        (status = 200, body = PagedResult<ExtendedNymNode>)
    )
)]
#[instrument(level = tracing::Level::DEBUG, skip(state))]
async fn node_delegations(
    Path(node_id): Path<NodeId>,
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

    todo!()
    // Ok(Json())
}
