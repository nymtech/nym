// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use futures::channel::mpsc;

pub type ConnectionId = u64;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum TransmissionLane {
    General,
    Reply,
    Retransmission,
    Control,
    ConnectionId(ConnectionId),
}

/// Announce connections that are closed, for whoever is interested.
/// One usecase is that the network-requester and socks5-client wants to know about this, so that
/// they can forward this to the `OutQueueControl` (via `ClientRequest` for the network-requester)
pub type ClosedConnectionSender = mpsc::UnboundedSender<ConnectionId>;
pub type ClosedConnectionReceiver = mpsc::UnboundedReceiver<ConnectionId>;

// The `OutQueueControl` publishes the backlog per lane, primarily so that upstream can slow down
// if needed.
#[derive(Clone)]
pub struct LaneQueueLength(std::sync::Arc<std::sync::Mutex<LaneQueueLengthInner>>);

impl LaneQueueLength {
    pub fn new() -> Self {
        LaneQueueLength(std::sync::Arc::new(std::sync::Mutex::new(
            LaneQueueLengthInner {
                map: HashMap::new(),
            },
        )))
    }

    pub fn set(&mut self, lane: &TransmissionLane, lane_length: Option<usize>) {
        match self.0.lock() {
            Ok(mut inner) => {
                if let Some(length) = lane_length {
                    inner
                        .map
                        .entry(*lane)
                        .and_modify(|e| *e = length)
                        .or_insert(length);
                } else {
                    inner.map.remove(lane);
                }
            }
            Err(err) => log::warn!("Failed to set lane queue length: {err}"),
        }
    }
}

impl Default for LaneQueueLength {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Deref for LaneQueueLength {
    type Target = std::sync::Arc<std::sync::Mutex<LaneQueueLengthInner>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct LaneQueueLengthInner {
    map: HashMap<TransmissionLane, usize>,
}
