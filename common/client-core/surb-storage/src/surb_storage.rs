// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use dashmap::iter::Iter;
use dashmap::DashMap;
use log::trace;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use nym_sphinx::anonymous_replies::ReplySurbWithKeyRotation;
use nym_sphinx::params::SphinxKeyRotation;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub struct ReceivedReplySurbsMap {
    inner: Arc<ReceivedReplySurbsMapInner>,
}

#[derive(Debug)]
struct ReceivedReplySurbsMapInner {
    data: DashMap<AnonymousSenderTag, ReceivedReplySurbs>,

    // the minimum amount of surbs that have to be kept in storage for requests for more surbs
    min_surb_threshold: AtomicUsize,

    // the maximum amount of surbs that we want to keep in storage so that we don't over-request them
    max_surb_threshold: AtomicUsize,
}

impl ReceivedReplySurbsMap {
    pub fn new(min_surb_threshold: usize, max_surb_threshold: usize) -> ReceivedReplySurbsMap {
        ReceivedReplySurbsMap {
            inner: Arc::new(ReceivedReplySurbsMapInner {
                data: DashMap::new(),
                min_surb_threshold: AtomicUsize::new(min_surb_threshold),
                max_surb_threshold: AtomicUsize::new(max_surb_threshold),
            }),
        }
    }

    #[cfg(all(not(target_arch = "wasm32"), feature = "fs-surb-storage"))]
    pub fn from_raw(
        min_surb_threshold: usize,
        max_surb_threshold: usize,
        raw: Vec<(AnonymousSenderTag, ReceivedReplySurbs)>,
    ) -> ReceivedReplySurbsMap {
        ReceivedReplySurbsMap {
            inner: Arc::new(ReceivedReplySurbsMapInner {
                data: raw.into_iter().collect(),
                min_surb_threshold: AtomicUsize::new(min_surb_threshold),
                max_surb_threshold: AtomicUsize::new(max_surb_threshold),
            }),
        }
    }

    pub fn as_raw_iter(&self) -> Iter<'_, AnonymousSenderTag, ReceivedReplySurbs> {
        self.inner.data.iter()
    }

    pub fn remove(&self, target: &AnonymousSenderTag) {
        self.inner.data.remove(target);
    }

    pub fn retain(&self, f: impl FnMut(&AnonymousSenderTag, &mut ReceivedReplySurbs) -> bool) {
        self.inner.data.retain(f);
    }

    pub fn surbs_last_received_at(&self, target: &AnonymousSenderTag) -> Option<OffsetDateTime> {
        self.inner
            .data
            .get(target)
            .map(|e| e.surbs_last_received_at())
    }

    pub fn pending_reception(&self, target: &AnonymousSenderTag) -> u32 {
        self.inner
            .data
            .get(target)
            .map(|e| e.pending_reception())
            .unwrap_or_default()
    }

    pub fn increment_pending_reception(
        &self,
        target: &AnonymousSenderTag,
        amount: u32,
    ) -> Option<u32> {
        self.inner
            .data
            .get_mut(target)
            .map(|mut e| e.increment_pending_reception(amount))
    }

    pub fn decrement_pending_reception(
        &self,
        target: &AnonymousSenderTag,
        amount: u32,
    ) -> Option<u32> {
        self.inner
            .data
            .get_mut(target)
            .map(|mut e| e.decrement_pending_reception(amount))
    }

    pub fn reset_pending_reception(&self, target: &AnonymousSenderTag) {
        if let Some(mut e) = self.inner.data.get_mut(target) {
            e.reset_pending_reception()
        }
    }

    pub fn min_surb_threshold(&self) -> usize {
        self.inner.min_surb_threshold.load(Ordering::Relaxed)
    }

    pub fn max_surb_threshold(&self) -> usize {
        self.inner.max_surb_threshold.load(Ordering::Relaxed)
    }

    pub fn available_surbs(&self, target: &AnonymousSenderTag) -> usize {
        self.inner
            .data
            .get(target)
            .map(|entry| entry.items_left())
            .unwrap_or_default()
    }

    pub fn contains_surbs_for(&self, target: &AnonymousSenderTag) -> bool {
        self.inner.data.contains_key(target)
    }

    pub fn get_reply_surbs(
        &self,
        target: &AnonymousSenderTag,
        amount: usize,
    ) -> (Option<Vec<ReceivedReplySurb>>, usize) {
        if let Some(mut entry) = self.inner.data.get_mut(target) {
            let surbs_left = entry.items_left();
            if surbs_left < self.min_surb_threshold() + amount {
                (None, surbs_left)
            } else {
                entry.get_reply_surbs(amount)
            }
        } else {
            (None, 0)
        }
    }

    pub fn get_reply_surb_ignoring_threshold(
        &self,
        target: &AnonymousSenderTag,
    ) -> Option<(Option<ReceivedReplySurb>, usize)> {
        self.inner
            .data
            .get_mut(target)
            .map(|mut s| s.get_reply_surb())
    }

    pub fn get_reply_surb(
        &self,
        target: &AnonymousSenderTag,
    ) -> Option<(Option<ReceivedReplySurb>, usize)> {
        self.inner.data.get_mut(target).map(|mut entry| {
            let surbs_left = entry.items_left();
            if surbs_left < self.min_surb_threshold() {
                (None, surbs_left)
            } else {
                entry.get_reply_surb()
            }
        })
    }

    pub fn insert_surbs<I: IntoIterator<Item = ReplySurbWithKeyRotation>>(
        &self,
        target: &AnonymousSenderTag,
        surbs: I,
    ) {
        if let Some(mut existing_data) = self.inner.data.get_mut(target) {
            existing_data.insert_reply_surbs(surbs)
        } else {
            let new_entry = ReceivedReplySurbs::new(surbs.into_iter().collect());
            self.inner.data.insert(*target, new_entry);
        }
    }
}

