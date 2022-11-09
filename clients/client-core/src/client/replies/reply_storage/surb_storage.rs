// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use dashmap::DashMap;
use log::error;
use nymsphinx::anonymous_replies::requests::AnonymousSenderTag;
use nymsphinx::anonymous_replies::ReplySurb;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

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
}

impl ReceivedReplySurbsMap {
    pub(crate) fn new(min_surb_threshold: usize) -> ReceivedReplySurbsMap {
        ReceivedReplySurbsMap {
            inner: Arc::new(ReceivedReplySurbsMapInner {
                data: DashMap::new(),
                min_surb_threshold: AtomicUsize::new(min_surb_threshold),
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
            if surbs_left < self.inner.min_surb_threshold.load(Ordering::Relaxed) + amount {
                (None, surbs_left)
            } else {
                entry.get_reply_surbs(amount)
            }
        } else {
            (None, 0)
        }
    }

    pub(crate) fn get_reply_surb(
        &self,
        target: &AnonymousSenderTag,
    ) -> Option<(Option<ReplySurb>, usize)> {
        self.inner
            .data
            .get_mut(target)
            .map(|mut s| s.get_reply_surb())
    }

    pub(crate) async fn additional_surbs_request(&self, target: &AnonymousSenderTag) -> Option<()> {
        // let (reply_surb, _)
        None
    }

    pub(crate) fn insert_surbs(&self, target: &AnonymousSenderTag, surbs: Vec<ReplySurb>) {
        if let Some(mut existing_data) = self.inner.data.get_mut(target) {
            existing_data.insert_reply_surbs(surbs)
        } else {
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
    requesting_more_surbs: bool,
}

impl ReceivedReplySurbs {
    fn new(initial_surbs: Vec<ReplySurb>) -> Self {
        ReceivedReplySurbs {
            data: initial_surbs.into(),
            requesting_more_surbs: false,
        }
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
    pub(crate) fn insert_reply_surbs(&mut self, surbs: Vec<ReplySurb>) {
        self.data.append(&mut surbs.into())
    }
}
