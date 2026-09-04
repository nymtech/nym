// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use time::OffsetDateTime;

/// A final-hop payload the gateway pushed into this session, bundled with its arrival time.
///
/// The stamp is taken where the frame is read, because the payload's round trip IS the measurement
/// and nothing downstream can recover the moment it arrived.
pub(crate) struct ReceivedPayload {
    /// UTC timestamp at which the payload was pulled off the websocket.
    pub(crate) received_at: OffsetDateTime,

    /// Plaintext of a `PushedMixMessage`, which for our own test packet is the payload verbatim: an
    /// ack-sized final hop carries no SURB-Ack, so the node splits nothing off the front of it.
    pub(crate) payload: Vec<u8>,
}

impl ReceivedPayload {
    /// Wraps `payload` and stamps it with the current UTC time.
    pub(crate) fn new(payload: Vec<u8>) -> Self {
        ReceivedPayload {
            received_at: OffsetDateTime::now_utc(),
            payload,
        }
    }
}

/// Something that happened on ONE gateway client session.
///
/// Both phases of a gateway probe read this one channel, for different reasons: the ingress phase
/// only needs to know the gateway accepted what it forwarded, since those packets come back over the
/// mixnet listener, whereas the delivery phase's arrivals ARE these events.
pub(crate) enum SessionEvent {
    /// The gateway accepted a forwarded packet, reporting what the session has left.
    ///
    /// The figure is carried for the LOG, not as a budget to manage and not as a verdict on whether
    /// the session was metered. A metered session never reaches this event at all: it holds no
    /// bandwidth, so its very first forward is refused. Whether a packet got through is the whole of
    /// what a liveness probe measures, and how the gateway came to let it through is not.
    Accepted { remaining_bandwidth: i64 },

    /// A final-hop payload the gateway delivered into this session.
    Delivered(ReceivedPayload),

    /// The gateway refused a request, or sent a frame we could not open.
    ///
    /// Filed rather than fatal: a failure on one phase must not abort the run, so nothing here may
    /// tear the session down on its own.
    Refused(String),

    /// The session ended, carrying the transport's reason when it supplied one.
    Closed(Option<String>),
}

/// Sender half of ONE session's channel.
pub(crate) type SessionEventsSender = UnboundedSender<SessionEvent>;

/// Receiver half of ONE session's channel.
pub(crate) type SessionEventsReceiver = UnboundedReceiver<SessionEvent>;
