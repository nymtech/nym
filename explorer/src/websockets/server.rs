use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use tokio::{net::TcpStream, sync::mpsc::UnboundedSender};
use tokio_native_tls::TlsStream;
use tokio_tungstenite::tungstenite::{Error as WsError, Message};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::{connect_async, stream::Stream};

type Tx = UnboundedSender<Message>;
type ClientMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;

struct DashboardWebsocket {
    clients: ClientMap,
}
