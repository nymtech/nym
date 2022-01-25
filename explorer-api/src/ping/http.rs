use std::collections::HashMap;
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Duration;

use rocket::serde::json::Json;
use rocket::{Route, State};
use rocket_okapi::okapi::openapi3::OpenApi;
use rocket_okapi::openapi_get_routes_spec;
use rocket_okapi::settings::OpenApiSettings;

use mixnet_contract_common::MixNodeBond;

use crate::ping::models::PingResponse;
use crate::state::ExplorerApiStateContext;

const CONNECTION_TIMEOUT_SECONDS: Duration = Duration::from_secs(10);

pub fn ping_make_default_routes(settings: &OpenApiSettings) -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![settings: index]
}

#[openapi(tag = "ping")]
#[get("/<pubkey>")]
pub(crate) async fn index(
    pubkey: &str,
    state: &State<ExplorerApiStateContext>,
) -> Option<Json<PingResponse>> {
    match state.inner.ping.clone().get(pubkey).await {
        Some(cache_value) => {
            trace!("Returning cached value for {}", pubkey);
            Some(Json(PingResponse {
                pending: cache_value.pending,
                ports: cache_value.ports,
            }))
        }
        None => {
            trace!("No cache value for {}", pubkey);

            match state.inner.get_mix_node(pubkey).await {
                Some(bond) => {
                    // set status to pending, so that any HTTP requests are pending
                    state.inner.ping.set_pending(pubkey).await;

                    // do the check
                    let ports = Some(port_check(&bond).await);
                    trace!("Tested mix node {}: {:?}", pubkey, ports);
                    let response = PingResponse {
                        ports,
                        pending: false,
                    };

                    // cache for 1 min
                    trace!("Caching value for {}", pubkey);
                    state.inner.ping.set(pubkey, response.clone()).await;

                    // return response
                    Some(Json(response))
                }
                None => None,
            }
        }
    }
}

async fn port_check(bond: &MixNodeBond) -> HashMap<u16, bool> {
    let mut ports: HashMap<u16, bool> = HashMap::new();

    let ports_to_test = vec![
        bond.mix_node.http_api_port,
        bond.mix_node.mix_port,
        bond.mix_node.verloc_port,
    ];

    trace!(
        "Testing mix node {} on ports {:?}...",
        bond.mix_node.identity_key,
        ports_to_test
    );

    for port in ports_to_test {
        ports.insert(port, do_port_check(&bond.mix_node.host, port).await);
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
    let parsed_host = format!("{}:{}", trimmed_host, port);
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
            Ok(Err(_stream_err)) => {
                warn!("{} ping failed {:}", addr, _stream_err);
                // didn't timeout but couldn't open tcp stream
                false
            }
            Err(_timeout) => {
                // timed out
                warn!("{} timed out {:}", addr, _timeout);
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
