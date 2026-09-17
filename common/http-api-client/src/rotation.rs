// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Per-host domain-fronting rotation state.
//!
//! This is kept separate from [`crate::url::FrontedUrl`] so that type can stay a plain,
//! comparable value describing *what* a host is, while a [`RotationManager`] - shared across
//! [`crate::Client`] clones just like `Client`'s own `current_idx` - tracks *where* each host's
//! rotation currently stands.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::url::Url;

/// Rotation cursor for a single configured host.
#[derive(Debug, Default)]
struct HostRotation {
    /// Index into that host's `fronts` list currently selected by the plain (non-
    /// `include_non_fronted_in_rotation`) rotation policies.
    current_front: AtomicUsize,

    // Used only by the `include_non_fronted_in_rotation` rotation policy: cycles through
    // `0..=fronts.len()` - slot 0 means this host is shown directly (unfronted), slot `i + 1`
    // means it's shown via `fronts[i]`. Each lap visits every slot exactly once.
    rotation_slot: AtomicUsize,
}

/// Shared, per-host front-rotation state for a [`crate::Client`]'s configured base URLs.
///
/// Indices line up 1:1 with `Client::base_urls`. Internally `Arc`-backed so that cloning a
/// `Client` shares rotation progress with the original, the same way `Client::current_idx` does.
#[derive(Debug, Clone)]
pub(crate) struct RotationManager {
    hosts: Arc<[HostRotation]>,
}

impl RotationManager {
    pub(crate) fn new(num_hosts: usize) -> Self {
        Self {
            hosts: (0..num_hosts).map(|_| HostRotation::default()).collect(),
        }
    }

