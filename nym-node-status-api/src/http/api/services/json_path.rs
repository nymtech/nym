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
