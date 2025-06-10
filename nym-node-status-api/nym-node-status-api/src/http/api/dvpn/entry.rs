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
        .route("/entry", axum::routing::get(get_entry_gateways))
        .route(
            "/entry/countries",
            axum::routing::get(get_entry_gateway_countries),
        )
        .route(
            "/entry/country/:two_letter_country_code",
            axum::routing::get(get_entry_gateways_by_country),
        )
}

#[utoipa::path(
    tag = "dVPN Directory Cache",
    get,
    path = "/entry",
    summary = "Gets available entry gateways from the Nym network directory",
    context_path = "/dvpn/v1/directory/gateways",
    operation_id = "getEntryGateways",
    responses(
        (status = 200, body = Vec<DVpnGateway>)
    )
)]
#[instrument(level = tracing::Level::INFO, skip(state))]
pub async fn get_entry_gateways(state: State<AppState>) -> HttpResult<Json<Vec<DVpnGateway>>> {
    Ok(Json(
        state
            .cache()
            .get_entry_dvpn_gateways(state.db_pool(), &MIN_SUPPORTED_VERSION)
            .await,
    ))
}

#[utoipa::path(
    tag = "dVPN Directory Cache",
    get,
    path = "/entry/countries",
    summary = "Gets available entry gateway countries as two-letter ISO country codes from the Nym network directory",
    context_path = "/dvpn/v1/directory/gateways",
    operation_id = "getEntryGatewayCountries",
    responses(
        (status = 200, body = Vec<String>)
    )
)]
#[instrument(level = tracing::Level::INFO, skip(state))]
pub async fn get_entry_gateway_countries(state: State<AppState>) -> HttpResult<Json<Vec<String>>> {
    Ok(Json(
        state
            .cache()
            .get_entry_dvpn_gateways(state.db_pool(), &MIN_SUPPORTED_VERSION)
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
    path = "/entry/country/{country_code}",
    summary = "Gets available entry gateways from the Nym network directory by country",
    context_path = "/dvpn/v1/directory/gateways",
    operation_id = "getEntryGatewaysByCountry",
    responses(
        (status = 200, body = Vec<DVpnGateway>)
    )
)]
#[instrument(level = tracing::Level::INFO, skip(state))]
pub async fn get_entry_gateways_by_country(
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
            .get_entry_dvpn_gateways(state.db_pool(), &MIN_SUPPORTED_VERSION)
            .await
            .into_iter()
            .filter(|gw| gw.location.two_letter_iso_country_code.to_lowercase() == country_filter)
            .collect(),
    ))
}
