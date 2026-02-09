// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Utility types for TCP proxy message handling and ordering.
//!
//! The Nym mixnet does not guarantee message ordering, so these utilities implement
//! session-based message reordering using sequence numbers and time-based decay.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fmt, ops::Deref, time::Instant};
use tokio::{io::AsyncWriteExt as _, net::tcp::OwnedWriteHalf};
use tracing::{debug, info};
use uuid::Uuid;

/// Default decay time in seconds before a message is considered "old" and processed regardless of order.
const DEFAULT_DECAY: u64 = 6;

/// A buffer for reordering out-of-order messages from the mixnet.
///
/// Messages arriving through the Nym mixnet may be received out of order due to
/// the probabilistic nature of mix node routing. `MessageBuffer` collects incoming
/// messages and reorders them based on their sequence numbers before writing to
/// the output stream.
///
/// ## Ordering Strategy
///
/// The buffer uses two strategies to determine when to deliver messages:
///
/// 1. **Sequence ordering**: Messages are delivered in sequence number order when
///    the expected next message arrives.
///
/// 2. **Decay-based delivery**: If a message has been waiting longer than
///    6 seconds (the default decay timeout), it is delivered even if
///    earlier messages haven't arrived. This prevents indefinite blocking when
///    messages are lost.
///
/// ## Usage
///
/// ```rust,ignore
/// let mut buffer = MessageBuffer::new();
///
/// // Push incoming messages (may be out of order)
/// buffer.push(message);
///
/// // Periodically tick to process and write ready messages
/// let should_close = buffer.tick(&mut write_half).await?;
/// ```
#[derive(Debug, Default)]
pub struct MessageBuffer {
    /// Buffered messages wrapped with timing information.
    buffer: Vec<DecayWrapper<ProxiedMessage>>,
    /// The next expected message ID in sequence.
    next_msg_id: u16,
}

impl MessageBuffer {
    /// Creates a new empty message buffer.
    pub fn new() -> Self {
        MessageBuffer {
            buffer: Vec::new(),
            next_msg_id: 0,
        }
    }

    /// Adds a message to the buffer for reordering.
    ///
    /// The message is wrapped in a [`DecayWrapper`] to track when it was received.
    pub fn push(&mut self, msg: ProxiedMessage) {
        self.buffer.push(DecayWrapper::new(msg));
    }

