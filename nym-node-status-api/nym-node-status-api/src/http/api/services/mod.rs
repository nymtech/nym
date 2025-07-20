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
    Query(params): Query<ServicesQueryParams>,
    State(state): State<AppState>,
) -> HttpResult<Json<PagedResult<Service>>> {
    let db = state.db_pool();
    let cache = state.cache();

    let paths = ParseJsonPaths::new().map_err(|e| {
        tracing::error!("Invalidly configured ParseJsonPaths: {e}");
        HttpError::internal()
    })?;
    let res = cache.get_gateway_list(db).await;
    let services = gateway_list_to_services(&paths, res, params.clone());

    Ok(Json(PagedResult::paginate(
        Pagination::new(params.size, params.page),
        services,
    )))
}

// Extract the conversion and filtering logic for testing
fn gateway_list_to_services(
    paths: &ParseJsonPaths,
    gateways: Vec<crate::http::models::Gateway>,
    params: ServicesQueryParams,
) -> Vec<Service> {
    let show_only_wss = params.wss.unwrap_or(false);
    let show_only_with_hostname = params.hostname.unwrap_or(false);
    let show_entry_gateways_only = params.entry.unwrap_or(false);

    gateways
        .iter()
        .map(|g| {
            let details = ParsedDetails::new(paths, g);

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
            apply_service_filters(f, show_only_wss, show_only_with_hostname, show_entry_gateways_only)
        })
        .map(|(s, _)| s)
        .collect()
}

