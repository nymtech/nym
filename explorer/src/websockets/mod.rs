use tokio_tungstenite::tungstenite::Error as WsError;

pub(crate) mod client;
mod server;

#[derive(Debug)]
enum WebsocketError {
    NetworkError(WsError),
}
