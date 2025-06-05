use axum::{
    extract::{Path, Query, State},
    Json, Router,
};
use semver::Version;
use serde::Deserialize;
use std::sync::LazyLock;
use tracing::instrument;
use utoipa::IntoParams;

use crate::http::models::DVpnGateway;
use crate::http::{
    error::{HttpError, HttpResult},
    state::AppState,
};

static MIN_SUPPORTED_VERSION: LazyLock<Version> = LazyLock::new(|| Version::new(1, 6, 2));

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
    let country_filter = two_letter_country_code.to_lowercase();
    match two_letter_country_code.len() {
        2 => Ok(Json(
            state
                .cache()
                .get_dvpn_gateway_list(state.db_pool(), &MIN_SUPPORTED_VERSION)
                .await
                .into_iter()
                .filter(|gw| {
                    gw.location.two_letter_iso_country_code.to_lowercase() == country_filter
                })
                .collect(),
        )),
        _ => Err(HttpError::invalid_input(
            "Only two letter country code is allowed",
        )),
    }
}
