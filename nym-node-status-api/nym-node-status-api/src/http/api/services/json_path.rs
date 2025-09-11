use serde_json_path::JsonPath;

use crate::http::models::Gateway;

pub(super) struct ParseJsonPaths {
    pub(super) path_ip_address: JsonPath,
    pub(super) path_hostname: JsonPath,
    pub(super) path_service_provider_client_id: JsonPath,
}

impl ParseJsonPaths {
    pub fn new() -> Result<Self, serde_json_path::ParseError> {
        Ok(ParseJsonPaths {
            path_ip_address: JsonPath::parse("$.host_information.ip_address[0]")?,
            path_hostname: JsonPath::parse("$.host_information.hostname")?,
            path_service_provider_client_id: JsonPath::parse("$.network_requester.address")?,
        })
    }
}

pub(super) struct ParsedDetails {
    pub(super) ip_address: Option<String>,
    pub(super) hostname: Option<String>,
    pub(super) service_provider_client_id: Option<String>,
}

impl ParsedDetails {
    fn get_string_from_json_path(
        value: &Option<serde_json::Value>,
        path: &JsonPath,
    ) -> Option<String> {
        match value {
            Some(value) => path
                .query(value)
                .exactly_one()
                .map(|v2| v2.as_str().map(|v3| v3.to_string()))
                .ok()
                .flatten(),
            None => None,
        }
    }
    pub fn new(paths: &ParseJsonPaths, g: &Gateway) -> ParsedDetails {
        ParsedDetails {
            hostname: ParsedDetails::get_string_from_json_path(
                &g.self_described,
                &paths.path_hostname,
            ),
            ip_address: ParsedDetails::get_string_from_json_path(
                &g.self_described,
                &paths.path_ip_address,
            ),
            service_provider_client_id: ParsedDetails::get_string_from_json_path(
                &g.self_described,
                &paths.path_service_provider_client_id,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::models::Gateway;
    use nym_node_requests::api::v1::node::models::NodeDescription;
    use serde_json::json;

    fn create_mock_gateway(self_described: Option<serde_json::Value>) -> Gateway {
        Gateway {
            gateway_identity_key: "mock_identity".to_string(),
            bonded: true,
            performance: 100,
            self_described,
            explorer_pretty_bond: None,
            description: NodeDescription {
                moniker: "mock_moniker".to_string(),
                website: "https://nymtech.net".to_string(),
                security_contact: "security@nymtech.net".to_string(),
                details: "mock_details".to_string(),
            },
            last_probe_result: None,
            last_probe_log: None,
            last_testrun_utc: None,
            last_updated_utc: "2025-01-01T12:00:00Z".to_string(),
            routing_score: 1.0,
            config_score: 100,
        }
    }

    #[test]
    fn test_parse_json_paths() {
        let paths = ParseJsonPaths::new().unwrap();
        assert_eq!(
            paths.path_ip_address.to_string(),
            "$.host_information.ip_address[0]"
        );
        assert_eq!(
            paths.path_hostname.to_string(),
            "$.host_information.hostname"
        );
        assert_eq!(
            paths.path_service_provider_client_id.to_string(),
            "$.network_requester.address"
        );
    }

    #[test]
    fn test_parsed_details() {
        let paths = ParseJsonPaths::new().unwrap();

        // Test with full data
        let gateway1 = create_mock_gateway(Some(json!({
            "host_information": {
                "ip_address": ["1.1.1.1"],
                "hostname": "nymtech.net"
            },
            "network_requester": {
                "address": "client_address.sP"
            }
        })));
        let details1 = ParsedDetails::new(&paths, &gateway1);
        assert_eq!(details1.ip_address, Some("1.1.1.1".to_string()));
        assert_eq!(details1.hostname, Some("nymtech.net".to_string()));
        assert_eq!(
            details1.service_provider_client_id,
            Some("client_address.sP".to_string())
        );

        // Test with missing data
        let gateway2 = create_mock_gateway(Some(json!({})));
        let details2 = ParsedDetails::new(&paths, &gateway2);
        assert_eq!(details2.ip_address, None);
        assert_eq!(details2.hostname, None);
        assert_eq!(details2.service_provider_client_id, None);

        // Test with no self_described field
        let gateway3 = create_mock_gateway(None);
        let details3 = ParsedDetails::new(&paths, &gateway3);
        assert_eq!(details3.ip_address, None);
        assert_eq!(details3.hostname, None);
        assert_eq!(details3.service_provider_client_id, None);
    }
}
