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
    path = "/",
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
