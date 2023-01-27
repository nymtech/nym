use crate::websocket::WebsocketConnectionError;
use socks5_requests::MessageError;

#[derive(thiserror::Error, Debug)]
pub enum NetworkRequesterError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Websocket error")]
    WebsocketConnectionError(#[from] WebsocketConnectionError),

    #[error("Websocket connection closed")]
    ConnectionClosed,

    #[error("encountered an error while trying to handle a provider request: {source}")]
    ProviderRequestError {
        #[from]
        source: MessageError,
    },
}
