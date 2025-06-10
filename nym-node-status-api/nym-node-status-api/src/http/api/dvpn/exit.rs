use crate::http::{
    api::dvpn::{country::TwoLetterCountryCodeParam, MIN_SUPPORTED_VERSION},
    models::DVpnGateway,
};
use crate::http::{
    error::{HttpError, HttpResult},
    state::AppState,
};
use axum::{
    extract::{Path, State},
    Json, Router,
};
use itertools::Itertools;
use tracing::instrument;

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/exit", axum::routing::get(get_exit_gateways))
        .route(
            "/exit/countries",
            axum::routing::get(get_entry_gateway_countries),
        )
        .route(
            "/exit/country/:two_letter_country_code",
            axum::routing::get(get_exit_gateways_by_country),
        )
}

#[utoipa::path(
    tag = "dVPN Directory Cache",
    get,
    path = "/exit",
    summary = "Gets available exit gateways from the Nym network directory",
    context_path = "/dvpn/v1/directory/gateways",
    operation_id = "getExitGateways",
    responses(
        (status = 200, body = Vec<DVpnGateway>)
    )
)]
#[instrument(level = tracing::Level::INFO, skip(state))]
pub async fn get_exit_gateways(state: State<AppState>) -> HttpResult<Json<Vec<DVpnGateway>>> {
    Ok(Json(
        state
            .cache()
            .get_exit_dvpn_gateways(state.db_pool(), &MIN_SUPPORTED_VERSION)
            .await,
    ))
}

#[utoipa::path(
    tag = "dVPN Directory Cache",
    get,
    path = "/exit/countries",
    summary = "Gets available exit gateway countries as two-letter ISO country codes from the Nym network directory",
    context_path = "/dvpn/v1/directory/gateways",
    operation_id = "getExitGatewayCountries",
    responses(
        (status = 200, body = Vec<String>)
    )
)]
#[instrument(level = tracing::Level::INFO, skip(state))]
pub async fn get_entry_gateway_countries(state: State<AppState>) -> HttpResult<Json<Vec<String>>> {
    Ok(Json(
        state
            .cache()
            .get_exit_dvpn_gateways(state.db_pool(), &MIN_SUPPORTED_VERSION)
            .await
            .into_iter()
            .map(|gw| gw.location.two_letter_iso_country_code.to_string())
            // dedup relies on iterator being sorted by country, but we already do that
            .dedup()
            .collect(),
    ))
}

#[utoipa::path(
    tag = "dVPN Directory Cache",
    get,
    path = "/exit/country/{country_code}",
    summary = "Gets available exit gateways from the Nym network directory by country",
    context_path = "/dvpn/v1/directory/gateways",
    operation_id = "getExitGatewaysByCountry",
    responses(
        (status = 200, body = Vec<DVpnGateway>)
    )
)]
#[instrument(level = tracing::Level::INFO, skip(state))]
pub async fn get_exit_gateways_by_country(
    Path(TwoLetterCountryCodeParam {
        two_letter_country_code,
    }): Path<TwoLetterCountryCodeParam>,
    state: State<AppState>,
) -> HttpResult<Json<Vec<DVpnGateway>>> {
    let country_filter = two_letter_country_code.to_lowercase();
    if country_filter.len() != 2 {
        return Err(HttpError::invalid_country_code());
    }
    Ok(Json(
        state
            .cache()
            .get_exit_dvpn_gateways(state.db_pool(), &MIN_SUPPORTED_VERSION)
            .await
            .into_iter()
            .filter(|gw| gw.location.two_letter_iso_country_code.to_lowercase() == country_filter)
            .collect(),
    ))
}
