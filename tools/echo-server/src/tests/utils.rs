use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProxiedMessage {
    pub message: Payload,
    session_id: Uuid,
    message_id: u16,
}

impl ProxiedMessage {
    pub fn new(message: Payload, session_id: Uuid, message_id: u16) -> Self {
        ProxiedMessage {
            message,
            session_id,
            message_id,
        }
    }

    pub fn message(&self) -> &Payload {
        &self.message
    }

    pub fn session_id(&self) -> Uuid {
        self.session_id
    }

    pub fn message_id(&self) -> u16 {
        self.message_id
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum Payload {
    Data(Vec<u8>),
    Close,
}

impl fmt::Display for ProxiedMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let message = match self.message() {
            Payload::Data(ref data) => format!("Data({})", data.len()),
            Payload::Close => "Close".to_string(),
        };
        write!(
            f,
            "ProxiedMessage {{ message: {}, session_id: {}, message_id: {} }}",
            message,
            self.session_id(),
            self.message_id()
        )
    }
}
