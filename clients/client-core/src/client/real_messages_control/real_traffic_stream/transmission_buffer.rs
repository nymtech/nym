// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use client_connections::TransmissionLane;
use rand::seq::SliceRandom;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    time::Duration,
};

#[cfg(not(target_arch = "wasm32"))]
use tokio::time;

#[cfg(target_arch = "wasm32")]
use wasm_timer;

use super::{get_time_now, RealMessage};

// The number of lanes included in the oldest set. Used when we need to prioritize traffic.
const OLDEST_LANE_SET_SIZE: usize = 5;
// As a way of prune connections we also check for timeouts.
const MSG_CONSIDERED_STALE_AFTER_SECS: u64 = 10 * 60;

#[derive(Default)]
pub(crate) struct TransmissionBuffer {
    buffer: HashMap<TransmissionLane, LaneBufferEntry>,
}

impl TransmissionBuffer {
    #[allow(unused)]
    pub(crate) fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub(crate) fn remove(&mut self, lane: &TransmissionLane) -> Option<LaneBufferEntry> {
        self.buffer.remove(lane)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn num_lanes(&self) -> usize {
        self.buffer.keys().count()
    }

    pub(crate) fn lane_length(&self, lane: &TransmissionLane) -> Option<usize> {
        self.buffer.get(lane).map(LaneBufferEntry::len)
    }

    #[allow(unused)]
    pub(crate) fn connections(&self) -> HashSet<u64> {
        self.buffer
            .keys()
            .filter_map(|lane| match lane {
                TransmissionLane::ConnectionId(id) => Some(id),
                _ => None,
            })
            .copied()
            .collect()
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn total_size(&self) -> usize {
        self.buffer.values().map(LaneBufferEntry::len).sum()
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn total_size_in_bytes(&self) -> usize {
        self.buffer
            .values()
            .map(|lane_buffer_entry| {
                lane_buffer_entry
                    .real_messages
                    .iter()
                    .map(|real_message| real_message.mix_packet.sphinx_packet().len())
                    .sum::<usize>()
            })
            .sum()
    }

    fn get_oldest_set(&self) -> Vec<TransmissionLane> {
        let mut buffer: Vec<_> = self
            .buffer
            .iter()
            .map(|(k, v)| (k, v.messages_transmitted))
            .collect();
        buffer.sort_by_key(|v| v.1);
        buffer
            .iter()
            .rev()
            .map(|(k, _)| *k)
            .take(OLDEST_LANE_SET_SIZE)
            .copied()
            .collect()
    }

    pub(crate) fn store(&mut self, lane: &TransmissionLane, real_messages: Vec<RealMessage>) {
        if let Some(lane_buffer_entry) = self.buffer.get_mut(lane) {
            lane_buffer_entry.append(real_messages);
        } else {
            self.buffer
                .insert(*lane, LaneBufferEntry::new(real_messages));
        }
    }

    fn pick_random_lane(&self) -> Option<&TransmissionLane> {
        let lanes: Vec<&TransmissionLane> = self.buffer.keys().collect();
        lanes.choose(&mut rand::thread_rng()).copied()
    }

    fn pick_random_small_lane(&self) -> Option<&TransmissionLane> {
        let lanes: Vec<&TransmissionLane> = self
            .buffer
            .iter()
            .filter(|(_, v)| v.is_small())
            .map(|(k, _)| k)
            .collect();
        lanes.choose(&mut rand::thread_rng()).copied()
    }

    fn pick_random_old_lane(&self) -> Option<TransmissionLane> {
        let lanes = self.get_oldest_set();
        lanes.choose(&mut rand::thread_rng()).copied()
    }

    fn pop_front_from_lane(&mut self, lane: &TransmissionLane) -> Option<RealMessage> {
        let real_msgs_queued = self.buffer.get_mut(lane)?;
        let real_next = real_msgs_queued.pop_front()?;
        real_msgs_queued.messages_transmitted += 1;
        if real_msgs_queued.is_empty() {
            self.buffer.remove(lane);
        }
        Some(real_next)
    }

    pub(crate) fn pop_next_message_at_random(&mut self) -> Option<(TransmissionLane, RealMessage)> {
        if self.buffer.is_empty() {
            return None;
        }

        // Very basic heuristic where we prioritize according to small lanes first, the older lanes
        // to try to finish lanes when possible, then the rest.
        let lane = if let Some(small_lane) = self.pick_random_small_lane() {
            *small_lane
        } else if let Some(old_lane) = self.pick_random_old_lane() {
            old_lane
        } else {
            *self.pick_random_lane()?
        };

        let msg = self.pop_front_from_lane(&lane)?;
        log::trace!("picking to send from lane: {:?}", lane);
        Some((lane, msg))
    }

    pub(crate) fn prune_stale_connections(&mut self) {
        let stale_entries: Vec<_> = self
            .buffer
            .iter()
            .filter_map(|(lane, entry)| if entry.is_stale() { Some(lane) } else { None })
            .copied()
            .collect();

        for lane in stale_entries {
            self.remove(&lane);
        }
    }
}

pub(crate) struct LaneBufferEntry {
    pub real_messages: VecDeque<RealMessage>,
    pub messages_transmitted: usize,
    #[cfg(not(target_arch = "wasm32"))]
    pub time_for_last_activity: time::Instant,
    #[cfg(target_arch = "wasm32")]
    pub time_for_last_activity: wasm_timer::Instant,
}

impl LaneBufferEntry {
    fn new(real_messages: Vec<RealMessage>) -> Self {
        LaneBufferEntry {
            real_messages: real_messages.into(),
            messages_transmitted: 0,
            time_for_last_activity: get_time_now(),
        }
    }

    fn append(&mut self, real_messages: Vec<RealMessage>) {
        self.real_messages.append(&mut real_messages.into());
        self.time_for_last_activity = get_time_now();
    }

    fn pop_front(&mut self) -> Option<RealMessage> {
        self.real_messages.pop_front()
    }

    fn is_small(&self) -> bool {
        self.real_messages.len() < 100
    }

    fn is_stale(&self) -> bool {
        get_time_now() - self.time_for_last_activity
            > Duration::from_secs(MSG_CONSIDERED_STALE_AFTER_SECS)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn len(&self) -> usize {
        self.real_messages.len()
    }

    fn is_empty(&self) -> bool {
        self.real_messages.is_empty()
    }
}
