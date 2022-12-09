use crate::websocket::WebsocketConnectionError;

#[derive(thiserror::Error, Debug)]
pub enum NetworkRequesterError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Websocket error")]
    WebsocketConnectionError(#[from] WebsocketConnectionError),

    #[error("Websocket connection closed")]
    ConnectionClosed,
}
