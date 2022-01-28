// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::channel::mpsc;
use futures::lock::Mutex;
use futures::StreamExt;
use std::collections::HashMap;
use std::ops::DerefMut;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

// convenience aliases
pub type PacketsMap = HashMap<String, u64>;

type PacketDataReceiver = mpsc::UnboundedReceiver<PacketEvent>;
type PacketEventSender = mpsc::UnboundedSender<PacketEvent>;

pub enum PacketEvent {
    Sent(String),
    Received,
    Dropped(String),
}

// Packet statistics is accumulated in `SharedCurrentPacketEvents` and then periodically merged in
// to `SharedNodeStats`
#[derive(Debug, Clone)]
pub struct SharedCurrentPacketEvents {
    inner: Arc<PacketEventsInner>,
}

#[derive(Debug)]
struct PacketEventsInner {
    received: AtomicU64,
    sent: Mutex<PacketsMap>,
    dropped: Mutex<PacketsMap>,
}

impl SharedCurrentPacketEvents {
    pub fn new() -> Self {
        SharedCurrentPacketEvents {
            inner: Arc::new(PacketEventsInner {
                received: AtomicU64::new(0),
                sent: Mutex::new(HashMap::new()),
                dropped: Mutex::new(HashMap::new()),
            }),
        }
    }

    fn increment_received(&self) {
        self.inner.received.fetch_add(1, Ordering::SeqCst);
    }

    async fn increment_sent(&self, destination: String) {
        let mut unlocked = self.inner.sent.lock().await;
        let receiver_count = unlocked.entry(destination).or_insert(0);
        *receiver_count += 1;
    }

    async fn increment_dropped(&self, destination: String) {
        let mut unlocked = self.inner.dropped.lock().await;
        let dropped_count = unlocked.entry(destination).or_insert(0);
        *dropped_count += 1;
    }

    pub(super) async fn acquire_and_reset(&self) -> (u64, PacketsMap, PacketsMap) {
        let mut unlocked_sent = self.inner.sent.lock().await;
        let mut unlocked_dropped = self.inner.dropped.lock().await;
        let received = self.inner.received.swap(0, Ordering::SeqCst);

        let sent = std::mem::take(unlocked_sent.deref_mut());
        let dropped = std::mem::take(unlocked_dropped.deref_mut());

        (received, sent, dropped)
    }
}

/// Receive and handle packet events by updating the shared data pointer, `ShareCurrentPacketData`
pub struct PacketEventHandler {
    current_data: SharedCurrentPacketEvents,
    update_receiver: PacketDataReceiver,
}

impl PacketEventHandler {
    pub(super) fn new(
        current_data: SharedCurrentPacketEvents,
        update_receiver: PacketDataReceiver,
    ) -> Self {
        PacketEventHandler {
            current_data,
            update_receiver,
        }
    }

    pub async fn run(&mut self) {
        while let Some(packet_data) = self.update_receiver.next().await {
            match packet_data {
                PacketEvent::Received => self.current_data.increment_received(),
                PacketEvent::Sent(destination) => {
                    self.current_data.increment_sent(destination).await
                }
                PacketEvent::Dropped(destination) => {
                    self.current_data.increment_dropped(destination).await
                }
            }
        }
    }
}

/// Report packet events by sending down a channel to the `PacketDataHandler`
#[derive(Clone)]
pub struct PacketEventReporter(PacketEventSender);

impl PacketEventReporter {
    pub fn new(update_sender: PacketEventSender) -> Self {
        PacketEventReporter(update_sender)
    }

    pub fn report_sent(&self, destination: String) {
        // in unbounded_send() failed it means that the receiver channel was disconnected
        // and hence something weird must have happened without a way of recovering
        self.0
            .unbounded_send(PacketEvent::Sent(destination))
            .unwrap()
    }

    // TODO: in the future this could be slightly optimised to get rid of the channel
    // in favour of incrementing value directly
    pub fn report_received(&self) {
        // in unbounded_send() failed it means that the receiver channel was disconnected
        // and hence something weird must have happened without a way of recovering
        self.0.unbounded_send(PacketEvent::Received).unwrap()
    }

    pub fn report_dropped(&self, destination: String) {
        // in unbounded_send() failed it means that the receiver channel was disconnected
        // and hence something weird must have happened without a way of recovering
        self.0
            .unbounded_send(PacketEvent::Dropped(destination))
            .unwrap()
    }
}