    /// Return the serialization of `url`'s currently active front if [`Self::take_rotation_turn`]
    /// has advanced host `idx` onto one of its fronts, or `url`'s own serialization otherwise.
    pub(crate) fn as_str<'a>(&self, idx: usize, url: &'a Url) -> &'a str {
        let slot = self.hosts[idx].rotation_slot.load(Ordering::Relaxed);
        if slot != 0
            && let Some(front) = url.fronts().and_then(|fronts| fronts.get(slot - 1))
        {
            return front.as_str();
        }
        url.inner_url().as_str()
    }

    /// Return the string representation of the front host (domain or IP address) currently
    /// selected for host `idx`, if any.
    pub(crate) fn front_str<'a>(&self, idx: usize, url: &'a Url) -> Option<&'a str> {
        let current = self.hosts[idx].current_front.load(Ordering::Relaxed);
        url.fronts()
            .and_then(|fronts| fronts.get(current))
            .and_then(|front| front.host_str())
    }

    /// Advance host `idx` to its next configured front. Returns `true` if updating the front
    /// wraps back to the first front, or if `url` has no (or only one) front configured.
    pub(crate) fn update(&self, idx: usize, url: &Url) -> bool {
        if let Some(fronts) = url.fronts()
            && fronts.len() > 1
        {
            let state = &self.hosts[idx];
            let current = state.current_front.load(Ordering::Relaxed);
            let next = (current + 1) % fronts.len();
            state.current_front.store(next, Ordering::Relaxed);
            return next == 0;
        }
        true
    }

    /// Used by [`crate::Client::update_host`] when a policy allows non-fronted domains into the
    /// rotation alongside fronted ones (`include_non_fronted_in_rotation`). Each host gets one
    /// turn shown directly before cycling through its configured fronts, one at a time.
    ///
    /// Advances host `idx` to the next turn in its lap - direct, then each configured front in
    /// turn, then back to direct - and returns `true` if that turn landed on a front (the caller
    /// should stay on this host rather than rotating away), or `false` if it landed back on the
    /// direct turn (the caller should move on, e.g. to the next base url).
    pub(crate) fn take_rotation_turn(&self, idx: usize, url: &Url) -> bool {
        let Some(fronts) = url.fronts() else {
            return false;
        };
        if fronts.is_empty() {
            return false;
        }
        let state = &self.hosts[idx];

        // slots 0..=fronts.len() form the lap; advancing past the last front wraps back to the
        // direct slot (0) rather than revisiting it a second time.
        let total_turns = fronts.len() + 1;
        let current = state.rotation_slot.load(Ordering::Relaxed);
        let next = (current + 1) % total_turns;
        state.rotation_slot.store(next, Ordering::Relaxed);

        next != 0
    }

    /// The front currently in use for host `idx`'s rotation turn, if [`Self::take_rotation_turn`]
    /// has advanced it onto one of its fronts. Returns `None` while that host's direct turn is
    /// active, meaning requests should go out unfronted.
    pub(crate) fn active_rotation_front_str<'a>(
        &self,
        idx: usize,
        url: &'a Url,
    ) -> Option<&'a str> {
        let slot = self.hosts[idx].rotation_slot.load(Ordering::Relaxed);
        if slot == 0 {
            return None;
        }
        url.fronts()?.get(slot - 1)?.host_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(fronts: Option<Vec<&str>>) -> Url {
        Url::new("https://a.test", fronts).unwrap()
    }

    #[test]
    fn front_str_starts_at_the_first_configured_front() {
        let url = url(Some(vec!["https://f0.test", "https://f1.test"]));
        let mgr = RotationManager::new(1);

        assert_eq!(mgr.front_str(0, &url), Some("f0.test"));
    }

    #[test]
    fn front_str_is_none_without_fronts() {
        let url = url(None);
        let mgr = RotationManager::new(1);

        assert_eq!(mgr.front_str(0, &url), None);
    }

    #[test]
    fn update_without_multiple_fronts_always_reports_wrapped() {
        let mgr = RotationManager::new(1);

        // no fronts configured at all
        let plain = url(None);
        assert!(mgr.update(0, &plain));
        assert!(mgr.update(0, &plain));

        // exactly one front - nothing to cycle through, so it "wraps" on every call and the
        // single front stays selected throughout
        let single = url(Some(vec!["https://f0.test"]));
        assert!(mgr.update(0, &single));
        assert_eq!(mgr.front_str(0, &single), Some("f0.test"));
        assert!(mgr.update(0, &single));
        assert_eq!(mgr.front_str(0, &single), Some("f0.test"));
    }

    #[test]
    fn update_cycles_through_fronts_and_reports_wraparound_only_on_the_last_one() {
        let url = url(Some(vec![
            "https://f0.test",
            "https://f1.test",
            "https://f2.test",
        ]));
        let mgr = RotationManager::new(1);

        // starts on f0; advancing steps through f1, f2, and only wraps back to f0 on the third
        // call.
        assert!(!mgr.update(0, &url));
        assert_eq!(mgr.front_str(0, &url), Some("f1.test"));

        assert!(!mgr.update(0, &url));
        assert_eq!(mgr.front_str(0, &url), Some("f2.test"));

        assert!(mgr.update(0, &url));
        assert_eq!(mgr.front_str(0, &url), Some("f0.test"));
    }

    #[test]
    fn update_state_is_independent_per_host() {
        let url = url(Some(vec!["https://f0.test", "https://f1.test"]));
        let mgr = RotationManager::new(2);

        mgr.update(0, &url);
        assert_eq!(mgr.front_str(0, &url), Some("f1.test"));
        // host 1 was never updated, so it's untouched by host 0's rotation.
        assert_eq!(mgr.front_str(1, &url), Some("f0.test"));
    }

    #[test]
    fn take_rotation_turn_without_fronts_never_advances() {
        let url = url(None);
        let mgr = RotationManager::new(1);

        for _ in 0..3 {
            assert!(!mgr.take_rotation_turn(0, &url));
            assert_eq!(mgr.active_rotation_front_str(0, &url), None);
        }
    }

    #[test]
    fn take_rotation_turn_with_empty_fronts_never_advances() {
        let url = url(Some(Vec::new()));
        let mgr = RotationManager::new(1);

        for _ in 0..3 {
            assert!(!mgr.take_rotation_turn(0, &url));
            assert_eq!(mgr.active_rotation_front_str(0, &url), None);
        }
    }

    /// The per-lap sequence for a host with fronts: one turn per configured front, then back to
    /// direct - exactly `fronts.len() + 1` turns per lap, with no turn repeated.
    #[test]
    fn take_rotation_turn_lap_sequence() {
        let url = url(Some(vec!["https://f0.test", "https://f1.test"]));
        let mgr = RotationManager::new(1);

        let lap = |mgr: &RotationManager| {
            [(); 3].map(|()| {
                let advanced = mgr.take_rotation_turn(0, &url);
                (advanced, mgr.active_rotation_front_str(0, &url))
            })
        };

        let expected = [
            (true, Some("f0.test")),
            (true, Some("f1.test")),
            (false, None),
        ];
        assert_eq!(lap(&mgr), expected, "first lap");
        // a second lap repeats identically rather than lingering on the direct turn.
        assert_eq!(lap(&mgr), expected, "second lap");
    }

    #[test]
    fn take_rotation_turn_single_front_alternates_with_direct() {
        let url = url(Some(vec!["https://f0.test"]));
        let mgr = RotationManager::new(1);

        assert!(mgr.take_rotation_turn(0, &url));
        assert_eq!(mgr.active_rotation_front_str(0, &url), Some("f0.test"));

        assert!(!mgr.take_rotation_turn(0, &url));
        assert_eq!(mgr.active_rotation_front_str(0, &url), None);

        assert!(mgr.take_rotation_turn(0, &url));
        assert_eq!(mgr.active_rotation_front_str(0, &url), Some("f0.test"));
    }

    #[test]
    fn take_rotation_turn_state_is_independent_per_host() {
        let url = url(Some(vec!["https://f0.test"]));
        let mgr = RotationManager::new(2);

        // advance host 0 onto its front.
        assert!(mgr.take_rotation_turn(0, &url));
        assert_eq!(mgr.active_rotation_front_str(0, &url), Some("f0.test"));

        // host 1 has never been touched, so it's still on its own direct turn.
        assert_eq!(mgr.active_rotation_front_str(1, &url), None);
    }
}
