use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use tokio::sync::mpsc::UnboundedSender;

use tokio_tungstenite::tungstenite::Message;

type Tx = UnboundedSender<Message>;
type ClientMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;

struct DashboardWebsocket {
    clients: ClientMap,
}
