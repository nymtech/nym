// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};

use nym_credentials_interface::CredentialSpendingData;
use nym_wireguard_types::PeerPublicKey;

const SEEN_CREDENTIAL_CACHE_TIME: Duration = Duration::from_secs(60 * 60); // 1 hour

#[derive(Eq, Hash, PartialEq)]
struct TimestampedPeerPubKey {
    peer_pub_key: PeerPublicKey,
    timestamp: SystemTime,
}

pub(crate) struct SeenCredentialCache {
    cached_credentials: HashMap<String, TimestampedPeerPubKey>,
}

impl SeenCredentialCache {
    pub(crate) fn new() -> Self {
        SeenCredentialCache {
            cached_credentials: HashMap::new(),
        }
    }

    pub(crate) fn insert_credential(
        &mut self,
        credential: CredentialSpendingData,
        peer_pub_key: PeerPublicKey,
    ) {
        let value = TimestampedPeerPubKey {
            peer_pub_key,
            timestamp: SystemTime::now(),
        };
        self.cached_credentials
            .insert(credential.serial_number_b58(), value);
    }

    pub(crate) fn contains(&self, credential: &CredentialSpendingData) -> bool {
        self.cached_credentials
            .contains_key(&credential.serial_number_b58())
    }

    pub(crate) fn remove_stale(&mut self) {
        let now = SystemTime::now();
        self.cached_credentials.retain(|_, value| {
            let Ok(cache_time) = now.duration_since(value.timestamp) else {
                return false;
            };
            cache_time < SEEN_CREDENTIAL_CACHE_TIME
        });
    }
}
