// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::key_rotation::key::SphinxPrivateKey;
use arc_swap::{ArcSwap, ArcSwapOption, Guard};
use std::ops::Deref;
use std::sync::Arc;
use tracing::error;

#[derive(Clone)]
pub(crate) struct ActiveSphinxKeys {
    inner: Arc<ActiveSphinxKeysInner>,
}

struct ActiveSphinxKeysInner {
    /// Key that's currently used as the default when processing packets with no explicit rotation information
    primary_key: ArcSwap<SphinxPrivateKey>,

    /// Optionally, a secondary key associated with this node. depending on the context it could either be
    /// the pre-announced key for the following rotation or a key from the previous rotation for the overlap period
    secondary_key: ArcSwapOption<SphinxPrivateKey>,
}

impl ActiveSphinxKeys {
    pub(crate) fn new_fresh(primary: SphinxPrivateKey) -> Self {
        ActiveSphinxKeys {
            inner: Arc::new(ActiveSphinxKeysInner {
                primary_key: ArcSwap::from_pointee(primary),
                secondary_key: Default::default(),
            }),
        }
    }

    pub(crate) fn new_loaded(
        primary: SphinxPrivateKey,
        secondary: Option<SphinxPrivateKey>,
    ) -> Self {
        ActiveSphinxKeys {
            inner: Arc::new(ActiveSphinxKeysInner {
                primary_key: ArcSwap::from_pointee(primary),
                secondary_key: ArcSwapOption::from_pointee(secondary),
            }),
        }
    }

    pub(crate) fn even(&self) -> Option<SphinxKeyGuard> {
        let primary = self.inner.primary_key.load();
        if primary.is_even_rotation() {
            return Some(SphinxKeyGuard::Primary(primary));
        }
        self.secondary()
    }

    pub(crate) fn odd(&self) -> Option<SphinxKeyGuard> {
        let primary = self.inner.primary_key.load();
        if !primary.is_even_rotation() {
            return Some(SphinxKeyGuard::Primary(primary));
        }
        self.secondary()
    }

    pub(crate) fn primary(&self) -> SphinxKeyGuard {
        SphinxKeyGuard::Primary(self.inner.primary_key.load())
    }

    pub(crate) fn secondary(&self) -> Option<SphinxKeyGuard> {
        let guard = self.inner.secondary_key.load();
        if guard.is_none() {
            return None;
        }

        Some(SphinxKeyGuard::Secondary(SecondaryKeyGuard { guard }))
    }

    pub(crate) fn set_secondary(&self, new_key: SphinxPrivateKey) {
        self.inner.secondary_key.store(Some(Arc::new(new_key)))
    }

    pub(crate) fn primary_key_rotation_id(&self) -> u32 {
        self.inner.primary_key.load().rotation_id()
    }

    pub(crate) fn secondary_key_rotation_id(&self) -> Option<u32> {
        self.inner
            .secondary_key
            .load()
            .as_ref()
            .map(|k| k.rotation_id())
    }

    // set the secondary (pre-announced key) as the primary
    // and the current primary as the secondary (for the overlap epoch)
    pub(crate) fn rotate(&self) -> bool {
        let Some(pre_announced) = self.inner.secondary_key.load_full() else {
            error!("sphinx key inconsistency - attempted to perform key rotation without having pre-announced new key");
            return false;
        };

        if pre_announced.rotation_id() != self.primary_key_rotation_id() + 1 {
            error!("sphinx key inconsistency - pre-announced key rotation id != primary + 1");
            return false;
        }

        let old_primary = self.inner.primary_key.swap(pre_announced);
        self.inner.secondary_key.store(Some(old_primary));
        true
    }

    pub(crate) fn deactivate_secondary(&self) {
        self.inner.secondary_key.store(None);
    }
}

pub(crate) enum SphinxKeyGuard {
    // Primary(Guard<Arc<SphinxPrivateKey>>),
    Primary(Guard<Arc<SphinxPrivateKey>>),
    Secondary(SecondaryKeyGuard),
}

impl Deref for SphinxKeyGuard {
    type Target = SphinxPrivateKey;

    fn deref(&self) -> &Self::Target {
        match self {
            SphinxKeyGuard::Primary(g) => g.deref(),
            SphinxKeyGuard::Secondary(g) => g.deref(),
        }
    }
}

// enum SecondaryKey {
//     PreAnnounced(SphinxPrivateKey),
//     PreviousOverlap(SphinxPrivateKey),
// }

// impl Deref for SecondaryKey {
//     type Target = SphinxPrivateKey;
//
//     fn deref(&self) -> &Self::Target {
//         match self {
//             SecondaryKey::PreAnnounced(key) => &key,
//             SecondaryKey::PreviousOverlap(key) => &key,
//         }
//     }
// }

pub(crate) struct SecondaryKeyGuard {
    guard: Guard<Option<Arc<SphinxPrivateKey>>>,
    // guard: Guard<Option<Arc<SecondaryKey>>>,
}

impl Deref for SecondaryKeyGuard {
    type Target = SphinxPrivateKey;

    fn deref(&self) -> &Self::Target {
        // SAFETY: the guard is ONLY constructed when the key is 'Some'
        #[allow(clippy::unwrap_used)]
        self.guard.as_ref().unwrap()
    }
}
