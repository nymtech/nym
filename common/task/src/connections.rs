// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::channel::mpsc;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

const LANE_CONSIDERED_CLEAR: usize = 10;

pub type ConnectionId = u64;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum TransmissionLane {
    General,
    // we need to treat surb-related requests and responses at higher priority
    // so that the rest of underlying communication could actually continue
    ReplySurbRequest,
    AdditionalReplySurbs,
    Retransmission,
    ConnectionId(ConnectionId),
}

/// Used by the connection controller to report current state for client connections.
pub type ConnectionCommandSender = mpsc::UnboundedSender<ConnectionCommand>;
pub type ConnectionCommandReceiver = mpsc::UnboundedReceiver<ConnectionCommand>;

pub enum ConnectionCommand {
    // Announce that at a connection was closed. E.g the `OutQueueControl` uses this to discard
    // transmission lanes.
    Close(ConnectionId),
}

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

    pub fn total(&self) -> usize {
        match self.0.lock() {
            Ok(inner) => inner.values().sum(),
            Err(err) => {
                log::warn!("Failed to get total queue length: {err}");
                0
            }
        }
    }

    pub async fn wait_until_clear(&self, lane: &TransmissionLane, timeout: Option<Duration>) {
        let total_time_waited = Instant::now();
        loop {
            let lane_length = self.get(lane).unwrap_or_default();
            if lane_length < LANE_CONSIDERED_CLEAR {
                break;
            }
            if timeout.is_some_and(|timeout| total_time_waited.elapsed() > timeout) {
                log::warn!("Timeout reached while waiting for queue to clear");
                break;
            }
            log::trace!("Waiting for queue to clear ({} items left)", lane_length);
            tokio::time::sleep(Duration::from_millis(100)).await;
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
    pub map: HashMap<TransmissionLane, usize>,
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

    pub fn total(&self) -> usize {
        self.map.values().sum()
    }
}
