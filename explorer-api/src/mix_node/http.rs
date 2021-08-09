// use rocket::serde::json::Json;
// use rocket::{Route, State};
//
// use crate::mix_node::models::NodeDescription;
// use crate::state::ExplorerApiStateContext;

/*pub fn mix_node_make_default_routes() -> Vec<Route> {
    routes_with_openapi![get]
}

#[openapi(tag = "ping")]
#[get("/<pubkey>/description")]
pub(crate) async fn get_description(
    pubkey: &str,
    state: &State<ExplorerApiStateContext>,
) -> Option<Json<NodeDescription>> {
}

async fn get_mix_node(pubkey: &str) {
    match state.inner.ping_cache.clone().get(pubkey.to_string()).await {
        Some(cache_value) => {
            trace!("Returning cached value for {}", pubkey);
            Some(Json(PingResponse {
                ports: cache_value.ports,
            }))
        }
        None => {
            trace!("No cache value for {}", pubkey);
            let mix_nodes = state.inner.mix_nodes.clone().get().await;

            trace!("Getting mix node {}", pubkey);

            match get_mix_node(pubkey, mix_nodes) {
                Some(bond) => {
                    let mut ports: HashMap<u16, bool> = HashMap::new();

                    let ports_to_test = vec![
                        bond.mix_node.http_api_port,
                        bond.mix_node.mix_port,
                        bond.mix_node.verloc_port,
                    ];

                    trace!(
                        "Testing mix node {} on ports {:?}...",
                        pubkey,
                        ports_to_test
                    );

                    for port in ports_to_test {
                        ports.insert(port, do_port_check(&bond.mix_node.host, &port).await);
                    }

                    trace!("Tested mix node {}: {:?}", pubkey, ports);

                    let response = PingResponse { ports };

                    // cache for 1 min
                    trace!("Caching value for {}", pubkey);
                    state
                        .inner
                        .ping_cache
                        .clone()
                        .set(pubkey.to_string(), response.clone())
                        .await;

                    // return response
                    Some(Json(response))
                }
                None => Option::None,
            }
        }
    }
}
*/
