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
        (status = 200, body = PagedResult<Service>)
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

    Ok(Json(PagedResult::paginate(
        Pagination::new(size, page),
        res,
    )))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::models::Service;
    use serde_json::json;

    #[test]
    fn test_service_filter() {
        // Test with all fields
        let service1 = Service {
            gateway_identity_key: "1".to_string(),
            last_updated_utc: "".to_string(),
            routing_score: 1.0,
            service_provider_client_id: Some("client_id".to_string()),
            ip_address: Some("1.1.1.1".to_string()),
            hostname: Some("nymtech.net".to_string()),
            mixnet_websockets: Some(json!({ "wss_port": 1234 })),
            last_successful_ping_utc: None,
        };
        let filter1 = ServiceFilter::new(&service1);
        assert!(filter1.has_wss);
        assert!(filter1.has_network_requester_sp);
        assert!(filter1.has_hostname);

        // Test with no fields
        let service2 = Service {
            gateway_identity_key: "2".to_string(),
            last_updated_utc: "".to_string(),
            routing_score: 0.0,
            service_provider_client_id: None,
            ip_address: None,
            hostname: None,
            mixnet_websockets: None,
            last_successful_ping_utc: None,
        };
        let filter2 = ServiceFilter::new(&service2);
        assert!(!filter2.has_wss);
        assert!(!filter2.has_network_requester_sp);
        assert!(!filter2.has_hostname);

        // Test with some fields
        let service3 = Service {
            gateway_identity_key: "3".to_string(),
            last_updated_utc: "".to_string(),
            routing_score: 0.5,
            service_provider_client_id: Some("".to_string()),
            ip_address: None,
            hostname: Some("nymtech.net".to_string()),
            mixnet_websockets: Some(json!({})),
            last_successful_ping_utc: None,
        };
        let filter3 = ServiceFilter::new(&service3);
        assert!(!filter3.has_wss);
        assert!(!filter3.has_network_requester_sp);
        assert!(filter3.has_hostname);
    }
}