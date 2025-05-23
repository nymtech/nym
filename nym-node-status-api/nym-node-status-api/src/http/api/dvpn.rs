use axum::{
    extract::{Path, Query, State},
    Json, Router,
};
use itertools::Itertools;
use serde::Deserialize;
use tracing::instrument;
use utoipa::IntoParams;

use crate::http::{error::{HttpError, HttpResult}, models::{Gateway}, state::AppState};
use crate::http::models::DVpnGateway;

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/", axum::routing::get(gateways))
        .route(
            "/country/:two_letter_country_code",
            axum::routing::get(gateways_by_country),
        )
}

async fn get_gateways_from_cache(
    State(state): State<AppState>,
) -> Vec<DVpnGateway> {
    let db = state.db_pool();
    let res = state.cache().get_gateway_list(db).await;

    // TODO: parse
    let MINIMUM_NYM_NODE_VERSION = "1.6.2";

    // TODO: cache the output of this filter
    let output: Vec<DVpnGateway> = res.iter()
        .filter(|g| {
            // gateways must be bonded and not blacklisted
            if !g.bonded {
                return false;
            }

            // gateways must meet minimum semver
            // if g.self_described.something_something < MINIMUM_NYM_NODE_VERSION {
            //     return false;
            // }

            true
        })
        .map(|d| d.try_into())
        .filter_map(Result::ok)
        .filter(|g: &DVpnGateway| {
            // gateways must have a country
            g.location.two_letter_iso_country_code.len() == 2
        })
        // TODO: sort by two-letter country code, then by identity key
        // .sorted_by(...)
        .collect();

    output
}

#[utoipa::path(
    tag = "dVPN Directory Cache",
    get,
    path = "/directory/gateways",
    summary = "Gets available entry and exit gateways from the Nym network directory",
    context_path = "/dvpn/v1",
    responses(
        (status = 200, body = Vec<DVpnGateway>)
    )
)]
#[instrument(level = tracing::Level::INFO, skip_all)]
async fn gateways(
    state: State<AppState>,
) -> HttpResult<Json<Vec<DVpnGateway>>> {
    let res = get_gateways_from_cache(state).await;
    Ok(Json(res))
}

#[allow(dead_code)] // clippy doesn't detect usage in utoipa macros
#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Path)]
struct TwoLetterCountryCodeParam {
    #[param(minimum = 2, maximum = 2)]
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
    Path(TwoLetterCountryCodeParam { two_letter_country_code}): Path<TwoLetterCountryCodeParam>,
    state: State<AppState>,
) -> HttpResult<Json<Vec<DVpnGateway>>> {
    match two_letter_country_code.len() {
        2 => {
            let res = get_gateways_from_cache(state).await;
            Ok(Json(res))
        }
        _ => Err(HttpError::invalid_input("Only two letter country code is allowed")),
    }
}
