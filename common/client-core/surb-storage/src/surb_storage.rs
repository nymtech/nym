// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use dashmap::iter::{Iter, IterMut};
use dashmap::DashMap;
use nym_sphinx::anonymous_replies::requests::AnonymousSenderTag;
use nym_sphinx::anonymous_replies::ReplySurbWithKeyRotation;
use nym_sphinx::params::SphinxKeyRotation;
use std::cmp::min;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use time::OffsetDateTime;
use tracing::{error, trace};

#[derive(Debug)]
pub struct RetrievedReplySurb {
    pub(crate) reply_surb: ReceivedReplySurb,
    pub(crate) stale_pile: bool,
}

impl RetrievedReplySurb {
    pub(crate) fn new_fresh(reply_surb: ReceivedReplySurb) -> Self {
        RetrievedReplySurb {
            reply_surb,
            stale_pile: false,
        }
    }

    pub(crate) fn new_stale(reply_surb: ReceivedReplySurb) -> Self {
        RetrievedReplySurb {
            reply_surb,
            stale_pile: true,
        }
    }
}

impl From<RetrievedReplySurb> for ReplySurbWithKeyRotation {
    fn from(retrieved: RetrievedReplySurb) -> Self {
        retrieved.reply_surb.into()
    }
}

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

    pub fn as_raw_iter_mut(&self) -> IterMut<'_, AnonymousSenderTag, ReceivedReplySurbs> {
        self.inner.data.iter_mut()
    }

    // pub fn remove(&self, target: &AnonymousSenderTag) {
    //     self.inner.data.remove(target);
    // }

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

    pub fn available_fresh_surbs(&self, target: &AnonymousSenderTag) -> usize {
        self.inner
            .data
            .get(target)
            .map(|entry| entry.fresh_left())
            .unwrap_or_default()
    }

    pub fn contains_surbs_for(&self, target: &AnonymousSenderTag) -> bool {
        self.inner.data.contains_key(target)
    }

    /// Attempt to retrieve the specified number of reply SURBs for the target sender
    /// and return the number of SURBs remaining in the storage after the call.
    pub fn get_reply_surbs(
        &self,
        target: &AnonymousSenderTag,
        amount: usize,
    ) -> (Option<Vec<RetrievedReplySurb>>, usize) {
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
    ) -> (Option<RetrievedReplySurb>, usize) {
        let Some(mut entry) = self.inner.data.get_mut(target) else {
            return (None, 0);
        };

        entry.get_reply_surb()
    }

    pub fn get_reply_surb(
        &self,
        target: &AnonymousSenderTag,
    ) -> (Option<RetrievedReplySurb>, usize) {
        let Some(mut entry) = self.inner.data.get_mut(target) else {
            return (None, 0);
        };

        let surbs_left = entry.items_left();
        if surbs_left < self.min_surb_threshold() {
            (None, surbs_left)
        } else {
            entry.get_reply_surb()
        }
    }

    pub fn re_insert_reply_surbs(
        &self,
        target: &AnonymousSenderTag,
        surbs: Vec<RetrievedReplySurb>,
    ) {
        error!("re-inserting {} unused surbs", surbs.len());
        let mut entry = self.inner.data.entry(*target).or_insert_with(|| {
            // this branch should realistically NEVER happen, but software be software, so let's not crash
            error!("attempting to return surbs to no longer existing entry {target}");
            ReceivedReplySurbs::new(VecDeque::new())
        });

        let entry = entry.value_mut();
        for returned_surb in surbs.into_iter().rev() {
            if returned_surb.stale_pile {
                entry.possibly_stale.push_front(returned_surb.reply_surb)
            } else {
                entry.data.push_front(returned_surb.reply_surb)
            }
        }
    }

    pub fn insert_fresh_surbs<I: IntoIterator<Item = ReplySurbWithKeyRotation>>(
        &self,
        target: &AnonymousSenderTag,
        surbs: I,
    ) {
        if let Some(mut existing_data) = self.inner.data.get_mut(target) {
            existing_data.insert_fresh_reply_surbs(surbs);

            if existing_data.possibly_stale.is_empty() {
                return;
            }

            // if we're above the minimum threshold, remove stale surbs
            let threshold = self.min_surb_threshold();
            let diff = existing_data.data.len().saturating_sub(threshold);

            trace!("will attempt to remove up to {diff} stale surbs");
            if diff > 0 {
                existing_data.remove_stale_surbs(diff);
            }
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
    possibly_stale: VecDeque<ReceivedReplySurb>,

    pending_reception: u32,
    surbs_last_received_at: OffsetDateTime,
}

impl ReceivedReplySurbs {
    fn new(initial_surbs: VecDeque<ReplySurbWithKeyRotation>) -> Self {
        let mut this = ReceivedReplySurbs {
            data: Default::default(),
            possibly_stale: Default::default(),
            pending_reception: 0,
            surbs_last_received_at: OffsetDateTime::now_utc(),
        };
        this.insert_fresh_reply_surbs(initial_surbs);
        this
    }

    #[cfg(all(not(target_arch = "wasm32"), feature = "fs-surb-storage"))]
    pub fn new_retrieved(
        surbs: Vec<ReplySurbWithKeyRotation>,
        surbs_last_received_at: OffsetDateTime,
    ) -> ReceivedReplySurbs {
        let mut this = ReceivedReplySurbs {
            data: Default::default(),
            possibly_stale: Default::default(),
            pending_reception: 0,
            surbs_last_received_at,
        };
        this.insert_fresh_reply_surbs(surbs);
        this.surbs_last_received_at = surbs_last_received_at;
        this
    }

    pub fn downgrade_freshness(&mut self) -> usize {
        debug_assert!(self.possibly_stale.is_empty());
        std::mem::swap(&mut self.data, &mut self.possibly_stale);
        self.possibly_stale.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty() && self.possibly_stale.is_empty()
    }

    #[cfg(all(not(target_arch = "wasm32"), feature = "fs-surb-storage"))]
    pub fn surbs_ref(&self) -> &VecDeque<ReceivedReplySurb> {
        &self.data
    }

    pub fn retain_fresh_surbs(&mut self, f: impl FnMut(&ReceivedReplySurb) -> bool) {
        self.data.retain(f);
    }

    pub fn retain_possibly_stale_surbs(&mut self, f: impl FnMut(&ReceivedReplySurb) -> bool) {
        self.possibly_stale.retain(f);
    }

    pub fn fresh_left(&self) -> usize {
        self.data.len()
    }

    pub fn possibly_stale_left(&self) -> usize {
        self.possibly_stale.len()
    }

    pub fn drop_possibly_stale_surbs(&mut self) {
        self.possibly_stale = VecDeque::new();
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

    /// Attempt to retrieve the specified number of reply SURBs (if at least that many are present)
    /// and return the number of SURBs remaining in the storage after the call.
    pub fn get_reply_surbs(&mut self, amount: usize) -> (Option<Vec<RetrievedReplySurb>>, usize) {
        if self.items_left() < amount {
            (None, self.items_left())
        } else {
            let available_fresh = self.fresh_left();

            // prefer the 'fresh' data if available. otherwise fallback to the possibly stale entries
            let mut reply_surbs = Vec::with_capacity(amount);

            let fresh_to_retrieve = min(available_fresh, amount);

            for surb in self.data.drain(..fresh_to_retrieve) {
                reply_surbs.push(RetrievedReplySurb::new_fresh(surb))
            }

            if available_fresh < amount {
                let stale_to_retrieve = amount - fresh_to_retrieve;
                for surb in self.possibly_stale.drain(..stale_to_retrieve) {
                    reply_surbs.push(RetrievedReplySurb::new_stale(surb))
                }
            }

            (Some(reply_surbs), self.items_left())
        }
    }

    pub fn get_reply_surb(&mut self) -> (Option<RetrievedReplySurb>, usize) {
        (self.pop_surb(), self.items_left())
    }

    fn pop_surb(&mut self) -> Option<RetrievedReplySurb> {
        // prefer the 'fresh' data if available. otherwise fallback to the possibly stale entries
        if let Some(fresh) = self.data.pop_front() {
            return Some(RetrievedReplySurb::new_fresh(fresh));
        }
        if let Some(stale) = self.possibly_stale.pop_front() {
            return Some(RetrievedReplySurb::new_stale(stale));
        }
        None
    }

    fn items_left(&self) -> usize {
        self.data.len() + self.possibly_stale.len()
    }

    pub fn remove_stale_surbs(&mut self, amount: usize) {
        // remove up to amount number of possibly stale surbs
        let amount = min(amount, self.possibly_stale.len());

        self.possibly_stale.drain(..amount);
    }

    // realistically we're always going to be getting multiple surbs at once
    pub(crate) fn insert_fresh_reply_surbs<I: IntoIterator<Item = ReplySurbWithKeyRotation>>(
        &mut self,
        surbs: I,
    ) {
        let received_at = OffsetDateTime::now_utc();
        let mut v = surbs
            .into_iter()
            .map(|surb| ReceivedReplySurb { surb, received_at })
            .collect::<VecDeque<_>>();

        if v.is_empty() {
            return;
        }

        trace!("storing {} surbs in the storage", v.len());
        self.data.append(&mut v);
        self.surbs_last_received_at = received_at;
        trace!("we now have {} surbs!", self.data.len());
    }
}
