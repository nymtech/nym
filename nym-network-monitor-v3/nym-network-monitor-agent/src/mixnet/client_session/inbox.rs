// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

// SCAFFOLD: the bodies land as group 9's tasks are worked through, at which point both allows come off
#![allow(dead_code, unused_variables)]

use crate::mixnet::client_session::events::{
    ReceivedPayload, SessionEvent, SessionEventsReceiver, SessionEventsSender,
};
use std::time::Duration;

/// The receiving side of ONE gateway client session.
///
/// The same shape as [`TargetInbox`](crate::mixnet::inbox::TargetInbox): it owns the channel, files
/// the facts its transport reported, and hands out what arrived. Deliberately a separate type rather
/// than one generic over both, because a session's facts are its own (an allowance and a refusal,
/// against the mixnet side's Noise handshake) and nothing reads them together.
///
/// Unlike the mixnet inbox it performs NO sphinx recovery. The gateway is the final hop of a delivery
/// packet, so it has already unwrapped the payload before pushing it, which leaves nothing here for a
/// payload key to open: what arrives is the test payload's bytes, and only its content has to be
/// parsed.
pub(crate) struct GatewaySessionInbox {
    /// How long [`next_payload`](Self::next_payload) waits before reporting a timeout.
    receive_timeout: Duration,

    /// Sender half kept alive so the channel stays open as long as this inbox exists.
    sender: SessionEventsSender,

    /// Receiver half drained by [`next_payload`](Self::next_payload) and
    /// [`all_available`](Self::all_available).
    receiver: SessionEventsReceiver,

    /// How many forwards the gateway acknowledged, which bounds what the ingress phase may claim to
    /// have put into the network.
    accepted: usize,

    /// The allowance the gateway last reported. See [`SessionEvent::Accepted`]: a sentinel-sized
    /// figure is the exemption, a plausible one means this gateway metered us.
    reported_allowance: Option<i64>,

    /// Why the gateway refused something, if it did.
    refusal: Option<String>,

    /// Set once the session ended, holding the transport's reason when there was one.
    closed: Option<Option<String>>,
}

impl GatewaySessionInbox {
    pub(crate) fn new(receive_timeout: Duration) -> Self {
        todo!()
    }

    /// Returns a clone of the sender half, which is what this session's reader files onto.
    pub(crate) fn events_sender(&self) -> SessionEventsSender {
        self.sender.clone()
    }

    /// Waits for the next delivered payload, up to `receive_timeout`.
    ///
    /// Loops rather than treating the first event as a payload, since the channel also carries the
    /// session's own facts. The timeout bounds the whole wait, so a stream of acknowledgements cannot
    /// extend it.
    pub(crate) async fn next_payload(&mut self) -> anyhow::Result<ReceivedPayload> {
        todo!()
    }

    /// Drains everything currently available without blocking.
    pub(crate) fn all_available(&mut self) -> Vec<ReceivedPayload> {
        todo!()
    }

    /// Files one event, returning the payload it carried if it was one.
    fn record(&mut self, event: SessionEvent) -> Option<ReceivedPayload> {
        todo!()
    }

    /// How many forwards the gateway acknowledged.
    pub(crate) fn accepted(&self) -> usize {
        self.accepted
    }

    /// Whether this session appears to have been granted the unmetered monitor exemption.
    ///
    /// `None` until the gateway has reported an allowance at all, which is the honest answer for a
    /// session that never got a packet accepted. Keyed on the MAGNITUDE of the reported figure and
    /// not on whether sending worked: a gateway that does not enforce zk-nyms hands an ordinary
    /// metered session a large free-testnet allowance too, so a successful send is not evidence the
    /// exemption was granted.
    pub(crate) fn appears_unmetered(&self) -> Option<bool> {
        todo!()
    }

    /// Why the gateway refused something, if it did.
    pub(crate) fn refusal(&self) -> Option<&str> {
        self.refusal.as_deref()
    }

    /// Whether the session has ended, and why if the transport said.
    pub(crate) fn closed(&self) -> Option<Option<&str>> {
        todo!()
    }
}
