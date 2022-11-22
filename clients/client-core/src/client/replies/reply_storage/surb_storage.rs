// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use dashmap::DashMap;
use log::error;
use nymsphinx::anonymous_replies::requests::{AnonymousSenderTag, ReplyMessage};
use nymsphinx::anonymous_replies::ReplySurb;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::Instant;

#[derive(Debug, Clone)]
pub struct ReceivedReplySurbsMap {
    // data: Arc<RwLock<HashMap<AnonymousSenderTag, ReceivedReplySurbs>>>,
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

pub enum SurbError {}

impl ReceivedReplySurbsMap {
    pub(crate) fn new(
        min_surb_threshold: usize,
        max_surb_threshold: usize,
    ) -> ReceivedReplySurbsMap {
        ReceivedReplySurbsMap {
            inner: Arc::new(ReceivedReplySurbsMapInner {
                data: DashMap::new(),
                min_surb_threshold: AtomicUsize::new(min_surb_threshold),
                max_surb_threshold: AtomicUsize::new(max_surb_threshold),
            }),
        }
    }

    // pub(crate) async fn create_new_sender_store(
    //     &mut self,
    //     target: AnonymousSenderTag,
    //     initial_surbs: Vec<ReplySurb>,
    // ) {
    //     let mut guard = self.data.write().await;
    //     let entry = ReceivedReplySurbs::new(initial_surbs);
    //     if let Some(existing_data) = guard.insert(target, entry) {
    //         existing_data.invalidate();
    //         let lost = existing_data.inner.data.lock().await.len();
    //         error!(
    //             "we have overwritten surbs stored for {:?}. We lost {:?} entries.",
    //             target, lost
    //         )
    //     }
    // }
    //
    // pub(crate) async fn get_handle(
    //     &self,
    //     target: &AnonymousSenderTag,
    // ) -> Option<ReceivedReplySurbs> {
    //     self.data.read().await.get(target).cloned()
    // }

    pub(crate) fn reset_surbs_last_received_at(&self, target: &AnonymousSenderTag) {
        if let Some(mut entry) = self.inner.data.get_mut(target) {
            entry.surbs_last_received_at = Instant::now();
        }
    }

    pub(crate) fn surbs_last_received_at(&self, target: &AnonymousSenderTag) -> Option<Instant> {
        self.inner
            .data
            .get(target)
            .map(|e| e.surbs_last_received_at())
    }

    pub(crate) fn pending_reception(&self, target: &AnonymousSenderTag) -> Option<u32> {
        self.inner.data.get(target).map(|e| e.pending_reception())
    }

    pub(crate) fn increment_pending_reception(
        &self,
        target: &AnonymousSenderTag,
        amount: u32,
    ) -> Option<u32> {
        self.inner
            .data
            .get_mut(target)
            .map(|mut e| e.increment_pending_reception(amount))
    }

    pub(crate) fn decrement_pending_reception(
        &self,
        target: &AnonymousSenderTag,
        amount: u32,
    ) -> Option<u32> {
        self.inner
            .data
            .get_mut(target)
            .map(|mut e| e.decrement_pending_reception(amount))
    }

    pub(crate) fn reset_pending_reception(&self, target: &AnonymousSenderTag) -> Option<()> {
        self.inner
            .data
            .get_mut(target)
            .map(|mut e| e.reset_pending_reception())
    }

    pub(crate) fn below_threshold(&self, amount: usize) -> bool {
        amount < self.min_surb_threshold()
    }

    pub(crate) fn min_surb_threshold(&self) -> usize {
        self.inner.min_surb_threshold.load(Ordering::Relaxed)
    }

    pub(crate) fn max_surb_threshold(&self) -> usize {
        self.inner.max_surb_threshold.load(Ordering::Relaxed)
    }

    pub(crate) fn available_surbs(&self, target: &AnonymousSenderTag) -> usize {
        self.inner
            .data
            .get(target)
            .map(|entry| entry.items_left())
            .unwrap_or_default()
    }

    pub(crate) fn contains_surbs_for(&self, target: &AnonymousSenderTag) -> bool {
        self.inner.data.contains_key(target)
    }

