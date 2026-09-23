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

/// Which rotation policy is reading/advancing a [`HostRotation`]'s `slot`. The two policies
/// disagree about what an untouched (`0`) slot means, so every access has to say which one
/// applies - see [`HostRotation::active_front_index`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RotationPolicy {
    /// The plain (non-`include_non_fronted_in_rotation`) policy: always shows a front once
    /// fronting is enabled, so slot `0` means "front `0`", never "no front".
    Plain,
    /// The `include_non_fronted_in_rotation` policy: cycles through a direct (unfronted) turn
    /// and each configured front in turn, so slot `0` means "the direct turn - no front active".
    Lap,
}

/// Rotation cursor for a single configured host, shared by both rotation policies.
///
/// Slot `0` is the host's untouched/reset state; slot `n + 1` means front `n` is currently
/// selected. [`RotationManager::update`] (the plain policy) and
/// [`RotationManager::take_rotation_turn`] (the `include_non_fronted_in_rotation` policy) both
/// advance this same slot, just with different cycle lengths - see their docs. Keeping a single
/// slot per host (rather than one cursor per policy) means there is exactly one place for every
/// reader to look: no accessor can forget to fall back to a second, possibly-stale cursor the
/// way this code once did.
#[derive(Debug, Default)]
struct HostRotation {
    slot: AtomicUsize,
}

impl HostRotation {
    /// The front index this host's slot currently points at, under `policy`'s interpretation of
    /// slot `0` - `None` only if `policy` is [`RotationPolicy::Lap`] and the host is resting on
    /// its direct turn.
    fn active_front_index(&self, policy: RotationPolicy) -> Option<usize> {
        match self.slot.load(Ordering::Relaxed) {
            0 if policy == RotationPolicy::Lap => None,
            0 => Some(0),
            slot => Some(slot - 1),
        }
    }
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

    /// Return the serialization of `url`'s currently active front, or `url`'s own serialization
    /// if no front is active. `lap` must match whichever rotation policy is actually driving
    /// host `idx` (`self.front.include_non_fronted_in_rotation()` on the caller's side) - passing
    /// the wrong one will misread slot `0` (see [`HostRotation::active_front_index`]).
    pub(crate) fn as_str<'a>(&self, idx: usize, url: &'a Url, lap: bool) -> &'a str {
        let policy = if lap {
            RotationPolicy::Lap
        } else {
            RotationPolicy::Plain
        };
        self.hosts[idx]
            .active_front_index(policy)
            .and_then(|index| url.fronts().and_then(|fronts| fronts.get(index)))
            .map(|front| front.as_str())
            .unwrap_or_else(|| url.inner_url().as_str())
    }

    /// Return the string representation of the front host (domain or IP address) currently
    /// selected for host `idx` under the plain rotation policy, if any.
    pub(crate) fn front_str<'a>(&self, idx: usize, url: &'a Url) -> Option<&'a str> {
        let index = self.hosts[idx].active_front_index(RotationPolicy::Plain)?;
        url.fronts()?.get(index)?.host_str()
    }

    /// Advance host `idx` to its next configured front. Returns `true` if updating the front
    /// wraps back to the first front, or if `url` has no (or only one) front configured.
    pub(crate) fn update(&self, idx: usize, url: &Url) -> bool {
        if let Some(fronts) = url.fronts()
            && fronts.len() > 1
        {
            let state = &self.hosts[idx];
            // Read via the plain policy so slot 0 (untouched, or left behind by the *other*
            // policy) is treated as front 0, matching this policy's "always on some front"
            // semantics - see `HostRotation::active_front_index`.
            let current = state
                .active_front_index(RotationPolicy::Plain)
                .expect("Plain policy always yields a front index");
            let next = (current + 1) % fronts.len();
            state.slot.store(next + 1, Ordering::Relaxed);
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
        let current = state.slot.load(Ordering::Relaxed);
        let next = (current + 1) % total_turns;
        state.slot.store(next, Ordering::Relaxed);

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
        let index = self.hosts[idx].active_front_index(RotationPolicy::Lap)?;
        url.fronts()?.get(index)?.host_str()
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

    /// `front_str` and `active_rotation_front_str` share the same underlying slot, so once
    /// `take_rotation_turn` has moved a host onto a front (slot != 0), both readings agree on
    /// which front that is - they only diverge on slot 0, where `front_str` (plain policy) reads
    /// "front 0" and `active_rotation_front_str` (lap policy) reads "no front, direct turn".
    #[test]
    fn front_str_tracks_take_rotation_turn_once_off_the_direct_slot() {
        let url = url(Some(vec!["https://f0.test", "https://f1.test"]));
        let mgr = RotationManager::new(1);

        // current_front is never advanced here - only take_rotation_turn is called.
        assert!(mgr.take_rotation_turn(0, &url));
        assert_eq!(mgr.active_rotation_front_str(0, &url), Some("f0.test"));
        assert_eq!(mgr.front_str(0, &url), Some("f0.test"));

        assert!(mgr.take_rotation_turn(0, &url));
        assert_eq!(mgr.active_rotation_front_str(0, &url), Some("f1.test"));
        assert_eq!(mgr.front_str(0, &url), Some("f1.test"));
    }

    /// The one case where the two policies' readings of the same slot intentionally diverge:
    /// slot 0 means "front 0" under the plain policy, but "no front, direct turn" under the lap
    /// policy.
    #[test]
    fn front_str_and_active_rotation_front_str_disagree_on_the_untouched_slot() {
        let url = url(Some(vec!["https://f0.test", "https://f1.test"]));
        let mgr = RotationManager::new(1);

        assert_eq!(mgr.front_str(0, &url), Some("f0.test"));
        assert_eq!(mgr.active_rotation_front_str(0, &url), None);
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

    /// The single shared slot survives a policy switch without panicking or indexing
    /// out-of-bounds, even though - as with the pre-refactor two-cursor design - there's no
    /// single "correct" front to land on when reinterpreting a lap-policy slot under the plain
    /// policy or vice versa.
    #[test]
    fn update_after_take_rotation_turn_does_not_panic_and_stays_in_bounds() {
        let url = url(Some(vec!["https://f0.test", "https://f1.test"]));
        let mgr = RotationManager::new(1);

        mgr.take_rotation_turn(0, &url);
        mgr.take_rotation_turn(0, &url);
        mgr.update(0, &url);

        assert!(mgr.front_str(0, &url).is_some());
    }
}
