// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};
use std::collections::vec_deque::{IntoIter, Iter};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RingBuffer<T> {
    #[serde(flatten)]
    inner: VecDeque<T>,
}

impl<T> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: VecDeque::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, item: T) {
        if self.inner.len() == self.inner.capacity() {
            self.inner.pop_front();
            self.inner.push_back(item);
            debug_assert!(self.inner.len() == self.inner.capacity());
        } else {
            self.inner.push_back(item);
        }
    }

    pub fn iter(&self) -> Iter<'_, T> {
        self.inner.iter()
    }
}

impl<T> From<RingBuffer<T>> for VecDeque<T> {
    fn from(value: RingBuffer<T>) -> Self {
        value.inner
    }
}

impl<T> From<RingBuffer<T>> for Vec<T> {
    fn from(value: RingBuffer<T>) -> Self {
        value.inner.into()
    }
}

impl<T> IntoIterator for RingBuffer<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}
