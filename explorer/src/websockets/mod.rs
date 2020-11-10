use std::{collections::HashMap, sync::Mutex};

use client::MetricsWebsocketClient;
use server::DashboardWebsocketServer;

pub(crate) mod client;
mod server;

pub async fn subscribe(metrics_socket: &str) {
    match MetricsWebsocketClient::connect(metrics_socket).await {
        Ok(_) => (),
        Err(e) => println!("metrics websocket failed to connect: {:?}", e),
    };
}

pub async fn listen(port: &str) {
    let clients = server::ClientMap::new(Mutex::new(HashMap::new()));
    let server = DashboardWebsocketServer::new(clients, port.to_string());
    server.start();
}