    pub(crate) fn get_reply_surbs(
        &self,
        target: &AnonymousSenderTag,
        amount: usize,
    ) -> (Option<Vec<ReplySurb>>, usize) {
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

    pub(crate) fn get_reply_surb_ignoring_threshold(
        &self,
        target: &AnonymousSenderTag,
    ) -> Option<(Option<ReplySurb>, usize)> {
        self.inner
            .data
            .get_mut(target)
            .map(|mut s| s.get_reply_surb())
    }

    pub(crate) fn get_reply_surb(
        &self,
        target: &AnonymousSenderTag,
    ) -> Option<(Option<ReplySurb>, usize)> {
        self.inner.data.get_mut(target).map(|mut entry| {
            let surbs_left = entry.items_left();
            if surbs_left < self.min_surb_threshold() {
                (None, surbs_left)
            } else {
                entry.get_reply_surb()
            }
        })
    }

    pub(crate) fn insert_surbs<I: IntoIterator<Item = ReplySurb>>(
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

    // pub(crate) fn insert_maybe_surbs<I: IntoIterator<Item = ReplySurb>>(
    //     &self,
    //     target: &AnonymousSenderTag,
    //     surbs: Option<I>,
    // ) {
    //     if let Some(reply_surbs) = surbs {
    //         self.insert_surbs(target, reply_surbs)
    //     }
    // }

    pub(crate) fn insert_surb(&self, target: &AnonymousSenderTag, reply_surb: ReplySurb) {
        if let Some(mut existing_data) = self.inner.data.get_mut(target) {
            existing_data.insert_reply_surb(reply_surb)
        } else {
            // this should never really get hit, but let's guard ourselves against it regardless...
            let mut surbs = VecDeque::new();
            surbs.push_back(reply_surb);
            let new_entry = ReceivedReplySurbs::new(surbs);
            self.inner.data.insert(*target, new_entry);
        }
    }
}

#[derive(Debug)]
struct ReceivedReplySurbs {
    // in the future we'd probably want to put extra data here to indicate when the SURBs got received
    // so we could invalidate entries from the previous key rotations
    data: VecDeque<ReplySurb>,

    pending_reception: u32,
    surbs_last_received_at: Instant,
}

impl ReceivedReplySurbs {
    fn new(initial_surbs: VecDeque<ReplySurb>) -> Self {
        ReceivedReplySurbs {
            data: initial_surbs,
            pending_reception: 0,
            surbs_last_received_at: Instant::now(),
        }
    }

    pub(crate) fn surbs_last_received_at(&self) -> Instant {
        self.surbs_last_received_at
    }

    pub(crate) fn pending_reception(&self) -> u32 {
        self.pending_reception
    }

    pub(crate) fn increment_pending_reception(&mut self, amount: u32) -> u32 {
        self.pending_reception += amount;
        self.pending_reception
    }

    pub(crate) fn decrement_pending_reception(&mut self, amount: u32) -> u32 {
        println!("{} - {}", self.pending_reception, amount);
        self.pending_reception -= amount;
        self.pending_reception
    }

    pub(crate) fn reset_pending_reception(&mut self) {
        self.pending_reception = 0
    }

    pub(crate) fn get_reply_surbs(&mut self, amount: usize) -> (Option<Vec<ReplySurb>>, usize) {
        if self.items_left() < amount {
            (None, self.items_left())
        } else {
            let surbs = self.data.drain(..amount).collect();
            (Some(surbs), self.items_left())
        }
    }

    pub(crate) fn get_reply_surb(&mut self) -> (Option<ReplySurb>, usize) {
        (self.pop_surb(), self.items_left())
    }

    fn pop_surb(&mut self) -> Option<ReplySurb> {
        self.data.pop_front()
    }

    fn items_left(&self) -> usize {
        self.data.len()
    }

    // realistically we're always going to be getting multiple surbs at once
    pub(crate) fn insert_reply_surbs<I: IntoIterator<Item = ReplySurb>>(&mut self, surbs: I) {
        let mut v = surbs.into_iter().collect::<VecDeque<_>>();
        println!("storing {} surbs in the storage", v.len());
        self.data.append(&mut v);
        self.surbs_last_received_at = Instant::now();
        println!("we now have {} surbs!", self.data.len());
    }

    pub(crate) fn insert_reply_surb(&mut self, surb: ReplySurb) {
        // TODO: be super careful about attempting to reset the instant here
        // as we frequently (or maybe only) use this method for re-inserting UNUSED, OLD
        // reply surb
        self.data.push_back(surb)
    }
}
