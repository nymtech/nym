use axum::{
    extract::{Query, State},
    Json, Router,
};
use time::Date;
use tracing::instrument;

use crate::http::{
    error::{HttpError, HttpResult},
    models::SessionStats,
    state::AppState,
    PagedResult, Pagination,
};

pub(crate) fn routes() -> Router<AppState> {
    Router::new().route("/", axum::routing::get(get_all_sessions))
    // .route("/:node_id", axum::routing::get(get_node_sessions))
    // .route("/:day", axum::routing::get(get_daily_sessions))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct SessionQueryParams {
    size: Option<usize>,
    page: Option<usize>,
    node_id: Option<String>,
    day: Option<String>,
}

#[utoipa::path(
    tag = "Sessions",
    get,
    params(
        SessionQueryParams
    ),
    path = "/v2/metrics/sessions",
    responses(
        (status = 200, body = PagedResult<SessionStats>)
    )
)]
#[instrument(level = tracing::Level::DEBUG, skip(state))]
async fn get_all_sessions(
    Query(SessionQueryParams {
        size,
        page,
        node_id,
        day,
    }): Query<SessionQueryParams>,
    State(state): State<AppState>,
) -> HttpResult<Json<PagedResult<SessionStats>>> {
    let db = state.db_pool();
    let res = state.cache().get_sessions_stats(db).await;

    let day_filtered = if let Some(day) = day {
        if let Ok(parsed_day) =
            Date::parse(&day, &time::format_description::well_known::Iso8601::DATE)
        {
            res.into_iter().filter(|s| s.day == parsed_day).collect()
        } else {
            return Err(HttpError::invalid_input(day));
        }
    } else {
        res
    };

    let day_and_node_filtered = if let Some(node_id) = node_id {
        if let Ok(parsed_node_id) = node_id.parse::<u32>() {
            day_filtered
                .into_iter()
                .filter(|s| s.node_id == parsed_node_id)
                .collect()
        } else {
            return Err(HttpError::invalid_input(node_id));
        }
    } else {
        day_filtered
    };

    Ok(Json(PagedResult::paginate(
        Pagination { size, page },
        day_and_node_filtered,
    )))
}
