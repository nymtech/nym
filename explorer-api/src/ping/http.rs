// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_mixnet_contract_common::{NodeId, MixNode};
use rocket::serde::json::Json;
use rocket::{Route, State};
use rocket_okapi::okapi::openapi3::OpenApi;
use rocket_okapi::openapi_get_routes_spec;
use rocket_okapi::settings::OpenApiSettings;
use std::collections::HashMap;
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Duration;

use crate::ping::models::PingResponse;
use crate::state::ExplorerApiStateContext;

const CONNECTION_TIMEOUT_SECONDS: Duration = Duration::from_secs(10);

pub fn ping_make_default_routes(settings: &OpenApiSettings) -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![settings: index]
}

#[openapi(tag = "ping")]
#[get("/<mix_id>")]
pub(crate) async fn index(
    mix_id: NodeId,
    state: &State<ExplorerApiStateContext>,
) -> Option<Json<PingResponse>> {
    match state.inner.ping.clone().get(mix_id).await {
        Some(cache_value) => {
            trace!("Returning cached value for {}", mix_id);
            Some(Json(PingResponse {
                pending: cache_value.pending,
                ports: cache_value.ports,
            }))
        }
        None => {
            trace!("No cache value for {}", mix_id);

            match state.inner.get_mix_node(mix_id).await {
                Some(node) => {
                    // set status to pending, so that any HTTP requests are pending
                    state.inner.ping.set_pending(mix_id).await;

                    // do the check
                    let ports = Some(port_check(node.mix_node()).await);
                    trace!("Tested mix node {}: {:?}", mix_id, ports);
                    let response = PingResponse {
                        ports,
                        pending: false,
                    };

                    // cache for 1 min
                    trace!("Caching value for {}", mix_id);
                    state.inner.ping.set(mix_id, response.clone()).await;

                    // return response
                    Some(Json(response))
                }
                None => None,
            }
        }
    }
}

async fn port_check(mix_node: &MixNode) -> HashMap<u16, bool> {
    let mut ports: HashMap<u16, bool> = HashMap::new();

    let ports_to_test = vec![
        mix_node.http_api_port,
        mix_node.mix_port,
        mix_node.verloc_port,
    ];

    trace!(
        "Testing mix node {} on ports {:?}...",
        mix_node.identity_key,
        ports_to_test
    );

    for port in ports_to_test {
        ports.insert(port, do_port_check(&mix_node.host, port).await);
    }

    ports
}

fn sanitize_and_resolve_host(host: &str, port: u16) -> Option<SocketAddr> {
    // trim the host
    let trimmed_host = host.trim();

    // host must be at least one non-whitespace character
    if trimmed_host.is_empty() {
        return None;
    }

    // the host string should hopefully parse and resolve into a valid socket address
    let parsed_host = format!("{trimmed_host}:{port}");
    match parsed_host.to_socket_addrs() {
        Ok(mut addrs) => addrs.next(),
        Err(e) => {
            warn!(
                "Failed to resolve {}:{} -> {}. Error: {}",
                host, port, parsed_host, e
            );
            None
        }
    }
}

async fn do_port_check(host: &str, port: u16) -> bool {
    match sanitize_and_resolve_host(host, port) {
        Some(addr) => match tokio::time::timeout(
            CONNECTION_TIMEOUT_SECONDS,
            tokio::net::TcpStream::connect(addr),
        )
        .await
        {
            Ok(Ok(_stream)) => {
                // didn't timeout and tcp stream is open
                trace!("Successfully pinged {}", addr);
                true
            }
            Ok(Err(stream_err)) => {
                warn!("{} ping failed {:}", addr, stream_err);
                // didn't timeout but couldn't open tcp stream
                false
            }
            Err(timeout) => {
                // timed out
                warn!("{} timed out {:}", addr, timeout);
                false
            }
        },
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_host_with_valid_ip_address_returns_some() {
        assert!(sanitize_and_resolve_host("8.8.8.8", 1234).is_some());
        assert!(sanitize_and_resolve_host("2001:4860:4860::8888", 1234).is_some());
    }

    #[test]
    fn resolve_host_with_valid_hostname_returns_some() {
        assert!(sanitize_and_resolve_host("nymtech.net", 1234).is_some());
    }

    #[test]
    fn resolve_host_with_malformed_ip_address_returns_none() {
        // these are invalid ip addresses
        assert!(sanitize_and_resolve_host("192.168.1.999", 1234).is_none());
        assert!(sanitize_and_resolve_host("10.999.999.999", 1234).is_none());
    }

    #[test]
    fn resolve_host_with_unknown_hostname_returns_none() {
        assert!(sanitize_and_resolve_host(
            "some-unknown-hostname-that-will-never-resolve.nymtech.net",
            1234
        )
        .is_none());
    }

    #[test]
    fn resolve_host_with_bad_strings_return_none() {
        assert!(sanitize_and_resolve_host("", 1234).is_none());
        assert!(sanitize_and_resolve_host(" ", 1234).is_none());
        assert!(sanitize_and_resolve_host(" 🤘 ", 1234).is_none());
        assert!(sanitize_and_resolve_host("🤘", 1234).is_none());
        assert!(sanitize_and_resolve_host("@", 1234).is_none());
        assert!(sanitize_and_resolve_host("*", 1234).is_none());
    }
}
