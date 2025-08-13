// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::message::to::{ToChannel, ToDirect};
use serde::{Deserialize, Serialize};

pub mod to;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "result")]
#[serde(rename_all = "snake_case")]
pub enum SendMessageResponse {
    Success {
        id: i64,
        automatic_new_visibility_policy: Option<i64>,
        msg: String,
    },
    Error {
        code: String,
        msg: String,
        stream: Option<String>,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum SendableMessageContent {
    // old name: 'private'
    Direct {
        // internally this is a list
        to: String,
        content: String,
    },
    // alternative name: 'channel'
    Stream {
        to: String,
        topic: Option<String>,
        content: String,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SendableMessage {
    #[serde(flatten)]
    content: SendableMessageContent,

    /// For clients supporting local echo, the event queue ID for the client.
    /// If passed, `local_id` is required. If the message is successfully sent,
    /// the server will include `local_id` in the message event that the client with this `queue_id`
    /// will receive notifying it of the new message via `GET /events`.
    /// This lets the client know unambiguously that it should replace the locally echoed message,
    /// rather than adding this new message
    /// (which would be correct if the user had sent the new message from another device).
    /// example: "fb67bf8a-c031-47cc-84cf-ed80accacda8"
    queue_id: Option<String>,

    /// For clients supporting local echo, a unique string-format identifier chosen freely by the client;
    /// the server will pass it back to the client without inspecting it, as described in the `queue_id` description.
    /// example: "100.01"
    local_id: Option<String>,

    /// Whether the message should be initially marked read by its sender.
    /// If unspecified, the server uses a heuristic based on the client name.
    read_by_sender: bool,
}

impl SendableMessage {
    pub fn new(content: impl Into<SendableMessageContent>) -> Self {
        SendableMessage {
            content: content.into(),
            queue_id: None,
            local_id: None,
            read_by_sender: false,
        }
    }

    #[must_use]
    pub fn with_queue(mut self, queue_id: impl Into<String>, local_id: impl Into<String>) -> Self {
        self.queue_id = Some(queue_id.into());
        self.local_id = Some(local_id.into());
        self
    }

    #[must_use]
    pub fn read_by_sender(mut self, read_by_sender: bool) -> Self {
        self.read_by_sender = read_by_sender;
        self
    }
}

pub type PrivateMessage = DirectMessage;

pub struct DirectMessage {
    to: String,
    content: String,
}

impl DirectMessage {
    pub fn new(to: impl Into<ToDirect>, content: impl Into<String>) -> Self {
        DirectMessage {
            to: to.into().to_string(),
            content: content.into(),
        }
    }
}

pub type ChannelMessage = StreamMessage;
pub struct StreamMessage {
    to: String,
    topic: Option<String>,
    content: String,
}

impl StreamMessage {
    pub fn new(
        to: impl Into<ToChannel>,
        content: impl Into<String>,
        topic: Option<String>,
    ) -> Self {
        StreamMessage {
            to: to.into().to_string(),
            topic,
            content: content.into(),
        }
    }

    pub fn no_topic(to: impl Into<ToChannel>, content: impl Into<String>) -> Self {
        Self::new(to, content, None)
    }

    #[must_use]
    pub fn with_topic(mut self, topic: impl Into<String>) -> Self {
        self.topic = Some(topic.into());
        self
    }
}

impl From<SendableMessageContent> for SendableMessage {
    fn from(content: SendableMessageContent) -> Self {
        SendableMessage::new(content)
    }
}

impl From<DirectMessage> for SendableMessage {
    fn from(msg: DirectMessage) -> Self {
        SendableMessageContent::from(msg).into()
    }
}

impl From<DirectMessage> for SendableMessageContent {
    fn from(msg: DirectMessage) -> Self {
        SendableMessageContent::Direct {
            to: msg.to,
            content: msg.content,
        }
    }
}

impl<T, S> From<(T, S)> for DirectMessage
where
    T: Into<ToDirect>,
    S: Into<String>,
{
    fn from((to, content): (T, S)) -> Self {
        DirectMessage::new(to, content)
    }
}

impl<T, S> From<(T, S)> for SendableMessage
where
    T: Into<ToDirect>,
    S: Into<String>,
{
    fn from((to, content): (T, S)) -> Self {
        DirectMessage::new(to, content).into()
    }
}

impl From<StreamMessage> for SendableMessage {
    fn from(msg: StreamMessage) -> Self {
        SendableMessageContent::from(msg).into()
    }
}

impl From<StreamMessage> for SendableMessageContent {
    fn from(msg: StreamMessage) -> Self {
        SendableMessageContent::Stream {
            to: msg.to,
            topic: msg.topic,
            content: msg.content,
        }
    }
}

impl<T, S> From<(T, S, Option<S>)> for StreamMessage
where
    T: Into<ToChannel>,
    S: Into<String>,
{
    fn from((to, content, topic): (T, S, Option<S>)) -> Self {
        StreamMessage::new(to, content, topic.map(Into::into))
    }
}

impl<T, S> From<(T, S, Option<S>)> for SendableMessage
where
    T: Into<ToChannel>,
    S: Into<String>,
{
    fn from(inner: (T, S, Option<S>)) -> Self {
        StreamMessage::from(inner).into()
    }
}
