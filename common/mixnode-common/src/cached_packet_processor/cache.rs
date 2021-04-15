// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use dashmap::mapref::one::Ref;
use dashmap::DashMap;
use futures::channel::mpsc;
use futures::StreamExt;
use log::*;
use nonexhaustive_delayqueue::{Expired, NonExhaustiveDelayQueue, TimerError};
use nymsphinx_types::header::keys::RoutingKeys;
use nymsphinx_types::SharedSecret;
use std::sync::Arc;
use tokio::time::Duration;

type CachedKeys = (Option<SharedSecret>, RoutingKeys);

pub(super) struct KeyCache {
    vpn_key_cache: Arc<DashMap<SharedSecret, CachedKeys>>,
    invalidator_sender: InvalidatorActionSender,
    cache_entry_ttl: Duration,
}

impl Drop for KeyCache {
    fn drop(&mut self) {
        debug!("dropping key cache");
        if self
            .invalidator_sender
            .unbounded_send(InvalidatorAction::Stop)
            .is_err()
        {
            debug!("invalidator has already been dropped")
        }
    }
}

impl KeyCache {
    pub(super) fn new(cache_entry_ttl: Duration) -> Self {
        let cache = Arc::new(DashMap::new());
        let (sender, receiver) = mpsc::unbounded();

        let mut invalidator = CacheInvalidator {
            entry_ttl: cache_entry_ttl,
            vpn_key_cache: Arc::clone(&cache),
            expirations: NonExhaustiveDelayQueue::new(),
            action_receiver: receiver,
        };

        // TODO: is it possible to avoid tokio::spawn here and make it semi-runtime agnostic?
        tokio::spawn(async move { invalidator.run().await });

        KeyCache {
            vpn_key_cache: cache,
            invalidator_sender: sender,
            cache_entry_ttl,
        }
    }

    pub(super) fn insert(&self, key: SharedSecret, cached_keys: CachedKeys) -> bool {
        trace!("inserting {:?} into the cache", key);
        let insertion_result = self.vpn_key_cache.insert(key, cached_keys).is_some();
        if !insertion_result {
            debug!("{:?} was put into the cache", key);
            // this shouldn't really happen, but don't insert entry to invalidator if it was already
            // in the cache
            self.invalidator_sender
                .unbounded_send(InvalidatorAction::Insert(key))
                .expect("Cache invalidator has crashed!");
        }
        insertion_result
    }

    // ElementGuard has Deref for CachedKeys so that's fine
    pub(super) fn get(&self, key: &SharedSecret) -> Option<Ref<SharedSecret, CachedKeys>> {
        self.vpn_key_cache.get(key)
    }

    pub(super) fn cache_entry_ttl(&self) -> Duration {
        self.cache_entry_ttl
    }

    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.vpn_key_cache.is_empty()
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.vpn_key_cache.len()
    }
}

enum InvalidatorAction {
    Insert(SharedSecret),
    Stop,
}

type InvalidatorActionSender = mpsc::UnboundedSender<InvalidatorAction>;
type InvalidatorActionReceiver = mpsc::UnboundedReceiver<InvalidatorAction>;

struct CacheInvalidator {
    entry_ttl: Duration,
    vpn_key_cache: Arc<DashMap<SharedSecret, CachedKeys>>,
    expirations: NonExhaustiveDelayQueue<SharedSecret>,
    action_receiver: InvalidatorActionReceiver,
}

// we do not have a strong requirement of invalidating things EXACTLY after their TTL expires.
// we want them to be eventually gone in a relatively timely manner.
impl CacheInvalidator {
    // two obvious ways I've seen of running this were as follows:
    //
    // 1) every X second, purge all expired entries
    // pros: simpler to implement
    // cons: will require to obtain write lock multiple times in quick succession
    //
    // 2) purge entry as soon as it expires
    // pros: the lock situation will be spread more in time
    // cons: possibly less efficient?

    fn handle_expired(&mut self, expired: Option<Result<Expired<SharedSecret>, TimerError>>) {
        let expired = expired.expect("the queue has unexpectedly terminated!");
        let expired_entry = expired.expect("Encountered timer issue within the runtime!");

        debug!(
            "{:?} has expired and will be removed",
            expired_entry.get_ref()
        );

        if self
            .vpn_key_cache
            .remove(&expired_entry.into_inner())
            .is_none()
        {
            error!("Tried to remove vpn cache entry for non-existent key!")
        }
    }

    /// Handles received action. Return `bool` indicates whether the invalidator
    /// should terminate.
    fn handle_action(&mut self, action: Option<InvalidatorAction>) -> bool {
        if action.is_none() {
            return true;
        }

        match action.unwrap() {
            InvalidatorAction::Stop => true,
            InvalidatorAction::Insert(shared_secret) => {
                self.expirations.insert(shared_secret, self.entry_ttl);
                false
            }
        }
    }

    async fn run(&mut self) {
        loop {
            tokio::select! {
                expired = self.expirations.next() => {
                    self.handle_expired(expired);
                }
                action = self.action_receiver.next() => {
                    if self.handle_action(action) {
                        info!("Stopping cache invalidator");
                        return
                    }
                }

            }
        }
    }
}
