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
#[derive(Clone, Debug)]
pub struct LaneQueueLengths(std::sync::Arc<std::sync::Mutex<LaneQueueLengthsInner>>);

impl LaneQueueLengths {
    pub fn new() -> Self {
        LaneQueueLengths(std::sync::Arc::new(std::sync::Mutex::new(
            LaneQueueLengthsInner {
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

    pub fn get(&self, lane: &TransmissionLane) -> Option<usize> {
        match self.0.lock() {
            Ok(inner) => inner.get(lane),
            Err(err) => {
                log::warn!("Failed to get lane queue length: {err}");
                None
            }
        }
    }
}

impl Default for LaneQueueLengths {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Deref for LaneQueueLengths {
    type Target = std::sync::Arc<std::sync::Mutex<LaneQueueLengthsInner>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct LaneQueueLengthsInner {
    map: HashMap<TransmissionLane, usize>,
}

impl LaneQueueLengthsInner {
    pub fn get(&self, lane: &TransmissionLane) -> Option<usize> {
        self.map.get(lane).copied()
    }

    pub fn values(&self) -> impl Iterator<Item = &usize> {
        self.map.values()
    }

    pub fn modify<F>(&mut self, lane: &TransmissionLane, f: F)
    where
        F: FnOnce(&mut usize),
    {
        self.map.entry(*lane).and_modify(f);
    }
}