// Extract filter application logic
fn apply_service_filters(
    filter: &ServiceFilter,
    show_only_wss: bool,
    show_only_with_hostname: bool,
    show_entry_gateways_only: bool,
) -> bool {
    let mut keep = filter.has_network_requester_sp;

    if show_entry_gateways_only {
        keep = true;
    }

    if show_only_wss {
        keep &= filter.has_wss;
    }
    if show_only_with_hostname {
        keep &= filter.has_hostname;
    }

    keep
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
    use crate::http::models::{Gateway, Service};
    use nym_node_requests::api::v1::node::models::NodeDescription;
    use serde_json::json;

    fn create_test_gateway(key: &str, has_wss: bool, has_network_requester: bool) -> Gateway {
        let mut self_described = json!({});
        if has_wss {
            self_described["mixnet_websockets"] = json!({ "wss_port": 1234 });
        }
        if has_network_requester {
            // ParsedDetails looks for these specific paths
            self_described["host_information"] = json!({
                "ip_address": ["192.168.1.1"],
                "hostname": "test.nymtech.net"
            });
            self_described["network_requester"] = json!({
                "address": "client123"
            });
        }
        
        Gateway {
            gateway_identity_key: key.to_string(),
            bonded: true,
            performance: 95,
            self_described: Some(self_described),
            explorer_pretty_bond: None,
            description: NodeDescription {
                moniker: "Test Gateway".to_string(),
                website: "".to_string(),
                security_contact: "".to_string(),
                details: "".to_string(),
            },
            last_probe_result: None,
            last_probe_log: None,
            last_testrun_utc: Some("2024-01-20T10:00:00Z".to_string()),
            last_updated_utc: "2024-01-20T11:00:00Z".to_string(),
            routing_score: 0.95,
            config_score: 100,
        }
    }

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
    
    #[test]
    fn test_apply_service_filters() {
        let filter_all = ServiceFilter {
            has_wss: true,
            has_network_requester_sp: true,
            has_hostname: true,
        };
        
        let filter_none = ServiceFilter {
            has_wss: false,
            has_network_requester_sp: false,
            has_hostname: false,
        };
        
        // Test default behavior (requires network_requester_sp)
        assert!(apply_service_filters(&filter_all, false, false, false));
        assert!(!apply_service_filters(&filter_none, false, false, false));
        
        // Test entry gateway mode (accepts all)
        assert!(apply_service_filters(&filter_all, false, false, true));
        assert!(apply_service_filters(&filter_none, false, false, true));
        
        // Test wss filter
        assert!(apply_service_filters(&filter_all, true, false, false));
        assert!(!apply_service_filters(&filter_none, true, false, false));
        
        // Test hostname filter
        assert!(apply_service_filters(&filter_all, false, true, false));
        assert!(!apply_service_filters(&filter_none, false, true, false));
        
        // Test combined filters
        assert!(apply_service_filters(&filter_all, true, true, false));
        assert!(!apply_service_filters(&filter_none, true, true, false));
        
        // Test entry mode does NOT override other filters - it just sets initial keep=true
        // But wss and hostname filters can still exclude items
        assert!(!apply_service_filters(&filter_none, true, true, true));
    }
    
    #[test]
    fn test_gateway_list_to_services() {
        let paths = ParseJsonPaths::new().unwrap();
        let gateways = vec![
            create_test_gateway("gw1", true, true),
            create_test_gateway("gw2", false, true),
            create_test_gateway("gw3", true, false),
            create_test_gateway("gw4", false, false),
        ];
        
        // Test no filters - only gateways with network_requester pass
        let params = ServicesQueryParams {
            size: None,
            page: None,
            wss: None,
            hostname: None,
            entry: None,
        };
        let services = gateway_list_to_services(&paths, gateways.clone(), params);
        assert_eq!(services.len(), 2); // gw1 and gw2 have network_requester
        assert!(services.iter().any(|s| s.gateway_identity_key == "gw1"));
        assert!(services.iter().any(|s| s.gateway_identity_key == "gw2"));
        
        // Test entry mode (accepts all)
        let params = ServicesQueryParams {
            size: None,
            page: None,
            wss: None,
            hostname: None,
            entry: Some(true),
        };
        let services = gateway_list_to_services(&paths, gateways.clone(), params);
        assert_eq!(services.len(), 4);
        
        // Test wss filter with entry mode
        let params = ServicesQueryParams {
            size: None,
            page: None,
            wss: Some(true),
            hostname: None,
            entry: Some(true),
        };
        let services = gateway_list_to_services(&paths, gateways.clone(), params);
        assert_eq!(services.len(), 2); // gw1 and gw3 have wss
        
        // Test hostname filter with entry mode
        let params = ServicesQueryParams {
            size: None,
            page: None,
            wss: None,
            hostname: Some(true),
            entry: Some(true),
        };
        let services = gateway_list_to_services(&paths, gateways.clone(), params);
        assert_eq!(services.len(), 2); // gw1 and gw2 have hostname
        
        // Test combined filters
        let params = ServicesQueryParams {
            size: None,
            page: None,
            wss: Some(true),
            hostname: Some(true),
            entry: Some(true),
        };
        let services = gateway_list_to_services(&paths, gateways, params);
        assert_eq!(services.len(), 1); // Only gw1 has both
        assert_eq!(services[0].gateway_identity_key, "gw1");
    }
    
    #[test]
    fn test_services_query_params_defaults() {
        let params = ServicesQueryParams {
            size: None,
            page: None,
            wss: None,
            hostname: None,
            entry: None,
        };
        
        assert_eq!(params.wss.unwrap_or(false), false);
        assert_eq!(params.hostname.unwrap_or(false), false);
        assert_eq!(params.entry.unwrap_or(false), false);
    }
    
    #[test]
    fn test_service_filter_edge_cases() {
        // Test with null wss_port value
        let service = Service {
            gateway_identity_key: "test".to_string(),
            last_updated_utc: "".to_string(),
            routing_score: 1.0,
            service_provider_client_id: Some("client".to_string()),
            ip_address: None,
            hostname: None,
            mixnet_websockets: Some(json!({ "wss_port": null })),
            last_successful_ping_utc: None,
        };
        let filter = ServiceFilter::new(&service);
        assert!(!filter.has_wss); // null port should be treated as no wss
        
        // Test with wss_port = 0
        let service2 = Service {
            gateway_identity_key: "test2".to_string(),
            last_updated_utc: "".to_string(),
            routing_score: 1.0,
            service_provider_client_id: None,
            ip_address: None,
            hostname: None,
            mixnet_websockets: Some(json!({ "wss_port": 0 })),
            last_successful_ping_utc: None,
        };
        let filter2 = ServiceFilter::new(&service2);
        assert!(filter2.has_wss); // Port 0 is still considered as having wss
    }
}