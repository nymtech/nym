use axum::{
    extract::{Path, State},
    Json, Router,
};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::{
    db::{
        models::JokeDto,
        queries::{self, select_joke_by_id},
    },
    http::{
        error::{Error, HttpResult},
        state::AppState,
    },
};

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/", axum::routing::get(jokes))
        .route("/:joke_id", axum::routing::get(joke_by_id))
        .route("/fetch_another", axum::routing::get(fetch_another))
}

#[utoipa::path(
    tag = "Dad Jokes",
    get,
    path = "/v1/jokes",
    responses(
        (status = 200, body = Vec<JokeDto>)
    )
)]
async fn jokes(State(state): State<AppState>) -> HttpResult<Json<Vec<JokeDto>>> {
    queries::select_all(state.db_pool())
        .await
        .map(Json::from)
        .map_err(|_| Error::internal())
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Path)]
struct JokeIdParam {
    joke_id: String,
}

#[utoipa::path(
    tag = "Dad Jokes",
    get,
    params(
        JokeIdParam
    ),
    path = "/v1/jokes/{joke_id}",
    responses(
        (status = 200, body = JokeDto)
    )
)]
async fn joke_by_id(
    Path(JokeIdParam { joke_id }): Path<JokeIdParam>,
    State(state): State<AppState>,
) -> HttpResult<Json<JokeDto>> {
    select_joke_by_id(state.db_pool(), &joke_id)
        .await
        .map(Json::from)
        .map_err(|_| Error::not_found(joke_id))
}

#[utoipa::path(
    tag = "Dad Jokes",
    get,
    path = "/v1/jokes/fetch_another",
    responses(
        (status = 200, body = String)
    )
)]
async fn fetch_another(State(_state): State<AppState>) -> HttpResult<Json<String>> {
    Ok(Json(String::from("Done boss, check the DB")))
}
