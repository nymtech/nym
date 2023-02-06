#[derive(thiserror::Error, Debug)]
pub enum NetworkRequesterError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Websocket connection closed")]
    ConnectionClosed,
}
