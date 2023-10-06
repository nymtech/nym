use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use log::info;
use tokio::sync::RwLock;

mod api;
use api::v1::client_registry::*;

use super::ClientRegistry;

const ROUTE_PREFIX: &str = "/api/v1/gateway/client-interfaces/wireguard";

pub(crate) async fn start_http_api(client_registry: Arc<RwLock<ClientRegistry>>) {
    // Port should be 80 post smoosh
    let port = 8000;

    info!("Started HTTP API on port {}", port);

    let client_registry = Arc::clone(&client_registry);

    let routes = Router::new()
        .route(&format!("{ROUTE_PREFIX}/clients"), get(get_all_clients))
        .route(&format!("{ROUTE_PREFIX}/client"), post(register_client))
        .route(&format!("{ROUTE_PREFIX}/client/:pub_key"), get(get_client))
        .with_state(Arc::clone(&client_registry));

    #[allow(clippy::unwrap_used)]
    axum::Server::bind(&format!("0.0.0.0:{}", port).parse().unwrap())
        .serve(routes.into_make_service())
        .await
        .unwrap();
}
