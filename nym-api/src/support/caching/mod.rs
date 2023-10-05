// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::Serialize;
use std::ops::Deref;
use time::OffsetDateTime;

pub(crate) mod cache;
pub(crate) mod refresher;

#[derive(Serialize, Clone)]
pub struct Cache<T> {
    pub value: T,
    as_at: i64,
}

impl<T> Cache<T> {
    pub fn new(value: T) -> Self {
        Cache {
            value,
            as_at: current_unix_timestamp(),
        }
    }

    pub(crate) fn update(&mut self, value: T) {
        self.value = value;
        self.as_at = current_unix_timestamp()
    }

    pub fn timestamp(&self) -> i64 {
        self.as_at
    }

    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T> Deref for Cache<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> Default for Cache<T>
where
    T: Default,
{
    fn default() -> Self {
        Cache {
            value: T::default(),
            as_at: 0,
        }
    }
}

fn current_unix_timestamp() -> i64 {
    let now = OffsetDateTime::now_utc();
    now.unix_timestamp()
}

// The cache can emit notifications to listeners about the current state
#[derive(Debug, PartialEq, Eq)]
pub enum CacheNotification {
    Start,
    Updated,
}
