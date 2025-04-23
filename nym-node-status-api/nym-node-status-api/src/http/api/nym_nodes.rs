use axum::{
    extract::{Query, State},
    Json, Router,
};
use tracing::instrument;

use crate::http::{
    error::{HttpError, HttpResult},
    models::ExtendedNymNode,
    state::AppState,
    PagedResult, Pagination,
};

pub(crate) fn routes() -> Router<AppState> {
    Router::new().route("/", axum::routing::get(nym_nodes))
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
