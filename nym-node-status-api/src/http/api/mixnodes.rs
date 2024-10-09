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
        (status = 200, body = PagedMixnode)
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
    match mix_id.parse::<u32>() {
        Ok(parsed_mix_id) => {
            let res = state.cache().get_mixnodes_list(state.db_pool()).await;

            match res.iter().find(|item| item.mix_id == parsed_mix_id) {
                Some(res) => Ok(Json(res.clone())),
                None => Err(HttpError::invalid_input(mix_id)),
            }
        }
        Err(_e) => Err(HttpError::invalid_input(mix_id)),
    }
}

#[utoipa::path(
    tag = "Mixnodes",
    get,
    path = "/v2/mixnodes/stats",
    responses(
        (status = 200, body = Vec<DailyStats>)
    )
)]
async fn get_stats(State(state): State<AppState>) -> HttpResult<Json<Vec<DailyStats>>> {
    let stats = state.cache().get_mixnode_stats(state.db_pool()).await;
    Ok(Json(stats))
}
