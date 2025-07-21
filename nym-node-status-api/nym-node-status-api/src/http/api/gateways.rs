use axum::{
    extract::{Path, Query, State},
    Json, Router,
};
use serde::Deserialize;
use utoipa::IntoParams;

use crate::http::{
    error::{HttpError, HttpResult},
    models::{Gateway, GatewaySkinny},
    state::AppState,
    PagedResult, Pagination,
};

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/", axum::routing::get(gateways))
        .route("/skinny", axum::routing::get(gateways_skinny))
        .route("/:identity_key", axum::routing::get(get_gateway))
}

#[utoipa::path(
    tag = "Gateways",
    get,
    params(
        Pagination
    ),
    path = "/v2/gateways",
    responses(
        (status = 200, body = PagedResult<Gateway>)
    )
)]
async fn gateways(
    Query(pagination): Query<Pagination>,
    State(state): State<AppState>,
) -> HttpResult<Json<PagedResult<Gateway>>> {
    let db = state.db_pool();
    let res = state.cache().get_gateway_list(db).await;

    Ok(Json(PagedResult::paginate(pagination, res)))
}

#[utoipa::path(
    tag = "Gateways",
    get,
    params(
        Pagination
    ),
    path = "/v2/gateways/skinny",
    responses(
        (status = 200, body = PagedResult<GatewaySkinny>)
    )
)]
async fn gateways_skinny(
    Query(pagination): Query<Pagination>,
    State(state): State<AppState>,
) -> HttpResult<Json<PagedResult<GatewaySkinny>>> {
    let db = state.db_pool();
    let res = state.cache().get_gateway_list(db).await;
    let res: Vec<GatewaySkinny> = filter_bonded_gateways_to_skinny(res);

    Ok(Json(PagedResult::paginate(pagination, res)))
}

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Path)]
struct IdentityKeyParam {
    identity_key: String,
}

#[utoipa::path(
    tag = "Gateways",
    get,
    params(
        IdentityKeyParam
    ),
    path = "/v2/gateways/{identity_key}",
    responses(
        (status = 200, body = Gateway)
    )
)]
async fn get_gateway(
    Path(IdentityKeyParam { identity_key }): Path<IdentityKeyParam>,
    State(state): State<AppState>,
) -> HttpResult<Json<Gateway>> {
    let db = state.db_pool();
    let res = state.cache().get_gateway_list(db).await;

    match res
        .iter()
        .find(|item| item.gateway_identity_key == identity_key)
    {
        Some(res) => Ok(Json(res.clone())),
        None => Err(HttpError::invalid_input(identity_key)),
    }
}

// Extract filtering logic for testing
fn filter_bonded_gateways_to_skinny(gateways: Vec<Gateway>) -> Vec<GatewaySkinny> {
    gateways
        .iter()
        .filter(|g| g.bonded)
        .map(|g| GatewaySkinny {
            gateway_identity_key: g.gateway_identity_key.clone(),
            self_described: g.self_described.clone(),
            performance: g.performance,
            explorer_pretty_bond: g.explorer_pretty_bond.clone(),
            last_probe_result: g.last_probe_result.clone(),
            last_testrun_utc: g.last_testrun_utc.clone(),
            last_updated_utc: g.last_updated_utc.clone(),
            routing_score: g.routing_score,
            config_score: g.config_score,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::models::Gateway;
    use nym_node_requests::api::v1::node::models::NodeDescription;

    fn create_test_gateway(identity_key: &str, bonded: bool, performance: u8) -> Gateway {
        Gateway {
            gateway_identity_key: identity_key.to_string(),
            bonded,
            performance,
            self_described: Some(serde_json::json!({"test": "data"})),
            explorer_pretty_bond: Some(serde_json::json!({"bond": "info"})),
            description: NodeDescription {
                moniker: "Test Gateway".to_string(),
                website: "".to_string(),
                security_contact: "".to_string(),
                details: "".to_string(),
            },
            last_probe_result: Some(serde_json::json!({"result": "ok"})),
            last_probe_log: None,
            last_testrun_utc: Some("2024-01-20T10:00:00Z".to_string()),
            last_updated_utc: "2024-01-20T11:00:00Z".to_string(),
            routing_score: 0.95,
            config_score: 100,
        }
    }

    #[test]
    fn test_filter_bonded_gateways_to_skinny_empty_list() {
        let gateways = vec![];
        let result = filter_bonded_gateways_to_skinny(gateways);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_filter_bonded_gateways_to_skinny_all_bonded() {
        let gateways = vec![
            create_test_gateway("gw1", true, 90),
            create_test_gateway("gw2", true, 95),
            create_test_gateway("gw3", true, 85),
        ];

        let result = filter_bonded_gateways_to_skinny(gateways);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].gateway_identity_key, "gw1");
        assert_eq!(result[1].gateway_identity_key, "gw2");
        assert_eq!(result[2].gateway_identity_key, "gw3");
    }

    #[test]
    fn test_filter_bonded_gateways_to_skinny_mixed() {
        let gateways = vec![
            create_test_gateway("gw1", true, 90),
            create_test_gateway("gw2", false, 95),
            create_test_gateway("gw3", true, 85),
            create_test_gateway("gw4", false, 100),
        ];

        let result = filter_bonded_gateways_to_skinny(gateways);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].gateway_identity_key, "gw1");
        assert_eq!(result[1].gateway_identity_key, "gw3");
    }

    #[test]
    fn test_filter_bonded_gateways_to_skinny_none_bonded() {
        let gateways = vec![
            create_test_gateway("gw1", false, 90),
            create_test_gateway("gw2", false, 95),
        ];

        let result = filter_bonded_gateways_to_skinny(gateways);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_gateway_to_skinny_conversion() {
        let gateway = create_test_gateway("test_gw", true, 98);
        let gateways = vec![gateway.clone()];

        let result = filter_bonded_gateways_to_skinny(gateways);
        assert_eq!(result.len(), 1);

        let skinny = &result[0];
        assert_eq!(skinny.gateway_identity_key, gateway.gateway_identity_key);
        assert_eq!(skinny.performance, gateway.performance);
        assert_eq!(skinny.self_described, gateway.self_described);
        assert_eq!(skinny.explorer_pretty_bond, gateway.explorer_pretty_bond);
        assert_eq!(skinny.last_probe_result, gateway.last_probe_result);
        assert_eq!(skinny.last_testrun_utc, gateway.last_testrun_utc);
        assert_eq!(skinny.last_updated_utc, gateway.last_updated_utc);
        assert_eq!(skinny.routing_score, gateway.routing_score);
        assert_eq!(skinny.config_score, gateway.config_score);
    }

    #[test]
    fn test_identity_key_param_deserialization() {
        let json = r#"{"identity_key": "test_key_123"}"#;
        let param: IdentityKeyParam = serde_json::from_str(json).unwrap();
        assert_eq!(param.identity_key, "test_key_123");
    }
}