    /// Retains only messages that satisfy the predicate.
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&DecayWrapper<ProxiedMessage>) -> bool,
    {
        self.buffer.retain(f);
    }

    /// Returns the number of messages currently buffered.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Returns `true` if the buffer contains no messages.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Returns an iterator over buffered messages.
    pub fn iter(&self) -> std::slice::Iter<'_, DecayWrapper<ProxiedMessage>> {
        self.buffer.iter()
    }

    /// Processes buffered messages and writes ready ones to the output stream.
    ///
    /// This method should be called periodically (e.g., every 100ms) to process
    /// buffered messages. It:
    ///
    /// 1. Identifies messages ready for delivery (in sequence or decayed)
    /// 2. Sorts them by message ID
    /// 3. Writes data payloads to the output stream
    /// 4. Updates the expected next message ID
    /// 5. Removes delivered messages from the buffer
    ///
    /// # Returns
    ///
    /// - `Ok(true)` if a [`Payload::Close`] message was encountered, indicating
    ///   the session should be closed.
    /// - `Ok(false)` if processing completed normally.
    /// - `Err` if writing to the stream failed.
    pub async fn tick(&mut self, write: &mut OwnedWriteHalf) -> Result<bool> {
        if self.is_empty() {
            return Ok(false);
        }

        debug!("Messages in buffer:");
        for msg in self.iter() {
            debug!("{}", msg.inner());
        }

        // Iterate over self, filtering messages where msg.decayed() = true (aka message is older than DEFAULT_DECAY seconds), or where msg.message_id is less than next_msg_id. Then collect and order according to message_id.
        let mut send_buffer = self
            .iter()
            .filter(|msg| msg.decayed() || msg.message_id() <= self.next_msg_id)
            .map(|msg| msg.inner())
            .collect::<Vec<&ProxiedMessage>>();
        send_buffer.sort_by(|a, b| a.message_id.cmp(&b.message_id()));

        if send_buffer.is_empty() {
            debug!("send buf is empty");
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

/// A wrapper that tracks the age of a value since it was created.
///
/// `DecayWrapper` is used by [`MessageBuffer`] to implement time-based message
/// delivery. Messages that have been waiting longer than the decay threshold
/// are delivered even if they're out of sequence, preventing indefinite blocking
/// when earlier messages are lost in the network.
///
/// ## Decay Behavior
///
/// A wrapped value is considered "decayed" when it has existed for longer than
/// the decay duration (default: 6 seconds). The [`decayed`](Self::decayed) method
/// checks this condition.
#[derive(Debug)]
pub struct DecayWrapper<T> {
    /// The wrapped value.
    value: T,
    /// When this wrapper was created.
    start: Instant,
    /// Decay threshold in seconds.
    decay: u64,
}

impl<T> DecayWrapper<T> {
    /// Returns `true` if this wrapper has existed longer than the decay threshold.
    ///
    /// Used by [`MessageBuffer::tick`] to determine if a message should be
    /// delivered regardless of its sequence position.
    pub fn decayed(&self) -> bool {
        debug!("Decayed: {:?}", self.start.elapsed().as_secs() > self.decay);
        self.start.elapsed().as_secs() > self.decay
    }

    /// Creates a new decay wrapper around the given value.
    ///
    /// The decay timer starts immediately upon creation.
    pub fn new(value: T) -> Self {
        DecayWrapper {
            value,
            start: Instant::now(),
            decay: DEFAULT_DECAY,
        }
    }

    /// Consumes the wrapper and returns the inner value.
    #[allow(dead_code)]
    pub fn into_inner(self) -> T {
        self.value
    }

    /// Returns a reference to the inner value.
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

/// A message sent through the TCP proxy over the Nym mixnet.
///
/// `ProxiedMessage` encapsulates the data being proxied along with metadata
/// needed for session management and message ordering.
///
/// ## Fields
///
/// - `message`: The actual payload (data bytes or close signal)
/// - `session_id`: Unique identifier for this TCP session
/// - `message_id`: Sequence number for ordering messages within a session
///
/// ## Serialization
///
/// Messages are serialized using `bincode` for efficient transmission through
/// the mixnet.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProxiedMessage {
    /// The message payload.
    pub message: Payload,
    /// Unique session identifier (one per TCP connection).
    pub session_id: Uuid,
    /// Sequence number for message ordering within the session.
    pub message_id: u16,
}

impl ProxiedMessage {
    /// Creates a new proxied message.
    ///
    /// # Arguments
    ///
    /// * `message` - The payload to send.
    /// * `session_id` - The session this message belongs to.
    /// * `message_id` - The sequence number of this message within the session.
    pub fn new(message: Payload, session_id: Uuid, message_id: u16) -> Self {
        ProxiedMessage {
            message,
            session_id,
            message_id,
        }
    }

    /// Returns a reference to the message payload.
    pub fn message(&self) -> &Payload {
        &self.message
    }

    /// Returns the session ID this message belongs to.
    pub fn session_id(&self) -> Uuid {
        self.session_id
    }

    /// Returns the sequence number of this message.
    pub fn message_id(&self) -> u16 {
        self.message_id
    }
}

/// The payload of a proxied message.
///
/// Each message through the TCP proxy contains either actual data bytes
/// or a control signal to close the session.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Payload {
    /// Raw data bytes to be forwarded.
    Data(Vec<u8>),
    /// Signal to close the session.
    ///
    /// When received, the session handler should finish processing any
    /// remaining buffered messages and then close the connection.
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
