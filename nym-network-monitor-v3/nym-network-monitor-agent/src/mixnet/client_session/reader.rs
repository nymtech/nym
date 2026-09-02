// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

// SCAFFOLD: the bodies land as group 9's tasks are worked through, at which point both allows come off
#![allow(dead_code, unused_variables)]

use crate::mixnet::client_session::events::{SessionEvent, SessionEventsSender};
use futures::Stream;
use nym_gateway_requests::SharedSymmetricKey;
use nym_task::ShutdownToken;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::{Error as WsError, Message};

/// The read half of one client session, turning websocket frames into [`SessionEvent`]s.
///
/// Generic over the stream for the same reason the mixnet's connection handler is: a test drives it
/// with `stream::iter` and never binds a socket.
///
/// It never returns an error and never closes the session on its own. Everything it observes,
/// including a refusal or the gateway hanging up, is filed on the channel: a probe scores what
/// arrived, so a failure is a result rather than an abort.
pub(crate) struct SessionReader {
    /// Key the registration handshake derived, used here to open inbound frames.
    shared_key: Arc<SharedSymmetricKey>,

    /// Where everything this reader observes is filed.
    events: SessionEventsSender,
}

impl SessionReader {
    pub(crate) fn new(shared_key: Arc<SharedSymmetricKey>, events: SessionEventsSender) -> Self {
        SessionReader { shared_key, events }
    }

    /// Reads until the stream ends, the gateway closes the session, or shutdown is signalled,
    /// filing one event per frame that carries something measurable.
    pub(crate) async fn handle_stream<S>(self, stream: S, shutdown: ShutdownToken)
    where
        S: Stream<Item = Result<Message, WsError>> + Unpin,
    {
        // 1. select over the stream and the shutdown token
        // 2. classify each frame, filing what it carried
        // 3. stop on a `Closed`, on the receiver going away, or on shutdown
        todo!()
    }

    /// Spawns [`handle_stream`](Self::handle_stream) so the probe can send while it reads.
    pub(crate) fn spawn<S>(self, stream: S, shutdown: ShutdownToken) -> JoinHandle<()>
    where
        S: Stream<Item = Result<Message, WsError>> + Unpin + Send + 'static,
    {
        todo!()
    }

    /// Classifies one frame.
    ///
    /// `None` for frames that carry nothing we measure: tungstenite answers ping and pong itself, so
    /// this only has to avoid mistaking them for data.
    fn classify(&self, message: Message) -> Option<SessionEvent> {
        match message {
            // a `PushedMixMessage`, encrypted under the session key
            Message::Binary(blob) => todo!(),
            Message::Text(text) => self.classify_control(text),
            Message::Close(frame) => todo!(),
            _ => None,
        }
    }

    /// Classifies a control frame: a send acknowledgement, a refusal, or something to ignore.
    ///
    /// The typed out-of-bandwidth error matters more than its message text, because it is the one
    /// response that says the session was metered, i.e. that this gateway holds no announced identity
    /// for us. It must not collapse into an anonymous refusal.
    fn classify_control(&self, text: String) -> Option<SessionEvent> {
        todo!()
    }
}
