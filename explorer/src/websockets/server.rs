use std::{
    collections::HashMap,
    io::Error as IoError,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc::UnboundedSender,
};
use tokio_tungstenite::tungstenite::Message;

type Tx = UnboundedSender<Message>;
pub type ClientMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;

pub struct DashboardWebsocketServer {
    clients: ClientMap,
    addr: String,
}

impl DashboardWebsocketServer {
    pub fn new(clients: ClientMap, port: String) -> DashboardWebsocketServer {
        let addr = format!("0.0.0.0:{}", port);
        DashboardWebsocketServer { clients, addr }
    }

    pub async fn start(&self) -> Result<(), IoError> {
        let try_socket = TcpListener::bind(&self.addr).await;
        let listener = try_socket.expect("websocket listener startup failed");
        println!("starting to listen on {}", self.addr);
        while let Ok((stream, addr)) = listener.accept().await {
            tokio::spawn(self.handle_connection(self.clients.clone(), stream, addr));
        }

        Ok(())
    }

    async fn handle_connection(&self, client: ClientMap, stream: TcpStream, addr: SocketAddr) {
        println!("client connected");
        // set up channels so that when something comes from the metrics server it gets copied to this client
    }
}
