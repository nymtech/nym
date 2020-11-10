use client::MetricsWebsocketClient;
use log::*;
use server::DashboardWebsocketServer;
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::Message;

pub(crate) mod client;
mod server;

pub async fn subscribe(metrics_socket: &str, sender: broadcast::Sender<Message>) {
    let mut ws_client = match MetricsWebsocketClient::connect(metrics_socket, sender).await {
        Ok(client) => client,
        Err(e) => {
            error!("metrics websocket failed to connect: {:?}", e);
            std::process::exit(1)
        }
    };

    ws_client.run().await;
}

pub async fn listen(port: u16, sender: broadcast::Sender<Message>) {
    let server = DashboardWebsocketServer::new(port, sender);
    if let Err(err) = server.start().await {
        error!("failed to start dashboard websocket server! - {:?}", err);
        std::process::exit(1)
    }
}
