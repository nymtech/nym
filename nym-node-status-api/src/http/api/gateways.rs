use axum::{
    extract::{Path, Query, State},
    Json, Router,
};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::http::{
    error::{HttpError, HttpResult},
    models::{Gateway, GatewaySkinny},
    state::AppState,
    PagedResult, Pagination,
};

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/", axum::routing::get(gateways))
        .route("/skinny", axum::routing::get(gateways_skinny))
        .route("/:identity_key", axum::routing::get(get_gateway))
}

#[utoipa::path(
    tag = "Gateways",
    get,
    params(
        Pagination
    ),
    path = "/v2/gateways",
    responses(
        (status = 200, body = PagedGateway)
    )
)]
async fn gateways(
    Query(pagination): Query<Pagination>,
    State(state): State<AppState>,
) -> HttpResult<Json<PagedResult<Gateway>>> {
    let db = state.db_pool();
    let res = state.cache().get_gateway_list(db).await;

    Ok(Json(PagedResult::paginate(pagination, res)))
}

#[utoipa::path(
    tag = "Gateways",
    get,
    params(
        Pagination
    ),
    path = "/v2/gateways/skinny",
    responses(
        (status = 200, body = PagedGatewaySkinny)
    )
)]
async fn gateways_skinny(
    Query(pagination): Query<Pagination>,
    State(state): State<AppState>,
) -> HttpResult<Json<PagedResult<GatewaySkinny>>> {
    let db = state.db_pool();
    let res = state.cache().get_gateway_list(db).await;
    let res: Vec<GatewaySkinny> = res
        .iter()
        .filter(|g| g.bonded)
        .map(|g| GatewaySkinny {
            gateway_identity_key: g.gateway_identity_key.clone(),
            self_described: g.self_described.clone(),
            performance: g.performance,
            explorer_pretty_bond: g.explorer_pretty_bond.clone(),
            last_probe_result: g.last_probe_result.clone(),
            last_testrun_utc: g.last_testrun_utc.clone(),
            last_updated_utc: g.last_updated_utc.clone(),
            routing_score: g.routing_score,
            config_score: g.config_score,
        })
        .collect();

    Ok(Json(PagedResult::paginate(pagination, res)))
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Path)]
struct IdentityKeyParam {
    identity_key: String,
}

#[utoipa::path(
    tag = "Gateways",
    get,
    params(
        IdentityKeyParam
    ),
    path = "/v2/gateways/{identity_key}",
    responses(
        (status = 200, body = Gateway)
    )
)]
async fn get_gateway(
    Path(IdentityKeyParam { identity_key }): Path<IdentityKeyParam>,
    State(state): State<AppState>,
) -> HttpResult<Json<Gateway>> {
    let db = state.db_pool();
    let res = state.cache().get_gateway_list(db).await;

    match res
        .iter()
        .find(|item| item.gateway_identity_key == identity_key)
    {
        Some(res) => Ok(Json(res.clone())),
        None => Err(HttpError::invalid_input(identity_key)),
    }
}
