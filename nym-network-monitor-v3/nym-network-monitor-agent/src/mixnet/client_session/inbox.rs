// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet::client_session::events::{
    ReceivedPayload, SessionEvent, SessionEventsReceiver, SessionEventsSender,
};
use anyhow::Context;
use futures::StreamExt;
use futures::channel::mpsc::unbounded;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, warn};

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

    /// Why the gateway refused something, if it did.
    ///
    /// This is the WHOLE diagnosis of a metered session, and the reason no allowance figure is kept
    /// alongside it: our session never claims free bandwidth, so a gateway that has not granted the
    /// exemption refuses the very first forward and no acknowledgement ever arrives to read a figure
    /// off. Judging the allowance's magnitude would answer only in the case that needs no answering.
    refusal: Option<String>,

    /// Set once the session ended, holding the transport's reason when there was one.
    closed: Option<Option<String>>,
}

impl GatewaySessionInbox {
    pub(crate) fn new(receive_timeout: Duration) -> Self {
        let (sender, receiver) = unbounded();

        GatewaySessionInbox {
            receive_timeout,
            sender,
            receiver,
            accepted: 0,
            refusal: None,
            closed: None,
        }
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
        timeout(self.receive_timeout, async {
            loop {
                let event = self
                    .receiver
                    .next()
                    .await
                    .context("the session's event stream has been exhausted")?;

                if let Some(payload) = self.record(event) {
                    return Ok(payload);
                }
            }
        })
        .await
        .inspect_err(|_| {
            warn!(
                "timed out waiting for the gateway to deliver a payload after {}",
                humantime::format_duration(self.receive_timeout)
            )
        })?
    }

    /// Drains everything currently available without blocking.
    pub(crate) fn all_available(&mut self) -> Vec<ReceivedPayload> {
        let mut payloads = Vec::new();
        while let Ok(event) = self.receiver.try_recv() {
            if let Some(payload) = self.record(event) {
                payloads.push(payload);
            }
        }

        debug!("drained {} immediately available payloads", payloads.len());
        payloads
    }

    /// Files one event, returning the payload it carried if it was one.
    fn record(&mut self, event: SessionEvent) -> Option<ReceivedPayload> {
        match event {
            SessionEvent::Delivered(payload) => Some(payload),
            SessionEvent::Accepted {
                remaining_bandwidth,
            } => {
                // logged once, on the first acknowledgement: the figure is diagnostic colour for a
                // human reading a run, not something the probe acts on, and a line per packet would
                // bury the rest of the run
                if self.accepted == 0 {
                    debug!(
                        "the gateway accepted our first forward, reporting {remaining_bandwidth} byte(s) of remaining allowance"
                    );
                }
                self.accepted += 1;
                None
            }
            SessionEvent::Refused(reason) => {
                warn!("the gateway refused a request on this session: {reason}");
                self.refusal = Some(reason);
                None
            }
            SessionEvent::Closed(reason) => {
                self.closed = Some(reason);
                None
            }
        }
    }

    /// How many forwards the gateway acknowledged.
    pub(crate) fn accepted(&self) -> usize {
        self.accepted
    }

    /// Why the gateway refused something, if it did.
    pub(crate) fn refusal(&self) -> Option<&str> {
        self.refusal.as_deref()
    }

    /// Whether the session has ended, and why if the transport said.
    pub(crate) fn closed(&self) -> Option<Option<&str>> {
        self.closed.as_ref().map(|reason| reason.as_deref())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    /// What an unmetered session is seeded with on the gateway side. Only ever a value to carry: the
    /// probe draws no conclusion from its magnitude.
    const EPHEMERAL_ALLOWANCE: i64 = i64::MAX / 2;

    fn inbox() -> GatewaySessionInbox {
        GatewaySessionInbox::new(Duration::from_millis(50))
    }

    fn accepted(remaining_bandwidth: i64) -> SessionEvent {
        SessionEvent::Accepted {
            remaining_bandwidth,
        }
    }

    // the session's channel carries the gateway's acknowledgements as well as its deliveries, so a
    // payload behind a run of them must be returned rather than the first event being taken for one
    #[tokio::test]
    async fn an_acknowledgement_is_filed_without_being_taken_for_a_payload() {
        let mut inbox = inbox();
        let sender = inbox.events_sender();

        for event in [
            accepted(EPHEMERAL_ALLOWANCE),
            accepted(EPHEMERAL_ALLOWANCE - 386),
            SessionEvent::Delivered(ReceivedPayload::new(b"probe".to_vec())),
        ] {
            sender
                .unbounded_send(event)
                .expect("the inbox dropped its channel");
        }

        let payload = inbox
            .next_payload()
            .await
            .expect("the payload behind the acknowledgements was not returned");

        assert_eq!(payload.payload, b"probe".to_vec());
        assert_eq!(inbox.accepted(), 2);
    }

    // a session that was METERED is diagnosed by its refusal, not by any allowance figure: it holds
    // no bandwidth, so its first forward is refused and no acknowledgement ever arrives to read one
    // off. this is the whole of what tells a run that scored zero apart from a dead gateway
    #[test]
    fn a_metered_session_is_diagnosed_by_its_refusal() {
        let mut inbox = inbox();
        inbox.record(SessionEvent::Refused(
            "the gateway reported: out of bandwidth (required 386, available 0)".to_string(),
        ));

        assert_eq!(inbox.accepted(), 0);
        assert!(
            inbox
                .refusal()
                .is_some_and(|refusal| refusal.contains("out of bandwidth"))
        );
    }

    #[test]
    fn a_refusal_and_a_close_are_both_filed() {
        let mut inbox = inbox();
        inbox.record(SessionEvent::Refused("out of bandwidth".to_string()));
        inbox.record(SessionEvent::Closed(Some("reset".to_string())));

        assert_eq!(inbox.refusal(), Some("out of bandwidth"));
        assert_eq!(inbox.closed(), Some(Some("reset")));
    }

    // a close with no reason still has to register AS a close: `None` means the session is live
    #[test]
    fn a_reasonless_close_is_distinguishable_from_a_live_session() {
        let mut inbox = inbox();
        assert_eq!(inbox.closed(), None);

        inbox.record(SessionEvent::Closed(None));
        assert_eq!(inbox.closed(), Some(None));
    }
}
