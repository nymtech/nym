// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub(crate) mod cache;
pub(crate) mod refresher;

// don't break existing imports
pub(crate) use cache::Cache;

// The cache can emit notifications to listeners about the current state
#[derive(Debug, PartialEq, Eq)]
pub enum CacheNotification {
    Start,
    Updated,
}
