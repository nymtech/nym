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
    primary_key: ArcSwap<SphinxPrivateKey>,
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

    pub(crate) fn rotate(&self, new_primary: SphinxPrivateKey) {
        if self.inner.secondary_key.load().is_some() {
            // this should NEVER happen, but technically nothing should blow up
            error!("somehow our secondary key was still set during the rotation!")
        }

        let old_primary = self.inner.primary_key.swap(Arc::new(new_primary));
        self.inner.secondary_key.store(Some(old_primary));
    }

    fn deactivate_secondary(&self) {
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

pub(crate) struct SecondaryKeyGuard {
    guard: Guard<Option<Arc<SphinxPrivateKey>>>,
}

impl Deref for SecondaryKeyGuard {
    type Target = SphinxPrivateKey;

    fn deref(&self) -> &Self::Target {
        // SAFETY: the guard is ONLY constructed when the key is 'Some'
        #[allow(clippy::unwrap_used)]
        self.guard.as_ref().unwrap()
    }
}
