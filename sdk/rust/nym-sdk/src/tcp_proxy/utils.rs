use std::{collections::HashSet, fmt, ops::Deref, time::Instant};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::{io::AsyncWriteExt as _, net::tcp::OwnedWriteHalf};
use tracing::{debug, info};
use uuid::Uuid;

// Keeps track of
// - incoming and unsorted messages wrapped in DecayWrapper for keeping track of when they were received
// - the expected next message ID (reset on .tick())
#[derive(Debug)]
pub struct MessageBuffer {
    buffer: Vec<DecayWrapper<ProxiedMessage>>,
    next_msg_id: u16,
}

impl MessageBuffer {
    pub fn new() -> Self {
        MessageBuffer {
            buffer: Vec::new(),
            next_msg_id: 0,
        }
    }

    pub fn push(&mut self, msg: ProxiedMessage) {
        self.buffer.push(DecayWrapper::new(msg));
    }

    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&DecayWrapper<ProxiedMessage>) -> bool,
    {
        self.buffer.retain(f);
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<DecayWrapper<ProxiedMessage>> {
        self.buffer.iter()
    }

    // Used by the client to create and manipulate a buffer of messages to write => OwnedWriteHalf.
    // Used by the server for this + to conditionally decide whether to kill a session on returning true.
    // #[instrument]
    pub async fn tick(&mut self, write: &mut OwnedWriteHalf) -> Result<bool> {
        if self.is_empty() {
            return Ok(false);
        }

        debug!("Messages in buffer:");
        for msg in self.iter() {
            debug!("{}", msg.inner());
        }

        // Iterate over self, filtering messages where msg.decayed() = true (aka message is older than 2 seconds), or where msg.message_id is less than next_msg_id. Then collect and order according to message_id.
        let mut send_buffer = self
            .iter()
            .filter(|msg| msg.decayed() || msg.message_id() <= self.next_msg_id)
            .map(|msg| msg.inner())
            .collect::<Vec<&ProxiedMessage>>();
        send_buffer.sort_by(|a, b| a.message_id.cmp(&b.message_id()));

        if send_buffer.is_empty() {
            info!("send buf is empty");
            return Ok(false);
        }

        let mut sent_messages = HashSet::new();

        // Send collected & ordered messages down OwnedReadHalf until matching on Close enum, in which case exit & cause server to start session shutdown.
        for msg in send_buffer {
            match &msg.message() {
                Payload::Data(data) => {
                    write.write_all(data).await?;
                    info!("Wrote message {} to stream", msg.message_id())
                }
                Payload::Close => {
                    return Ok(true);
                }
            }
            // Store sent messages in hashmap.
            sent_messages.insert(msg.message_id());
        }

        // Iterate through sent, find the largest message ID and add 1, set this as expected next message ID.
        self.next_msg_id = sent_messages
            .iter()
            .max()
            .expect("This is safe since we know we've set something")
            + 1;
        // Retain as next_msg_id in MessageBuffer instance for parsing potential further incoming msgs.
        self.retain(|msg| !sent_messages.contains(&msg.inner().message_id()));
        info!("next_msg_id is: {}", self.next_msg_id.clone());
        Ok(false)
    }
}

// Wrapper used for tracking the 'age' of a message from when it was received.
// Used in the ordering logic in MessageBuffer.tick().
#[derive(Debug)]
pub struct DecayWrapper<T> {
    value: T,
    start: Instant,
    decay: u64,
}

impl<T> DecayWrapper<T> {
    pub fn decayed(&self) -> bool {
        debug!("Decayed: {:?}", self.start.elapsed().as_secs() > self.decay);
        self.start.elapsed().as_secs() > self.decay
    }

    pub fn new(value: T) -> Self {
        DecayWrapper {
            value,
            start: Instant::now(),
            decay: 6,
        }
    }

    #[allow(dead_code)]
    pub fn into_inner(self) -> T {
        self.value
    }

    pub fn inner(&self) -> &T {
        &self.value
    }
}

impl<T> Deref for DecayWrapper<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProxiedMessage {
    message: Payload,
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

#[derive(Serialize, Deserialize, Clone, Debug)]
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
