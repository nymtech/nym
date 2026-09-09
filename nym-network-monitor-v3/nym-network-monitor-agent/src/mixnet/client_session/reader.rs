// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet::client_session::events::{ReceivedPayload, SessionEvent, SessionEventsSender};
use futures::{Stream, StreamExt};
use nym_gateway_requests::{BinaryResponse, SendResponse, ServerResponse, SharedSymmetricKey};
use nym_task::ShutdownToken;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::{Error as WsError, Message};
use tracing::{debug, trace};

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
    pub(crate) async fn handle_stream<S>(self, mut stream: S, shutdown: ShutdownToken)
    where
        S: Stream<Item = Result<Message, WsError>> + Unpin,
    {
        loop {
            let message = tokio::select! {
                _ = shutdown.cancelled() => {
                    debug!("the session reader was shut down");
                    return;
                }
                message = stream.next() => message,
            };

            let event = match message {
                // the gateway went away without a close frame
                None => SessionEvent::Closed(None),
                Some(Err(err)) => SessionEvent::Closed(Some(err.to_string())),
                Some(Ok(message)) => match self.classify(message) {
                    Some(event) => event,
                    // carried nothing we measure
                    None => continue,
                },
            };

            // the session is over once it is closed, so nothing further can arrive to be filed. taken
            // BEFORE the send, which consumes the event
            let ended = matches!(event, SessionEvent::Closed(_));

            if self.events.unbounded_send(event).is_err() {
                // the probe finished and dropped its inbox, which is the ordinary end of a run
                trace!("the session's inbox is gone, so the reader is stopping");
                return;
            }

            if ended {
                return;
            }
        }
    }

    /// Spawns [`handle_stream`](Self::handle_stream) so the probe can send while it reads.
    pub(crate) fn spawn<S>(self, stream: S, shutdown: ShutdownToken) -> JoinHandle<()>
    where
        S: Stream<Item = Result<Message, WsError>> + Unpin + Send + 'static,
    {
        tokio::spawn(async move { self.handle_stream(stream, shutdown).await })
    }

    /// Classifies one frame.
    ///
    /// `None` for frames that carry nothing we measure: tungstenite answers ping and pong itself, so
    /// this only has to avoid mistaking them for data.
    fn classify(&self, message: Message) -> Option<SessionEvent> {
        match message {
            // a `PushedMixMessage`, encrypted under the session key
            Message::Binary(blob) => Some(self.classify_pushed(blob)),
            Message::Text(text) => self.classify_control(text),
            Message::Close(frame) => Some(SessionEvent::Closed(frame.map(|frame| {
                format!(
                    "the gateway closed the session: {} ({})",
                    frame.reason, frame.code
                )
            }))),
            _ => None,
        }
    }

    /// Opens a binary frame, which on a live session is a final-hop payload the gateway delivered.
    fn classify_pushed(&self, blob: Vec<u8>) -> SessionEvent {
        match BinaryResponse::try_from_encrypted_tagged_bytes(blob, &self.shared_key) {
            Ok(BinaryResponse::PushedMixMessage { message }) => {
                // stamped HERE, where the frame came off the socket: the payload's round trip IS the
                // measurement and nothing downstream can recover the moment it arrived
                SessionEvent::Delivered(ReceivedPayload::new(message))
            }
            // `BinaryResponse` is non-exhaustive, so a gateway on a newer protocol may push something
            // this build does not know. not a delivery, and not fatal
            Ok(other) => SessionEvent::Refused(format!(
                "the gateway pushed an unrecognised binary response of kind {:?}",
                other.kind()
            )),
            Err(err) => SessionEvent::Refused(format!(
                "a frame pushed by the gateway could not be opened: {err}"
            )),
        }
    }

    /// Classifies a control frame: a send acknowledgement, a refusal, or something to ignore.
    ///
    /// A refusal keeps its full text, so the out-of-bandwidth case arrives with both figures rather
    /// than flattened to "error". That case is the ONLY signal that the session was metered, i.e.
    /// that this gateway holds no announced identity for us, so a run's zeros are then ours to
    /// explain rather than the gateway's: a metered session holds no bandwidth, so it is refused on
    /// its first forward and never reports an allowance anything could be inferred from.
    fn classify_control(&self, text: String) -> Option<SessionEvent> {
        let response = match ServerResponse::try_from(text) {
            Ok(response) => response,
            Err(err) => {
                return Some(SessionEvent::Refused(format!(
                    "the gateway sent a control frame that did not parse: {err}"
                )));
            }
        };

        match response {
            ServerResponse::Send(SendResponse {
                remaining_bandwidth,
                ..
            }) => Some(SessionEvent::Accepted {
                remaining_bandwidth,
            }),
            ServerResponse::TypedError { error } => Some(SessionEvent::Refused(format!(
                "the gateway reported: {error}"
            ))),
            ServerResponse::Error { message } => Some(SessionEvent::Refused(format!(
                "the gateway reported: {message}"
            ))),
            // registration and protocol negotiation are read directly during establishment, so
            // anything else arriving here is out of place rather than meaningful
            other => {
                debug!(
                    "ignoring a {} control frame on an established session",
                    other.name()
                );
                None
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::mixnet::client_session::events::SessionEventsReceiver;
    use futures::channel::mpsc::unbounded;
    use futures::stream;
    use nym_gateway_requests::SimpleGatewayRequestsError;
    use tokio_tungstenite::tungstenite::protocol::CloseFrame;
    use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;

    fn key() -> Arc<SharedSymmetricKey> {
        Arc::new(
            SharedSymmetricKey::try_from_bytes(&[42u8; 32]).expect("the test key was malformed"),
        )
    }

    /// A frame the gateway would push for a delivered final-hop payload.
    fn pushed(payload: &[u8]) -> Message {
        BinaryResponse::PushedMixMessage {
            message: payload.to_vec(),
        }
        .into_ws_message(&key())
        .expect("failed to seal the pushed message")
    }

    /// Runs the reader over a fixed set of frames and returns everything it filed.
    async fn events_from(messages: Vec<Message>) -> Vec<SessionEvent> {
        let (sender, receiver) = unbounded();
        let stream = stream::iter(messages.into_iter().map(Ok));

        SessionReader::new(key(), sender)
            .handle_stream(stream, ShutdownToken::new())
            .await;

        drain(receiver)
    }

    fn drain(mut receiver: SessionEventsReceiver) -> Vec<SessionEvent> {
        let mut events = Vec::new();
        while let Ok(event) = receiver.try_recv() {
            events.push(event);
        }
        events
    }

    // the delivery phase's whole measurement: a pushed payload, opened under the session key
    #[tokio::test]
    async fn a_pushed_payload_is_filed_as_delivered() {
        let events = events_from(vec![pushed(b"probe-payload")]).await;

        match events.as_slice() {
            [SessionEvent::Delivered(payload), SessionEvent::Closed(None)] => {
                assert_eq!(payload.payload, b"probe-payload".to_vec())
            }
            _ => panic!("a pushed payload was not filed as a delivery"),
        }
    }

    // the ingress phase counts what the gateway acknowledged, and reads the metering diagnosis off
    // the same acknowledgement
    #[tokio::test]
    async fn a_send_acknowledgement_is_filed_with_its_allowance() {
        let response = ServerResponse::Send(SendResponse {
            remaining_bandwidth: 4_611_686_018_427_387_903,
            upgrade_mode: false,
        });

        let events = events_from(vec![response.into()]).await;

        match events.as_slice() {
            [
                SessionEvent::Accepted {
                    remaining_bandwidth,
                },
                SessionEvent::Closed(None),
            ] => assert_eq!(*remaining_bandwidth, 4_611_686_018_427_387_903),
            _ => panic!("a send acknowledgement was not filed"),
        }
    }

    // an out-of-bandwidth refusal is what a METERED session hits, which is the signal that this
    // gateway never ingested our announced identity. both figures have to survive into the message
    #[tokio::test]
    async fn an_out_of_bandwidth_refusal_keeps_its_figures() {
        let response = ServerResponse::TypedError {
            error: SimpleGatewayRequestsError::OutOfBandwidth {
                required: 386,
                available: 0,
            },
        };

        let events = events_from(vec![response.into()]).await;

        match events.as_slice() {
            [SessionEvent::Refused(reason), SessionEvent::Closed(None)] => {
                assert!(reason.contains("386"), "{reason}");
                assert!(reason.contains('0'), "{reason}");
            }
            _ => panic!("a typed error was not filed as a refusal"),
        }
    }

    // a refusal must NOT end the session: a failure on one phase cannot abort the run, so the reader
    // has to keep filing after one
    #[tokio::test]
    async fn a_refusal_does_not_stop_the_reader() {
        let refusal = ServerResponse::Error {
            message: "nope".to_string(),
        };

        let events = events_from(vec![refusal.into(), pushed(b"after")]).await;

        assert!(matches!(
            events.as_slice(),
            [
                SessionEvent::Refused(_),
                SessionEvent::Delivered(_),
                SessionEvent::Closed(None)
            ]
        ));
    }

    // a close is terminal and is DATA: the probe has to be able to tell a session that ended from one
    // that simply delivered nothing
    #[tokio::test]
    async fn a_close_frame_ends_the_reader_and_is_filed_with_its_reason() {
        let close = Message::Close(Some(CloseFrame {
            code: CloseCode::Policy,
            reason: "unauthorised".into(),
        }));

        // the payload after the close must never be filed
        let events = events_from(vec![close, pushed(b"after-close")]).await;

        match events.as_slice() {
            [SessionEvent::Closed(Some(reason))] => {
                assert!(reason.contains("unauthorised"), "{reason}")
            }
            _ => panic!("a close frame did not end the reader"),
        }
    }

    // a frame we cannot open is a refusal rather than a delivery, so a wrong session key can never be
    // counted as a delivered packet
    #[tokio::test]
    async fn an_unopenable_frame_is_not_counted_as_a_delivery() {
        let events = events_from(vec![Message::Binary(vec![0u8; 32])]).await;

        assert!(matches!(
            events.as_slice(),
            [SessionEvent::Refused(_), SessionEvent::Closed(None)]
        ));
    }

    // tungstenite answers ping and pong itself, so they only have to not be mistaken for data
    #[tokio::test]
    async fn ping_and_pong_frames_carry_nothing() {
        let events = events_from(vec![
            Message::Ping(vec![1, 2, 3]),
            Message::Pong(vec![1, 2, 3]),
        ])
        .await;

        assert!(matches!(events.as_slice(), [SessionEvent::Closed(None)]));
    }

    // a transport error ends the session with its reason rather than being swallowed
    #[tokio::test]
    async fn a_transport_error_is_filed_as_a_close() {
        let (sender, receiver) = unbounded();
        let stream = stream::iter(vec![Err(WsError::ConnectionClosed)]);

        SessionReader::new(key(), sender)
            .handle_stream(stream, ShutdownToken::new())
            .await;

        match drain(receiver).as_slice() {
            [SessionEvent::Closed(Some(_))] => (),
            _ => panic!("a transport error was not filed as a close"),
        }
    }
}
