// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::helpers::{get_time_now, Instant};
use crate::client::real_messages_control::real_traffic_stream::RealMessage;
use nym_sphinx::chunking::fragment::Fragment;
use nym_task::connections::TransmissionLane;
use rand::{seq::SliceRandom, Rng};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    time::Duration,
};

// The number of lanes included in the oldest set. Used when we need to prioritize traffic.
const OLDEST_LANE_SET_SIZE: usize = 4;
// As a way of prune connections we also check for timeouts.
const MSG_CONSIDERED_STALE_AFTER_SECS: u64 = 10 * 60;

// this trait is apparently not used in wasm
#[allow(dead_code)]
pub(crate) trait SizedData {
    fn data_size(&self) -> usize;
}

impl SizedData for RealMessage {
    fn data_size(&self) -> usize {
        self.packet_size()
    }
}

impl SizedData for Fragment {
    fn data_size(&self) -> usize {
        // note that raw `Fragment` is smaller than packet payload
        // as it doesn't include surb-ack or the [shared] key materials
        self.payload_size()
    }
}

#[derive(Default)]
pub(crate) struct TransmissionBuffer<T> {
    buffer: HashMap<TransmissionLane, LaneBufferEntry<T>>,
}

impl<T> TransmissionBuffer<T> {
    pub(crate) fn new() -> Self {
        TransmissionBuffer {
            buffer: HashMap::new(),
        }
    }

    #[allow(unused)]
    pub(crate) fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub(crate) fn remove(&mut self, lane: &TransmissionLane) -> Option<LaneBufferEntry<T>> {
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

    pub(crate) fn total_size(&self) -> usize {
        self.buffer.values().map(LaneBufferEntry::len).sum()
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn total_size_in_bytes(&self) -> usize
    where
        T: SizedData,
    {
        self.buffer
            .values()
            .map(|lane_buffer_entry| {
                lane_buffer_entry
                    .items
                    .iter()
                    .map(|item| item.data_size())
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

    pub(crate) fn store<I: IntoIterator<Item = T>>(&mut self, lane: &TransmissionLane, items: I) {
        if let Some(lane_buffer_entry) = self.buffer.get_mut(lane) {
            lane_buffer_entry.extend(items);
        } else {
            self.buffer
                .insert(*lane, LaneBufferEntry::new(items.into_iter().collect()));
        }
    }

    pub(crate) fn store_multiple(&mut self, items: Vec<(TransmissionLane, T)>) {
        for (lane, item) in items {
            self.buffer
                .entry(lane)
                .or_insert_with(LaneBufferEntry::new_empty)
                .push_item(item)
        }
    }

    fn pick_random_lane<R: Rng + ?Sized>(&self, rng: &mut R) -> Option<&TransmissionLane> {
        let lanes: Vec<&TransmissionLane> = self.buffer.keys().collect();
        lanes.choose(rng).copied()
    }

    fn pick_random_small_lane<R: Rng + ?Sized>(&self, rng: &mut R) -> Option<&TransmissionLane> {
        let lanes: Vec<&TransmissionLane> = self
            .buffer
            .iter()
            .filter(|(_, v)| v.is_small())
            .map(|(k, _)| k)
            .collect();
        lanes.choose(rng).copied()
    }

    // 2/3 chance to pick from the old lanes
    fn pick_random_old_lane<R: Rng + ?Sized>(&self, rng: &mut R) -> Option<TransmissionLane> {
        let rand = &mut rand::thread_rng();
        if rand.gen_ratio(2, 3) {
            let lanes = self.get_oldest_set();
            lanes.choose(rand).copied()
        } else {
            self.pick_random_lane(rng).copied()
        }
    }

    fn pop_front_from_lane(&mut self, lane: &TransmissionLane) -> Option<T> {
        let real_msgs_queued = self.buffer.get_mut(lane)?;
        let real_next = real_msgs_queued.pop_front()?;
        real_msgs_queued.messages_transmitted += 1;
        if real_msgs_queued.is_empty() {
            self.buffer.remove(lane);
        }
        Some(real_next)
    }

    pub(crate) fn pop_at_most_n_next_messages_at_random(
        &mut self,
        n: usize,
    ) -> Option<Vec<(TransmissionLane, T)>> {
        if self.buffer.is_empty() {
            return None;
        }

        let rng = &mut rand::thread_rng();
        let mut items = Vec::with_capacity(n);

        while items.len() < n {
            let Some(next) = self.pop_next_message_at_random(rng) else {
                break;
            };
            items.push(next)
        }

        Some(items)
    }

    pub(crate) fn pop_next_message_at_random<R: Rng + ?Sized>(
        &mut self,
        // turns out the caller always have access to some rng, so no point in instantiating new one
        rng: &mut R,
    ) -> Option<(TransmissionLane, T)> {
        if self.buffer.is_empty() {
            return None;
        }

        // Very basic heuristic where we prioritize according to small lanes first, the older lanes
        // to try to finish lanes when possible, then the rest.
        let lane = if let Some(small_lane) = self.pick_random_small_lane(rng) {
            *small_lane
        } else if let Some(old_lane) = self.pick_random_old_lane(rng) {
            old_lane
        } else {
            *self.pick_random_lane(rng)?
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

pub(crate) struct LaneBufferEntry<T> {
    pub items: VecDeque<T>,
    pub messages_transmitted: usize,
    pub time_for_last_activity: Instant,
}

impl<T> LaneBufferEntry<T> {
    fn new_empty() -> Self {
        LaneBufferEntry {
            items: VecDeque::new(),
            messages_transmitted: 0,
            time_for_last_activity: get_time_now(),
        }
    }

    fn new(items: VecDeque<T>) -> Self {
        LaneBufferEntry {
            items,
            messages_transmitted: 0,
            time_for_last_activity: get_time_now(),
        }
    }

    fn push_item(&mut self, item: T) {
        self.items.push_back(item);
        // I'm not updating time here on purpose. This method is called just after `new_empty`,
        // where the time is already set. Furthermore, this method is called there multiple times at once
    }

    fn extend<I: IntoIterator<Item = T>>(&mut self, items: I) {
        self.items.extend(items);
        self.time_for_last_activity = get_time_now();
    }

    fn pop_front(&mut self) -> Option<T> {
        self.items.pop_front()
    }

    fn is_small(&self) -> bool {
        self.items.len() < 100
    }

    fn is_stale(&self) -> bool {
        get_time_now() - self.time_for_last_activity
            > Duration::from_secs(MSG_CONSIDERED_STALE_AFTER_SECS)
    }

    fn len(&self) -> usize {
        self.items.len()
    }

    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}
