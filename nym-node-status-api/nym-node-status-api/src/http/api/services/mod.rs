use crate::http::{
    error::{HttpError, HttpResult},
    models::Service,
    state::AppState,
    PagedResult, Pagination,
};
use axum::{
    extract::{Query, State},
    Json, Router,
};
use json_path::{ParseJsonPaths, ParsedDetails};
use tracing::instrument;

mod json_path;

pub(crate) fn routes() -> Router<AppState> {
    Router::new().route("/", axum::routing::get(mixnodes))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub(crate) struct ServicesQueryParams {
    size: Option<usize>,
    page: Option<usize>,
    wss: Option<bool>,
    hostname: Option<bool>,
    entry: Option<bool>,
}

#[utoipa::path(
    tag = "Services",
    get,
    params(
        ServicesQueryParams,
    ),
    path = "/v2/services",
    responses(
        (status = 200, body = PagedService)
    )
)]
#[instrument(level = tracing::Level::DEBUG, skip(state))]
async fn mixnodes(
    Query(ServicesQueryParams {
        size,
        page,
        wss,
        hostname,
        entry,
    }): Query<ServicesQueryParams>,
    State(state): State<AppState>,
) -> HttpResult<Json<PagedResult<Service>>> {
    let db = state.db_pool();
    let cache = state.cache();

    let show_only_wss = wss.unwrap_or(false);
    let show_only_with_hostname = hostname.unwrap_or(false);
    let show_entry_gateways_only = entry.unwrap_or(false);

    let paths = ParseJsonPaths::new().map_err(|e| {
        tracing::error!("Invalidly configured ParseJsonPaths: {e}");
        HttpError::internal()
    })?;
    let res = cache.get_gateway_list(db).await;
    let res: Vec<Service> = res
        .iter()
        .map(|g| {
            let details = ParsedDetails::new(&paths, g);

            let s = Service {
                gateway_identity_key: g.gateway_identity_key.clone(),
                ip_address: details.ip_address,
                service_provider_client_id: details.service_provider_client_id,
                hostname: details.hostname,
                last_successful_ping_utc: g.last_testrun_utc.clone(),
                last_updated_utc: g.last_updated_utc.clone(),
                // routing_score: g.routing_score,
                routing_score: 1f32,
                mixnet_websockets: g
                    .self_described
                    .clone()
                    .and_then(|s| s.get("mixnet_websockets").cloned()),
            };

            let f = ServiceFilter::new(&s);

            (s, f)
        })
        .filter(|(_, f)| {
            let mut keep = f.has_network_requester_sp;

            if show_entry_gateways_only {
                keep = true;
            }

            if show_only_wss {
                keep &= f.has_wss;
            }
            if show_only_with_hostname {
                keep &= f.has_hostname;
            }

            keep
        })
        .map(|(s, _)| s)
        .collect();

    Ok(Json(PagedResult::paginate(Pagination { size, page }, res)))
}

struct ServiceFilter {
    has_wss: bool,
    has_network_requester_sp: bool,
    has_hostname: bool,
}

impl ServiceFilter {
    fn new(s: &Service) -> Self {
        let has_wss = match &s.mixnet_websockets {
            Some(v) => v.get("wss_port").map(|v2| !v2.is_null()).unwrap_or(false),
            None => false,
        };
        let has_hostname = s.hostname.is_some();
        let has_network_requester_sp = match &s.service_provider_client_id {
            Some(v) => !v.is_empty(),
            None => false,
        };

        ServiceFilter {
            has_wss,
            has_hostname,
            has_network_requester_sp,
        }
    }
}