#[derive(Debug)]
pub struct ReceivedReplySurb {
    pub(crate) surb: ReplySurbWithKeyRotation,
    pub(crate) received_at: OffsetDateTime,
}

impl From<ReceivedReplySurb> for ReplySurbWithKeyRotation {
    fn from(surb: ReceivedReplySurb) -> Self {
        surb.surb
    }
}

impl ReceivedReplySurb {
    pub fn received_at(&self) -> OffsetDateTime {
        self.received_at
    }

    pub fn key_rotation(&self) -> SphinxKeyRotation {
        self.surb.key_rotation()
    }
}

#[derive(Debug)]
pub struct ReceivedReplySurbs {
    data: VecDeque<ReceivedReplySurb>,

    pending_reception: u32,
    surbs_last_received_at: OffsetDateTime,
}

impl ReceivedReplySurbs {
    fn new(initial_surbs: VecDeque<ReplySurbWithKeyRotation>) -> Self {
        let mut this = ReceivedReplySurbs {
            data: Default::default(),
            pending_reception: 0,
            surbs_last_received_at: OffsetDateTime::now_utc(),
        };
        this.insert_reply_surbs(initial_surbs);
        this
    }

    #[cfg(all(not(target_arch = "wasm32"), feature = "fs-surb-storage"))]
    pub fn new_retrieved(
        surbs: Vec<ReplySurbWithKeyRotation>,
        surbs_last_received_at: OffsetDateTime,
    ) -> ReceivedReplySurbs {
        let mut this = ReceivedReplySurbs {
            data: Default::default(),
            pending_reception: 0,
            surbs_last_received_at,
        };
        this.insert_reply_surbs(surbs);
        this.surbs_last_received_at = surbs_last_received_at;
        this
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    #[cfg(all(not(target_arch = "wasm32"), feature = "fs-surb-storage"))]
    pub fn surbs_ref(&self) -> &VecDeque<ReceivedReplySurb> {
        &self.data
    }

    pub fn retain_surbs(&mut self, f: impl FnMut(&ReceivedReplySurb) -> bool) {
        self.data.retain(f)
    }

    pub fn surbs_last_received_at(&self) -> OffsetDateTime {
        self.surbs_last_received_at
    }

    pub fn pending_reception(&self) -> u32 {
        self.pending_reception
    }

    pub fn increment_pending_reception(&mut self, amount: u32) -> u32 {
        self.pending_reception += amount;
        self.pending_reception
    }

    pub fn decrement_pending_reception(&mut self, amount: u32) -> u32 {
        self.pending_reception = self.pending_reception.saturating_sub(amount);
        self.pending_reception
    }

    pub fn reset_pending_reception(&mut self) {
        self.pending_reception = 0;
    }

    pub fn get_reply_surbs(&mut self, amount: usize) -> (Option<Vec<ReceivedReplySurb>>, usize) {
        if self.items_left() < amount {
            (None, self.items_left())
        } else {
            let surbs = self.data.drain(..amount).collect();
            (Some(surbs), self.items_left())
        }
    }

    pub fn get_reply_surb(&mut self) -> (Option<ReceivedReplySurb>, usize) {
        (self.pop_surb(), self.items_left())
    }

    fn pop_surb(&mut self) -> Option<ReceivedReplySurb> {
        self.data.pop_front()
    }

    fn items_left(&self) -> usize {
        self.data.len()
    }

    // realistically we're always going to be getting multiple surbs at once
    pub fn insert_reply_surbs<I: IntoIterator<Item = ReplySurbWithKeyRotation>>(
        &mut self,
        surbs: I,
    ) {
        let received_at = OffsetDateTime::now_utc();
        let mut v = surbs
            .into_iter()
            .map(|surb| ReceivedReplySurb { surb, received_at })
            .collect::<VecDeque<_>>();

        trace!("storing {} surbs in the storage", v.len());
        self.data.append(&mut v);
        self.surbs_last_received_at = received_at;
        trace!("we now have {} surbs!", self.data.len());
    }
}
