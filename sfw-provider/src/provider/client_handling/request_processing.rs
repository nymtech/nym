use crypto::encryption;
use std::sync::Arc;

// PacketProcessor contains all data required to correctly process client requests
#[derive(Clone)]
pub struct RequestProcessor {
    secret_key: Arc<encryption::PrivateKey>,
}

impl RequestProcessor {
    pub(crate) fn new(secret_key: encryption::PrivateKey) -> Self {
        RequestProcessor {
            secret_key: Arc::new(secret_key),
        }
    }
}
