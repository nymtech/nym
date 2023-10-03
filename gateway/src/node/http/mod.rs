use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use log::info;
use tokio::sync::RwLock;

mod client_registry;
use client_registry::*;

use super::ClientRegistry;

const ROUTE_PREFIX: &str = "/api/v1/gateway/client-interfaces/wireguard";

pub(crate) async fn start_http_api(client_registry: Arc<RwLock<ClientRegistry>>) {
    info!("Started HTTP API on port 80");

    let client_registry = Arc::clone(&client_registry);

    let routes = Router::new()
        .route(&format!("{ROUTE_PREFIX}/register"), post(register))
        .route(&format!("{ROUTE_PREFIX}/clients"), get(clients))
        .route(&format!("{ROUTE_PREFIX}/client/:pub_key"), get(client))
        .with_state(Arc::clone(&client_registry));

    #[allow(clippy::unwrap_used)]
    axum::Server::bind(&"0.0.0.0:80".parse().unwrap())
        .serve(routes.into_make_service())
        .await
        .unwrap();
}
