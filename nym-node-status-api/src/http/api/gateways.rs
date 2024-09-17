use axum::{
    extract::{Query, State},
    Json, Router,
};

use crate::http::{error::HttpResult, models::Gateway, state::AppState, PagedResult, Pagination};

pub(crate) fn routes() -> Router<AppState> {
    Router::new().route("/", axum::routing::get(gateways))
}

#[utoipa::path(
    tag = "Gateways",
    get,
    params(
        Pagination
    ),
    path = "/v2/gateways",
    responses(
        (status = 200, body = Json<PagedResult<Gateway>>)
    )
)]
async fn gateways(
    Query(pagination): Query<Pagination>,
    State(state): State<AppState>,
) -> HttpResult<Json<PagedResult<Gateway>>> {
    let res = state.cache();
    let (size, page) = pagination.to_inner_values();

    Ok(Json(PagedResult {
        page,
        size,
        total: todo!(),
        items: todo!(),
    }))
}
