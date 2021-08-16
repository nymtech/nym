use std::collections::HashMap;
use std::net::ToSocketAddrs;
use std::time::Duration;

use rocket::serde::json::Json;
use rocket::{Route, State};

use crate::ping::models::PingResponse;
use crate::state::ExplorerApiStateContext;
use mixnet_contract::MixNodeBond;

const CONNECTION_TIMEOUT_SECONDS: Duration = Duration::from_secs(10);

pub fn ping_make_default_routes() -> Vec<Route> {
    routes_with_openapi![index]
}

#[openapi(tag = "ping")]
#[get("/<pubkey>")]
pub(crate) async fn index(
    pubkey: &str,
    state: &State<ExplorerApiStateContext>,
) -> Option<Json<PingResponse>> {
    match state.inner.ping_cache.clone().get(pubkey).await {
        Some(cache_value) => {
            trace!("Returning cached value for {}", pubkey);
            Some(Json(PingResponse {
                ports: cache_value.ports,
            }))
        }
        None => {
            trace!("No cache value for {}", pubkey);

            match state.inner.get_mix_node(pubkey).await {
                Some(bond) => {
                    let ports = port_check(&bond).await;

                    trace!("Tested mix node {}: {:?}", pubkey, ports);

                    let response = PingResponse { ports };

                    // cache for 1 min
                    trace!("Caching value for {}", pubkey);
                    state.inner.ping_cache.set(pubkey, response.clone()).await;

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

async fn do_port_check(host: &str, port: u16) -> bool {
    let addr = format!("{}:{}", host, port)
        .to_socket_addrs()
        .unwrap()
        .next()
        .unwrap();
    match tokio::time::timeout(
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
            error!("{} ping failed {:}", addr, _stream_err);
            // didn't timeout but couldn't open tcp stream
            false
        }
        Err(_timeout) => {
            // timed out
            error!("{} timed out {:}", addr, _timeout);
            false
        }
    }
}
