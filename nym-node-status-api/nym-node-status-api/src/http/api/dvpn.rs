use axum::{
    extract::{Path, Query, State},
    Json, Router,
};
use serde::Deserialize;
use tracing::instrument;
use utoipa::IntoParams;

use crate::http::models::DVpnGateway;
use crate::http::{
    error::{HttpError, HttpResult},
    state::AppState,
};

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/", axum::routing::get(dvpn_gateways))
        .route(
            "/country/:two_letter_country_code",
            axum::routing::get(gateways_by_country),
        )
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
struct MinNodeVersionQuery {
    min_node_version: Option<String>,
}

#[utoipa::path(
    tag = "dVPN Directory Cache",
    get,
    params(
        MinNodeVersionQuery
    ),
    path = "/directory/gateways",
    summary = "Gets available entry and exit gateways from the Nym network directory",
    context_path = "/dvpn/v1",
    responses(
        (status = 200, body = Vec<DVpnGateway>)
    )
)]
#[instrument(level = tracing::Level::INFO, skip(state))]
async fn dvpn_gateways(
    Query(MinNodeVersionQuery { min_node_version }): Query<MinNodeVersionQuery>,
    state: State<AppState>,
) -> HttpResult<Json<Vec<DVpnGateway>>> {
    let min_node_version: String = min_node_version.unwrap_or_else(|| String::from("1.6.2"));
    let _min_node_version = semver::Version::parse(&min_node_version)
        .map_err(|_| HttpError::invalid_input("Min version must be valid semver"))?;

    Ok(Json(
        state.cache().get_dvpn_gateway_list(state.db_pool()).await,
    ))
}

#[allow(dead_code)] // clippy doesn't detect usage in utoipa macros
#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Path)]
struct TwoLetterCountryCodeParam {
    #[param(min_length = 2, max_length = 2)]
    two_letter_country_code: String,
}

#[utoipa::path(
    tag = "dVPN Directory Cache",
    get,
    params(
        TwoLetterCountryCodeParam
    ),
    path = "/directory/gateways/country/{two_letter_country_code}",
    summary = "Gets available gateways from the Nym network directory by country",
    context_path = "/dvpn/v1",
    responses(
        (status = 200, body = Vec<DVpnGateway>)
    )
)]
#[instrument(level = tracing::Level::INFO, skip(state), fields(two_letter_country_code = two_letter_country_code))]
async fn gateways_by_country(
    Path(TwoLetterCountryCodeParam {
        two_letter_country_code,
    }): Path<TwoLetterCountryCodeParam>,
    state: State<AppState>,
) -> HttpResult<Json<Vec<DVpnGateway>>> {
    match two_letter_country_code.len() {
        2 => Ok(Json(
            state.cache().get_dvpn_gateway_list(state.db_pool()).await,
        )),
        _ => Err(HttpError::invalid_input(
            "Only two letter country code is allowed",
        )),
    }
}
