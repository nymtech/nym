use axum::{
    extract::{Path, State},
    Json, Router,
};
use itertools::Itertools;
use serde::Deserialize;
use tracing::instrument;
use utoipa::IntoParams;

use crate::http::{api::dvpn::MIN_SUPPORTED_VERSION, models::DVpnGateway};
use crate::http::{
    error::{HttpError, HttpResult},
    state::AppState,
};

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/country/:two_letter_country_code",
            axum::routing::get(get_gateways_by_country),
        )
        .route("/countries", axum::routing::get(get_gateway_countries))
}

#[allow(dead_code)] // clippy doesn't detect usage in utoipa macros
#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Path)]
pub(crate) struct TwoLetterCountryCodeParam {
    #[param(min_length = 2, max_length = 2)]
    pub(crate) two_letter_country_code: String,
}

#[utoipa::path(
    tag = "dVPN Directory Cache",
    get,
    params(
        TwoLetterCountryCodeParam
    ),
    path = "/country/{two_letter_country_code}",
    summary = "Gets available gateways from the Nym network directory by country",
    context_path = "/dvpn/v1/directory/gateways",
    operation_id = "getGatewaysByCountry",
    responses(
        (status = 200, body = Vec<DVpnGateway>)
    )
)]
#[instrument(level = tracing::Level::INFO, skip(state), fields(two_letter_country_code = two_letter_country_code))]
pub async fn get_gateways_by_country(
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
        _ => Err(HttpError::invalid_country_code()),
    }
}

#[utoipa::path(
    tag = "dVPN Directory Cache",
    get,
    path = "/countries",
    summary = "Gets available exit gateway countries as two-letter ISO country codes from the Nym network directory",
    context_path = "/dvpn/v1/directory/gateways",
    operation_id = "getGatewayCountries",
    responses(
        (status = 200, body = Vec<String>)
    )
)]
#[instrument(level = tracing::Level::INFO, skip(state))]
pub async fn get_gateway_countries(state: State<AppState>) -> HttpResult<Json<Vec<String>>> {
    Ok(Json(
        state
            .cache()
            .get_dvpn_gateway_list(state.db_pool(), &MIN_SUPPORTED_VERSION)
            .await
            .into_iter()
            .map(|gw| gw.location.two_letter_iso_country_code.to_string())
            // dedup relies on iterator being sorted by country, but we already do that
            .dedup()
            .collect(),
    ))
}
